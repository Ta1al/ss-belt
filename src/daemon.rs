use anyhow::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::action;
use crate::capabilities::{Desktop, ToolSet};
use crate::config::Config;
use crate::error::{is_user_error, user};
use crate::paths::Paths;
use crate::steps::Ctx;
use crate::error::HintExt;

#[derive(Debug, Clone)]
pub enum Event {
    Action(String),
    Reload,
    Quit,
}

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_signal: i32) {
    STOP.store(true, Ordering::SeqCst);
}

/// Is a daemon listening? Connecting is the only reliable check: a pid file can
/// outlive a crash.
pub fn is_running(paths: &Paths) -> bool {
    let socket = paths.socket_file();
    if !socket.exists() {
        return false;
    }
    match UnixStream::connect(&socket) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let _ = stream.write_all(b"ping\n");
            let mut line = String::new();
            let _ = BufReader::new(stream).read_line(&mut line);
            line.trim() == "pong"
        }
        Err(_) => false,
    }
}

/// Send one command to a running daemon and return its single-line reply.
pub fn send(paths: &Paths, command: &str) -> Result<String> {
    let socket = paths.socket_file();
    let mut stream = UnixStream::connect(&socket).map_err(|_| {
        user("no daemon is running").hint("start it with `ss-belt daemon`")
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    writeln!(stream, "{command}")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn run_action(paths: &Paths, config: &Config, action_name: &str) {
    let tools = ToolSet::detect();
    let desktop = Desktop::detect();
    match Ctx::new(paths.clone(), config.clone(), tools, desktop, action_name) {
        Ok(mut ctx) => match action::execute(&mut ctx, action_name, None) {
            Ok(execution) => {
                if !execution.message.is_empty() {
                    tracing::info!("{}: {}", action_name, execution.message);
                }
            }
            Err(err) if is_user_error(&err) => {
                tracing::warn!("{action_name}: {err:#}");
                ctx.notify("ss-belt failed", &format!("{err:#}"));
            }
            Err(err) => {
                tracing::error!("{action_name}: {err:#}");
                ctx.notify("ss-belt failed", &format!("{err:#}"));
            }
        },
        Err(err) => tracing::error!("could not start {action_name}: {err:#}"),
    }
}

/// The daemon loop: owns the socket, the tray, the X11 grabs and a worker
/// thread that runs actions one at a time.
pub fn run(paths: Paths, mut config: Config) -> Result<()> {
    paths.ensure()?;

    let lock = acquire_lock(&paths)?;
    let _ = fs::remove_file(paths.socket_file());
    let listener = UnixListener::bind(paths.socket_file())
        .with_context(|| format!("binding {}", paths.socket_file().display()))?;
    listener.set_nonblocking(true)?;

    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_signal as *const () as libc::sighandler_t);
    }

    let (event_tx, event_rx) = mpsc::channel::<Event>();
    let (job_tx, job_rx) = mpsc::channel::<Event>();

    // Worker: runs actions sequentially so a long-running annotator never blocks
    // the socket or the hotkey grabber.
    {
        let paths = paths.clone();
        let job_config = config.clone();
        std::thread::spawn(move || worker(paths, job_config, job_rx));
    }

    // Tray.
    let _tray = crate::tray::spawn(event_tx.clone(), false);

    // X11 hotkeys, when there is an X server to grab them on.
    if std::env::var_os("DISPLAY").is_some() {
        let hotkey_config = config.clone();
        let hotkey_tx = event_tx.clone();
        std::thread::spawn(move || {
            if let Err(err) = crate::hotkeys::x11::grab(&hotkey_config, |action_name| {
                let _ = hotkey_tx.send(Event::Action(action_name.to_string()));
            }) {
                tracing::debug!("x11 hotkeys unavailable: {err:#}");
            }
        });
    }

    tracing::info!("daemon listening on {}", paths.socket_file().display());
    eprintln!("ss-belt daemon {} listening on {}", crate::VERSION, paths.socket_file().display());
    eprintln!("  hotkeys: {}", hotkey_summary(&config));
    eprintln!("  (Wayland users: see `ss-belt hotkeys --print <desktop>`)");

    while !STOP.load(Ordering::SeqCst) {
        // Socket commands.
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle_client(stream, &paths, &mut config, &job_tx);
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => tracing::warn!("accept failed: {err}"),
        }

        // Hotkey and tray events.
        match event_rx.recv_timeout(Duration::from_millis(80)) {
            Ok(Event::Action(name)) => {
                let _ = job_tx.send(Event::Action(name));
            }
            Ok(Event::Reload) => {
                config = reload(&paths, &config);
                let _ = job_tx.send(Event::Reload);
            }
            Ok(Event::Quit) => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = fs::remove_file(paths.socket_file());
    drop(lock);
    tracing::info!("daemon stopped");
    Ok(())
}

fn handle_client(
    mut stream: UnixStream,
    paths: &Paths,
    config: &mut Config,
    job_tx: &Sender<Event>,
) -> Result<()> {
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    let command = line.trim();
    let reply = match command {
        "ping" => "pong".to_string(),
        "status" => crate::doctor::summary_line_with(paths, config, true)?,
        "reload" => {
            *config = reload(paths, config);
            let _ = job_tx.send(Event::Reload);
            "reloaded".to_string()
        }
        "quit" => {
            STOP.store(true, Ordering::SeqCst);
            let _ = job_tx.send(Event::Quit);
            "bye".to_string()
        }
        other if other.starts_with("action ") => {
            let name = other.trim_start_matches("action ").trim().to_string();
            let _ = job_tx.send(Event::Action(name));
            "queued".to_string()
        }
        other => format!("unknown command: {other}"),
    };
    writeln!(stream, "{reply}")?;
    Ok(())
}

fn hotkey_summary(config: &Config) -> String {
    let mut parts = Vec::new();
    for action in crate::config::HOTKEY_ACTIONS {
        if let Some(accelerator) = config.hotkeys.get(action) {
            parts.push(format!("{action}={accelerator}"));
        }
    }
    parts.join(" ")
}

fn reload(paths: &Paths, fallback: &Config) -> Config {
    match Config::load(paths) {
        Ok(config) => {
            tracing::info!("config reloaded");
            config
        }
        Err(err) => {
            tracing::error!("config reload failed, keeping the previous one: {err:#}");
            fallback.clone()
        }
    }
}

struct Lock {
    _file: fs::File,
    path: std::path::PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_lock(paths: &Paths) -> Result<Lock> {
    if is_running(paths) {
        return Err(user("a daemon is already running").hint("stop it with `ss-belt daemon stop`"));
    }
    let path = paths.lock_file();
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;

    let fd = std::os::unix::io::AsRawFd::as_raw_fd(&file);
    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(user("a daemon is already running")
            .hint("stop it with `ss-belt daemon stop`"));
    }
    let _ = fs::write(&path, std::process::id().to_string());
    Ok(Lock { _file: file, path })
}

fn worker(paths: Paths, mut config: Config, jobs: Receiver<Event>) {
    while let Ok(job) = jobs.recv() {
        match job {
            Event::Action(name) => run_action(&paths, &config, &name),
            Event::Reload => {
                config = reload(&paths, &config);
            }
            Event::Quit => break,
        }
    }
}

/// Ensure a daemon is reachable, starting one detached if needed.
pub fn ensure_running(paths: &Paths, config: &Config) -> Result<()> {
    if let Ok(reply) = send(paths, "ping") {
        if reply == "pong" {
            return Ok(());
        }
    }
    let exe = std::env::current_exe().context("locating the ss-belt executable")?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(root) = &paths.root {
        command.env("SS_BELT_HOME", root);
    }
    command.spawn().context("starting the daemon")?;

    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if is_running(paths) {
            return Ok(());
        }
    }
    let _ = config;
    Err(user("the daemon did not start").hint("start it manually with `ss-belt daemon`"))
}

pub fn socket_path(paths: &Paths) -> std::path::PathBuf {
    paths.socket_file()
}

pub fn shared_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(STOP.load(Ordering::SeqCst)))
}
