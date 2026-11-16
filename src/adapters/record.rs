use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::capabilities::{SessionType, ToolId};
use crate::error::{missing_tools, user};
use crate::paths::Paths;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::error::HintExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub pid: i32,
    pub path: String,
    pub kind: String,
    pub backend: String,
    pub started: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Backend {
    WlScreenrec,
    WfRecorder,
    Ffmpeg,
}

impl Backend {
    fn label(self) -> &'static str {
        match self {
            Backend::WlScreenrec => "wl-screenrec",
            Backend::WfRecorder => "wf-recorder",
            Backend::Ffmpeg => "ffmpeg",
        }
    }
}

fn select(probe: &Probe<'_>) -> Result<Backend> {
    match probe.config.record.backend.as_str() {
        "wl-screenrec" => return Ok(Backend::WlScreenrec),
        "wf-recorder" => return Ok(Backend::WfRecorder),
        "ffmpeg" => return Ok(Backend::Ffmpeg),
        _ => {}
    }

    if probe.desktop.is_wayland() && probe.tools.has(ToolId::WlScreenrec) {
        return Ok(Backend::WlScreenrec);
    }
    if probe.desktop.is_wayland() && probe.tools.has(ToolId::WfRecorder) {
        return Ok(Backend::WfRecorder);
    }
    if probe.tools.has(ToolId::Ffmpeg) {
        if probe.desktop.is_wayland() {
            return Err(missing_tools(
                "screen recording on Wayland",
                &[ToolId::WlScreenrec, ToolId::WfRecorder],
            ));
        }
        return Ok(Backend::Ffmpeg);
    }
    Err(missing_tools(
        "screen recording",
        &[ToolId::WlScreenrec, ToolId::WfRecorder, ToolId::Ffmpeg],
    ))
}

pub fn start_support(probe: &Probe<'_>) -> Support {
    match select(probe) {
        Ok(backend) => {
            if backend == Backend::Ffmpeg && probe.desktop.session == SessionType::None {
                Support::Unavailable("no display for ffmpeg to record".into())
            } else if backend == Backend::Ffmpeg && probe.desktop.is_wayland() {
                Support::Degraded(
                    "ffmpeg records the XWayland display only; native Wayland windows are missing"
                        .into(),
                )
            } else {
                Support::Ready
            }
        }
        Err(err) => Support::Unavailable(err.to_string()),
    }
}

pub fn stop_support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

pub fn read_state(paths: &Paths) -> Option<Recording> {
    let text = fs::read_to_string(paths.state_file()).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn is_recording(paths: &Paths) -> bool {
    read_state(paths).map(|state| process_alive(state.pid)).unwrap_or(false)
}

/// Children this process spawned, so they can be reaped instead of becoming
/// zombies that keep looking "alive" to `kill(pid, 0)`.
fn children() -> &'static Mutex<Vec<(i32, Child)>> {
    static CHILDREN: OnceLock<Mutex<Vec<(i32, Child)>>> = OnceLock::new();
    CHILDREN.get_or_init(|| Mutex::new(Vec::new()))
}

fn register(child: Child) -> i32 {
    let pid = child.id() as i32;
    children().lock().expect("recorder registry").push((pid, child));
    pid
}

/// Wait for a child we own. Returns false when the pid belongs to another
/// process (the daemon started it, for example).
fn reap(pid: i32, timeout: Duration) -> bool {
    let mut registry = children().lock().expect("recorder registry");
    let Some(index) = registry.iter().position(|(candidate, _)| *candidate == pid) else {
        return false;
    };
    let (_, mut child) = registry.remove(index);
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return true;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return true,
        }
    }
}

fn process_alive(pid: i32) -> bool {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return false;
    }
    // A zombie answers kill(0) but is not recording anything.
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => !stat
            .rsplit_once(')')
            .map(|(_, rest)| rest.trim_start().starts_with('Z'))
            .unwrap_or(false),
        Err(_) => true,
    }
}

fn signal(pid: i32, signal: i32) {
    unsafe {
        libc::kill(pid, signal);
    }
}

pub fn start_run(ctx: &mut Ctx) -> Result<()> {
    if is_recording(&ctx.paths) {
        return Err(user("a recording is already running")
            .hint("press the record hotkey again to stop it"));
    }

    let probe = Probe { tools: &ctx.tools, desktop: &ctx.desktop, config: &ctx.config };
    let backend = select(&probe)?;
    let support = start_support(&probe);
    if !support.ready() {
        return Err(user(support.detail().unwrap_or("recording is not available").to_string()));
    }
    if let Support::Degraded(detail) = &support {
        ctx.note(detail.clone());
    }

    let path = ctx.stage("recording.mp4");

    if ctx.dry_run {
        ctx.note(format!("dry-run: would start {} recording", backend.label()));
        ctx.value = Value::Video(path);
        return Ok(());
    }

    let mut command = build_command(ctx, backend, &path)?;
    let log = fs::File::create(ctx.paths.data_dir.join("recorder.log")).ok();
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log.map(Stdio::from).unwrap_or_else(Stdio::null));

    let child = command
        .spawn()
        .with_context(|| format!("starting {}", backend.label()))?;
    // The pid is recorded so the daemon (or the next CLI call) can stop it even
    // from another process. Within this process the handle is kept so the child
    // is reaped rather than left as a zombie.
    let pid = register(child);

    let state = Recording {
        pid,
        path: path.display().to_string(),
        kind: ctx.action.clone(),
        backend: backend.label().to_string(),
        started: chrono::Local::now().to_rfc3339(),
    };
    fs::write(ctx.paths.state_file(), serde_json::to_vec_pretty(&state)?)?;

    // Give the recorder a moment to fail fast (bad flags, no permission).
    std::thread::sleep(Duration::from_millis(400));
    if !process_alive(pid) {
        let _ = fs::remove_file(ctx.paths.state_file());
        let log = fs::read_to_string(ctx.paths.data_dir.join("recorder.log")).unwrap_or_default();
        return Err(user(format!(
            "{} exited immediately: {}",
            backend.label(),
            crate::util::truncate_text(&log, 300)
        )));
    }

    ctx.note(format!("recording with {} (pid {pid})", backend.label()));
    ctx.value = Value::Video(path);
    ctx.notify("Recording", "Press the hotkey again to stop");
    Ok(())
}

fn build_command(ctx: &Ctx, backend: Backend, path: &Path) -> Result<Command> {
    let fps = ctx.config.record.fps.max(1).to_string();
    let mut command = Command::new(ctx.tools.program(match backend {
        Backend::WlScreenrec => ToolId::WlScreenrec,
        Backend::WfRecorder => ToolId::WfRecorder,
        Backend::Ffmpeg => ToolId::Ffmpeg,
    }));

    match backend {
        Backend::WlScreenrec => {
            command
                .arg("--filename")
                .arg(path)
                .arg("--framerate")
                .arg(&fps);
            if ctx.config.record.audio {
                command.arg("--audio");
            }
        }
        Backend::WfRecorder => {
            command.arg("-f").arg(path).arg("-r").arg(&fps);
            if ctx.config.record.audio {
                command.arg("--audio");
            }
        }
        Backend::Ffmpeg => {
            let display = std::env::var("DISPLAY")
                .map_err(|_| user("ffmpeg recording needs DISPLAY (X11)"))?;
            command
                .args(["-y", "-loglevel", "error", "-f", "x11grab"])
                .args(["-framerate", &fps, "-i", &display])
                .args(["-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p"])
                .arg(path);
        }
    }
    Ok(command)
}

pub fn stop_run(ctx: &mut Ctx) -> Result<()> {
    let state = read_state(&ctx.paths)
        .ok_or_else(|| user("no recording is running").hint("start one first"))?;

    if ctx.dry_run {
        ctx.note(format!("dry-run: would stop recording {}", state.path));
        ctx.value = Value::Video(PathBuf::from(&state.path));
        return Ok(());
    }

    signal(state.pid, libc::SIGINT);

    if !reap(state.pid, Duration::from_secs(15)) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if !process_alive(state.pid) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if process_alive(state.pid) {
            signal(state.pid, libc::SIGKILL);
            std::thread::sleep(Duration::from_millis(300));
        }
    }
    let _ = fs::remove_file(ctx.paths.state_file());

    let path = PathBuf::from(&state.path);
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        return Err(user("the recording is empty; the recorder may have been cancelled")
            .hint("check the compositor's screen-copy permissions"));
    }

    ctx.note(format!("recorded {} ({})", path.display(), crate::util::human_bytes(size)));
    ctx.value = Value::Video(path);
    Ok(())
}

/// Convert a video into a GIF with ffmpeg's palette pipeline.
pub fn gif_support(probe: &Probe<'_>) -> Support {
    if probe.tools.has(ToolId::Ffmpeg) || probe.tools.has(ToolId::Gifski) {
        Support::Ready
    } else {
        Support::Unavailable("GIF conversion needs ffmpeg or gifski".into())
    }
}

pub fn gif_run(ctx: &mut Ctx) -> Result<()> {
    let input = ctx.require_video("gif.convert")?;

    if input.extension().and_then(|e| e.to_str()) == Some("gif") {
        ctx.note("already a GIF");
        return Ok(());
    }

    let output = ctx.stage("recording.gif");

    if ctx.dry_run {
        ctx.note(format!("dry-run: would convert {} to GIF", input.display()));
        ctx.value = Value::Video(output);
        return Ok(());
    }

    if ctx.tools.has(ToolId::Gifski) {
        let fps = ctx.config.record.gif_fps.max(1).to_string();
        let width = ctx.config.record.gif_width.max(16).to_string();
        let result = crate::exec::Cmd::new(ctx.tools.program(ToolId::Gifski))
            .args(["--fps", &fps, "--width", &width, "-o"])
            .arg_path(&output)
            .arg_path(&input)
            .timeout(Duration::from_secs(600))
            .run()?;
        if !result.success() {
            return Err(user(format!("gifski failed: {}", result.failure())));
        }
    } else {
        let fps = ctx.config.record.gif_fps.max(1).to_string();
        let width = ctx.config.record.gif_width.max(16).to_string();
        let filter = format!(
            "fps={fps},scale={width}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
        );
        let result = crate::exec::Cmd::new(ctx.tools.program(ToolId::Ffmpeg))
            .args(["-y", "-loglevel", "error", "-i"])
            .arg_path(&input)
            .args(["-vf", &filter, "-loop", "0"])
            .arg_path(&output)
            .timeout(Duration::from_secs(600))
            .run()?;
        if !result.success() {
            return Err(user(format!("ffmpeg GIF conversion failed: {}", result.failure())));
        }
    }

    if !output.exists() {
        return Err(user("GIF conversion produced no file"));
    }
    ctx.note(format!(
        "converted to GIF ({})",
        crate::util::human_bytes(fs::metadata(&output).map(|m| m.len()).unwrap_or(0))
    ));
    ctx.value = Value::Video(output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn state_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let paths = testing::paths(dir.path());
        let state = Recording {
            pid: 1234,
            path: "/tmp/x.mp4".into(),
            kind: "gif".into(),
            backend: "wl-screenrec".into(),
            started: "now".into(),
        };
        fs::write(paths.state_file(), serde_json::to_vec(&state).unwrap()).unwrap();
        let read = read_state(&paths).unwrap();
        assert_eq!(read.pid, 1234);
        assert_eq!(read.kind, "gif");
    }

    #[test]
    fn no_state_means_not_recording() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_recording(&testing::paths(dir.path())));
    }

    #[test]
    fn ffmpeg_is_the_x11_fallback() {
        let tools = testing::tools(&[ToolId::Ffmpeg]);
        let mut desktop = testing::x11();
        desktop.session = SessionType::X11;
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(matches!(select(&probe), Ok(Backend::Ffmpeg)));
    }

    #[test]
    fn wayland_without_a_recorder_is_explicit() {
        let tools = testing::tools(&[ToolId::Ffmpeg]);
        let desktop = testing::wayland();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        let support = start_support(&probe);
        assert!(!support.ready());
        assert!(support.detail().unwrap().contains("wl-screenrec"));
    }
}
