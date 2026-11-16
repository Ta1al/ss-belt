use anyhow::{Context, Result};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use ss_belt::action::{self, Execution};
use ss_belt::capabilities::{Desktop, ToolSet};
use ss_belt::config::Config;
use ss_belt::error::{is_cancelled, is_user_error, user, HintExt};
use ss_belt::history::History;
use ss_belt::paths::Paths;
use ss_belt::steps::{Ctx, Value};
use ss_belt::{APP_NAME, VERSION};

#[derive(Parser, Debug)]
#[command(
    name = "ss-belt",
    version,
    about = "One toolbelt for Linux capture, annotate, record, OCR, QR and upload",
    long_about = None,
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
struct Cli {
    /// Show what would happen without capturing, uploading or writing files.
    #[arg(long, global = true)]
    dry_run: bool,

    /// Use an alternate config file.
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Increase log verbosity (-v for steps, -vv for debug).
    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Capture a region, save it and copy it.
    Region,
    /// Capture every monitor.
    Full,
    /// Capture the active window.
    Window,
    /// Capture the current monitor.
    Monitor,
    /// Capture a region and annotate it before saving.
    Annotate,
    /// Edit the image currently on the clipboard.
    #[command(name = "clipboard-edit")]
    ClipboardEdit,
    /// Decode a QR code from a region and copy the result.
    #[command(name = "qr-scan")]
    QrScan {
        /// Decode this image instead of capturing the screen.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Turn text into a QR image.
    #[command(name = "qr-make")]
    QrMake {
        /// Text to encode; reads the clipboard when omitted.
        text: Option<String>,
    },
    /// Extract text from a region with tesseract.
    Ocr {
        /// Read this image instead of capturing the screen.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Upload a file, or the last capture when no file is given.
    Upload {
        file: Option<PathBuf>,
        /// Destination name from the config.
        #[arg(long)]
        destination: Option<String>,
    },
    /// Annotate the last capture again.
    #[command(name = "annotate-last")]
    AnnotateLast,
    /// Record a GIF (run again to stop).
    Gif {
        /// Record for this many seconds, then convert.
        #[arg(long)]
        duration: Option<u64>,
    },
    /// Record a video (run again to stop).
    Video {
        #[arg(long)]
        duration: Option<u64>,
    },
    /// Annotate a file, or the clipboard image.
    Edit {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// QR helpers.
    Qr {
        #[command(subcommand)]
        mode: QrMode,
    },
    /// List the built-in actions.
    Actions,
    /// List every pipeline step and whether it works here.
    Steps {
        /// Only print steps that can run on this host.
        #[arg(long)]
        available: bool,
    },
    /// Probe the host and report what works.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Show or check the configuration.
    Config {
        /// Print the config file path.
        #[arg(long)]
        path: bool,
        /// Print the documented default config.
        #[arg(long)]
        default: bool,
        /// Parse and validate without running anything.
        #[arg(long)]
        check: bool,
    },
    /// Show recent history.
    History {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Only uploads.
        #[arg(long)]
        uploads: bool,
        /// Delete all history.
        #[arg(long)]
        clear: bool,
    },
    /// Print hotkey snippets, or grab X11 hotkeys.
    Hotkeys {
        /// Print a snippet for this desktop (niri, sway, hyprland, gnome, kde).
        #[arg(long)]
        print: Option<String>,
        /// Check that every configured accelerator parses.
        #[arg(long)]
        check: bool,
        /// Grab the hotkeys on X11 (blocks).
        #[arg(long)]
        grab: bool,
    },
    /// Run or control the background daemon.
    Daemon {
        #[arg(long)]
        detach: bool,
        #[command(subcommand)]
        action: Option<DaemonCommand>,
    },
    /// Run an action in the daemon (or in-process when it is not running).
    Trigger {
        /// Action name, e.g. region, qr_scan, gif.
        action: String,
    },
    /// One-line status of the toolchain and daemon.
    Status,
    /// Generate shell completions.
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
enum QrMode {
    /// Decode a QR code from the screen.
    Scan,
    /// Encode text into a QR image.
    Make { text: Option<String> },
}

#[derive(Subcommand, Debug)]
enum DaemonCommand {
    /// Start the daemon in the background.
    Start,
    /// Ask the daemon to exit.
    Stop,
    /// Print the daemon status.
    Status,
    /// Reload the config in a running daemon.
    Reload,
}

fn main() -> ExitCode {
    // Behave like a normal Unix tool when stdout is a closed pipe (`| head`).
    // Rust ignores SIGPIPE by default, which turns a closed pipe into a panic.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if is_cancelled(&err) => {
            eprintln!("cancelled");
            ExitCode::SUCCESS
        }
        Err(err) => {
            if is_user_error(&err) {
                eprintln!("{}: {err:#}", APP_NAME);
            } else {
                eprintln!("{}: {err:#}", APP_NAME);
            }
            ExitCode::FAILURE
        }
    }
}

fn init_tracing(verbose: u8) {
    use tracing_subscriber::EnvFilter;
    let default = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}

fn discover_paths(cli: &Cli) -> Paths {
    let _ = cli;
    Paths::discover()
}

fn load_config(cli: &Cli, paths: &Paths) -> Result<Config> {
    match &cli.config {
        Some(path) => Config::from_path(path),
        None => Config::load(paths),
    }
}

fn context(cli: &Cli, action_name: &str) -> Result<Ctx> {
    let paths = discover_paths(cli);
    paths.ensure()?;
    let config = load_config(cli, &paths)?;
    let mut ctx = Ctx::new(paths, config, ToolSet::detect(), Desktop::detect(), action_name)?;
    ctx.dry_run = cli.dry_run;
    Ok(ctx)
}

fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Command::Actions => {
            println!("{:<16} {}", "ACTION", "WHAT IT DOES");
            for (name, description) in action::ACTIONS {
                println!("{name:<16} {description}");
            }
            Ok(())
        }

        Command::Steps { available } => {
            let paths = discover_paths(&cli);
            let config = load_config(&cli, &paths)?;
            let tools = ToolSet::detect();
            let desktop = Desktop::detect();
            let probe = ss_belt::steps::Probe { tools: &tools, desktop: &desktop, config: &config };
            println!("{:<18} {:<10} {:<10} {}", "STEP", "IN", "OUT", "STATUS");
            for spec in ss_belt::steps::registry() {
                let support = spec.support(&probe);
                if *available && !support.ready() {
                    continue;
                }
                let detail = support.detail().map(|d| format!("  ({d})")).unwrap_or_default();
                println!(
                    "{:<18} {:<10} {:<10} {}{}",
                    spec.name,
                    spec.input.as_str(),
                    spec.output.as_str(),
                    support.label(),
                    detail
                );
            }
            Ok(())
        }

        Command::Doctor { json } => {
            let paths = discover_paths(&cli);
            paths.ensure()?;
            let config = load_config(&cli, &paths)?;
            ss_belt::doctor::print(&paths, &config, *json)
        }

        Command::Config { path, default, check } => {
            let paths = discover_paths(&cli);
            if *path {
                println!("{}", paths.config_file().display());
                return Ok(());
            }
            if *default {
                print!("{}", ss_belt::config::DEFAULT_CONFIG);
                return Ok(());
            }
            if *check {
                let config = load_config(&cli, &paths)?;
                println!("ok: {} is valid", paths.config_file().display());
                println!("  save dir   {}", config.save_dir().display());
                println!("  actions    {}", config.actions.len() + action::DEFAULT_ACTIONS.len());
                println!("  uploads    {}", config.upload.destinations.len());
                return Ok(());
            }
            let config = load_config(&cli, &paths)?;
            println!("{}", toml::to_string_pretty(&config)?);
            Ok(())
        }

        Command::History { limit, uploads, clear } => {
            let paths = discover_paths(&cli);
            paths.ensure()?;
            let history = History::open(&paths)?;
            if *clear {
                let removed = history.prune(0, 1)?;
                println!("cleared {removed} history entries");
                return Ok(());
            }
            let rows = if *uploads { history.uploads(*limit)? } else { history.recent(*limit)? };
            if rows.is_empty() {
                println!("no history yet");
            }
            for row in rows.iter().rev() {
                println!("{}", row.line());
            }
            Ok(())
        }

        Command::Hotkeys { print, check, grab } => {
            let paths = discover_paths(&cli);
            let config = load_config(&cli, &paths)?;
            if *check {
                for action in ss_belt::config::HOTKEY_ACTIONS {
                    match config.hotkeys.get(action) {
                        Some(accelerator) => match ss_belt::hotkeys::parse_accelerator(accelerator) {
                            Ok(parsed) => println!("{action:<16} {accelerator:<18} ok ({})", parsed.label()),
                            Err(err) => println!("{action:<16} {accelerator:<18} INVALID: {err}"),
                        },
                        None => println!("{action:<16} (not bound)"),
                    }
                }
                return Ok(());
            }
            if *grab {
                let socket_paths = discover_paths(&cli);
                return ss_belt::hotkeys::x11::grab(&config, move |action_name| {
                    let _ = ss_belt::daemon::send(&socket_paths, &format!("action {action_name}"));
                });
            }
            let desktop = print.clone().unwrap_or_else(|| {
                let detected = Desktop::detect();
                if detected.desktop.is_empty() {
                    "generic".to_string()
                } else {
                    detected.desktop
                }
            });
            print!("{}", ss_belt::hotkeys::generate::print(&desktop, &config, &paths)?);
            Ok(())
        }

        Command::Daemon { detach, action } => match action {
            Some(DaemonCommand::Start) => {
                let paths = discover_paths(&cli);
                let config = load_config(&cli, &paths)?;
                ss_belt::daemon::ensure_running(&paths, &config)?;
                println!("daemon running on {}", paths.socket_file().display());
                Ok(())
            }
            Some(DaemonCommand::Stop) => {
                let paths = discover_paths(&cli);
                println!("{}", ss_belt::daemon::send(&paths, "quit")?);
                Ok(())
            }
            Some(DaemonCommand::Status) => {
                let paths = discover_paths(&cli);
                println!("{}", ss_belt::daemon::send(&paths, "status")?);
                Ok(())
            }
            Some(DaemonCommand::Reload) => {
                let paths = discover_paths(&cli);
                println!("{}", ss_belt::daemon::send(&paths, "reload")?);
                Ok(())
            }
            None => {
                let paths = discover_paths(&cli);
                let config = load_config(&cli, &paths)?;
                if *detach {
                    let exe = std::env::current_exe()?;
                    let log = paths.log_file();
                    let stderr = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log)
                        .map(std::process::Stdio::from)
                        .unwrap_or_else(|_| std::process::Stdio::null());
                    std::process::Command::new(exe)
                        .arg("daemon")
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(stderr)
                        .spawn()
                        .context("detaching the daemon")?;
                    eprintln!("daemon log: {}", log.display());
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    println!("daemon started on {}", paths.socket_file().display());
                    Ok(())
                } else {
                    ss_belt::daemon::run(paths, config)
                }
            }
        },

        Command::Trigger { action } => {
            if !ss_belt::action::ACTIONS.iter().any(|(name, _)| name == action) {
                return Err(user(format!("unknown action `{action}`"))
                    .hint("run `ss-belt actions` for the list"));
            }
            let paths = discover_paths(&cli);
            if ss_belt::daemon::is_running(&paths) {
                let reply = ss_belt::daemon::send(&paths, &format!("action {action}"))?;
                println!("{reply}");
                return Ok(());
            }
            let mut ctx = context(&cli, action)?;
            run_action(&mut ctx, action, None)?;
            print_value(&ctx.value);
            Ok(())
        }

        Command::Status => {
            let paths = discover_paths(&cli);
            let config = load_config(&cli, &paths)?;
            println!("{}", ss_belt::doctor::summary_line(&paths, &config)?);
            Ok(())
        }

        Command::Completion { shell } => {
            let mut command = Cli::command();
            clap_complete::generate(*shell, &mut command, "ss-belt", &mut std::io::stdout());
            Ok(())
        }

        Command::Upload { file, destination } => {
            let mut ctx = context(&cli, "upload")?;
            ctx.upload_destination = destination.clone();
            if let Some(file) = file {
                if !file.exists() {
                    return Err(user(format!("{} does not exist", file.display())));
                }
                ctx.value = if ss_belt::util::is_video_ext(file) {
                    Value::Video(file.clone())
                } else {
                    Value::Image(file.clone())
                };
                run_steps(&mut ctx, &["upload", "clipboard.copy", "notify"])?;
            } else {
                run_action(&mut ctx, "upload", None)?;
            }
            print_value(&ctx.value);
            Ok(())
        }

        Command::Edit { file } => {
            let mut ctx = context(&cli, "clipboard_edit")?;
            if let Some(file) = file {
                ctx.value = Value::Image(file.clone());
                run_steps(&mut ctx, &["annotate", "save", "clipboard.copy", "notify"])?;
            } else {
                run_action(&mut ctx, "clipboard_edit", None)?;
            }
            print_value(&ctx.value);
            Ok(())
        }

        Command::QrMake { text } => qr_make(&cli, text.clone()),

        Command::QrScan { file } => {
            let mut ctx = context(&cli, "qr_scan")?;
            match file {
                Some(file) => {
                    ctx.value = Value::Image(file.clone());
                    run_steps(&mut ctx, &["qr.scan", "clipboard.copy", "notify"])?;
                }
                None => run_action(&mut ctx, "qr_scan", None)?,
            }
            print_value(&ctx.value);
            Ok(())
        }

        Command::Ocr { file } => {
            let mut ctx = context(&cli, "ocr")?;
            match file {
                Some(file) => {
                    ctx.value = Value::Image(file.clone());
                    run_steps(&mut ctx, &["ocr", "clipboard.copy", "notify"])?;
                }
                None => run_action(&mut ctx, "ocr", None)?,
            }
            print_value(&ctx.value);
            Ok(())
        }

        Command::Qr { mode } => match mode {
            QrMode::Scan => {
                let mut ctx = context(&cli, "qr_scan")?;
                run_action(&mut ctx, "qr_scan", None)?;
                print_value(&ctx.value);
                Ok(())
            }
            QrMode::Make { text } => qr_make(&cli, text.clone()),
        },

        Command::Gif { duration } => run_record(&cli, "gif", *duration),
        Command::Video { duration } => run_record(&cli, "video", *duration),

        other => {
            let name = action_name(other);
            let mut ctx = context(&cli, name)?;
            run_action(&mut ctx, name, None)?;
            print_value(&ctx.value);
            Ok(())
        }
    }
}

fn qr_make(cli: &Cli, text: Option<String>) -> Result<()> {
    let mut ctx = context(cli, "qr_make")?;
    if let Some(text) = text {
        ctx.value = Value::Text(text);
        run_steps(&mut ctx, &["qr.make", "save", "clipboard.copy", "notify"])?;
    } else {
        run_action(&mut ctx, "qr_make", None)?;
    }
    print_value(&ctx.value);
    Ok(())
}

fn run_record(cli: &Cli, name: &str, duration: Option<u64>) -> Result<()> {
    let mut ctx = context(cli, name)?;
    run_action(&mut ctx, name, duration)?;
    print_value(&ctx.value);
    Ok(())
}

fn run_action(ctx: &mut Ctx, name: &str, duration: Option<u64>) -> Result<()> {
    match action::execute(ctx, name, duration) {
        Ok(Execution { message, .. }) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            Ok(())
        }
        Err(err) if is_cancelled(&err) => Ok(()),
        Err(err) => Err(err),
    }
}

fn run_steps(ctx: &mut Ctx, steps: &[&str]) -> Result<()> {
    let pipeline = ss_belt::pipeline::Pipeline {
        name: ctx.action.clone(),
        steps: steps.iter().map(|s| (*s).to_string()).collect(),
        custom: true,
        start: ctx.value.kind(),
    };
    match ss_belt::pipeline::run(ctx, &pipeline) {
        Ok(_) => Ok(()),
        Err(err) if is_cancelled(&err) => Ok(()),
        Err(err) => Err(err),
    }
}

fn action_name(command: &Command) -> &'static str {
    match command {
        Command::Region => "region",
        Command::Full => "full",
        Command::Window => "window",
        Command::Monitor => "monitor",
        Command::Annotate => "annotate",
        Command::ClipboardEdit => "clipboard_edit",
        Command::QrScan { .. } => "qr_scan",
        Command::QrMake { .. } => "qr_make",
        Command::Ocr { .. } => "ocr",
        Command::Upload { .. } => "upload",
        Command::AnnotateLast => "annotate_last",
        Command::Gif { .. } => "gif",
        Command::Video { .. } => "video",
        _ => "region",
    }
}

/// Print the pipeline result in a form that composes: a path, a URL or text.
fn print_value(value: &Value) {
    match value {
        Value::Image(path) | Value::Video(path) => println!("{}", path.display()),
        Value::Url(url) => println!("{url}"),
        Value::Text(text) => println!("{text}"),
        Value::Empty => {}
    }
}

#[allow(dead_code)]
fn version_line() -> String {
    format!("{APP_NAME} {VERSION}")
}
