use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::capabilities::{SessionType, ToolId};
use crate::error::{cancelled, missing_tools, user};
use crate::exec::{Cmd, Output};
use crate::steps::{Ctx, Probe, Support, Value};
use crate::error::HintExt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Region,
    Full,
    Window,
    Monitor,
}

impl Target {
    pub fn as_str(self) -> &'static str {
        match self {
            Target::Region => "region",
            Target::Full => "full",
            Target::Window => "window",
            Target::Monitor => "monitor",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Backend {
    Grim,
    Spectacle,
    Gnome,
    X11,
}

impl Backend {
    fn label(self) -> &'static str {
        match self {
            Backend::Grim => "grim",
            Backend::Spectacle => "spectacle",
            Backend::Gnome => "gnome-screenshot",
            Backend::X11 => "ffmpeg x11grab",
        }
    }
}

fn requested_backend(probe: &Probe<'_>) -> Option<Backend> {
    match probe.config.capture.backend.as_str() {
        "grim" => Some(Backend::Grim),
        "spectacle" => Some(Backend::Spectacle),
        "gnome" => Some(Backend::Gnome),
        "x11" | "import" => Some(Backend::X11),
        _ => None,
    }
}

fn select(probe: &Probe<'_>, target: Target) -> Result<Backend> {
    if let Some(backend) = requested_backend(probe) {
        return Ok(backend);
    }

    let tools = probe.tools;
    let desktop = probe.desktop;

    if desktop.is_wayland() && desktop.is_wlroots && tools.has(ToolId::Grim) {
        return Ok(Backend::Grim);
    }
    if desktop.is_wayland() && desktop.is_kde() && tools.has(ToolId::Spectacle) {
        return Ok(Backend::Spectacle);
    }
    if desktop.is_wayland() && desktop.is_gnome() && tools.has(ToolId::GnomeScreenshot) {
        return Ok(Backend::Gnome);
    }
    if tools.has(ToolId::Grim) && desktop.is_wlroots {
        return Ok(Backend::Grim);
    }
    if tools.has(ToolId::Spectacle) && desktop.is_kde() {
        return Ok(Backend::Spectacle);
    }
    if tools.has(ToolId::GnomeScreenshot) && desktop.is_gnome() {
        return Ok(Backend::Gnome);
    }
    if tools.has(ToolId::Ffmpeg) && (desktop.has_xwayland || desktop.session == SessionType::X11) {
        return Ok(Backend::X11);
    }

    let _ = target;
    Err(missing_tools("screen capture", &recommended_tools(desktop)))
}

fn recommended_tools(desktop: &crate::capabilities::Desktop) -> Vec<ToolId> {
    if desktop.is_kde() {
        vec![ToolId::Spectacle]
    } else if desktop.is_gnome() {
        vec![ToolId::GnomeScreenshot]
    } else if desktop.is_wayland() {
        vec![ToolId::Grim, ToolId::Slurp]
    } else {
        vec![ToolId::Ffmpeg]
    }
}

fn support_for(probe: &Probe<'_>, target: Target) -> Support {
    match select(probe, target) {
        Ok(backend) => match (backend, target) {
            (Backend::Grim, Target::Window) => Support::Unavailable(
                "Wayland has no window enumeration; capture the region instead".into(),
            ),
            (Backend::X11, Target::Region) => Support::Unavailable(
                "region capture needs a Wayland selector (grim + slurp) or a full-screen \
                 capture you crop afterwards"
                    .into(),
            ),
            (Backend::X11, Target::Window) => Support::Unavailable(
                "the ffmpeg X11 backend cannot pick a window; capture the screen instead".into(),
            ),
            (Backend::Grim, Target::Region) if !probe.tools.has(ToolId::Slurp) => {
                Support::Unavailable("region capture with grim needs slurp".into())
            }
            (Backend::Grim, Target::Monitor) => Support::Degraded(
                "grim captures all outputs; trim the result or install a per-monitor tool".into(),
            ),
            (Backend::Gnome, Target::Monitor) => {
                Support::Degraded("gnome-screenshot always captures every monitor".into())
            }
            (Backend::X11, _) if probe.desktop.is_wayland() => Support::Degraded(
                "XWayland only: native Wayland windows will be missing from the capture".into(),
            ),
            (Backend::X11, Target::Monitor) => {
                Support::Degraded("X11 captures the whole screen, not one monitor".into())
            }
            _ => Support::Ready,
        },
        Err(err) => Support::Unavailable(err.to_string()),
    }
}

pub fn region_support(probe: &Probe<'_>) -> Support {
    support_for(probe, Target::Region)
}

pub fn full_support(probe: &Probe<'_>) -> Support {
    support_for(probe, Target::Full)
}

pub fn window_support(probe: &Probe<'_>) -> Support {
    support_for(probe, Target::Window)
}

pub fn monitor_support(probe: &Probe<'_>) -> Support {
    support_for(probe, Target::Monitor)
}

pub fn region_run(ctx: &mut Ctx) -> Result<()> {
    run_target(ctx, Target::Region)
}

pub fn full_run(ctx: &mut Ctx) -> Result<()> {
    run_target(ctx, Target::Full)
}

pub fn window_run(ctx: &mut Ctx) -> Result<()> {
    run_target(ctx, Target::Window)
}

pub fn monitor_run(ctx: &mut Ctx) -> Result<()> {
    run_target(ctx, Target::Monitor)
}

fn run_target(ctx: &mut Ctx, target: Target) -> Result<()> {
    let probe = Probe { tools: &ctx.tools, desktop: &ctx.desktop, config: &ctx.config };
    let backend = select(&probe, target)?;
    let support = support_for(&probe, target);
    if !support.ready() {
        return Err(user(support.detail().unwrap_or("this capture mode is not available").to_string()));
    }
    if let Support::Degraded(detail) = &support {
        ctx.note(detail.clone());
    }

    if ctx.dry_run {
        ctx.note(format!(
            "dry-run: would capture {} with {}",
            target.as_str(),
            backend.label()
        ));
        ctx.value = Value::Image(ctx.stage("capture.png"));
        return Ok(());
    }

    ctx.sleep_delay();
    let output = ctx.stage("capture.png");
    let _ = fs::remove_file(&output);

    match backend {
        Backend::Grim => grim(ctx, target, &output)?,
        Backend::Spectacle => spectacle(ctx, target, &output)?,
        Backend::Gnome => gnome(ctx, target, &output)?,
        Backend::X11 => x11(ctx, target, &output)?,
    }

    finish(ctx, &output, backend, target)
}

fn finish(ctx: &mut Ctx, output: &Path, backend: Backend, target: Target) -> Result<()> {
    if !output.exists() {
        return Err(user(format!("{} produced no image", backend.label())));
    }
    let size = fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        let _ = fs::remove_file(output);
        return Err(cancelled());
    }

    let (width, height) = image::image_dimensions(output).unwrap_or((0, 0));
    if width == 0 || height == 0 {
        return Err(user(format!("{} produced an unreadable image", backend.label())));
    }

    ctx.note(format!("captured {} {width}x{height} with {}", target.as_str(), backend.label()));
    ctx.value = Value::Image(output.to_path_buf());
    Ok(())
}

fn check(_ctx: &Ctx, output: &Output, what: &str) -> Result<()> {
    if output.success() {
        return Ok(());
    }
    let message = output.failure();
    if message.contains("cancelled") || message.contains("Cancel") {
        return Err(cancelled());
    }
    Err(user(format!("{what} failed: {message}")))
}

fn grim(ctx: &mut Ctx, target: Target, output: &Path) -> Result<()> {
    let program = ctx.tools.program(ToolId::Grim);
    let cursor = ctx.config.capture.cursor;

    match target {
        Target::Region => {
            let slurp = ctx.tools.program(ToolId::Slurp);
            let selection = Cmd::new(&slurp)
                .timeout(Duration::from_secs(120))
                .run()?;
            if !selection.success() {
                return Err(cancelled());
            }
            let geometry = selection.stdout_str();
            if geometry.trim().is_empty() {
                return Err(cancelled());
            }
            let mut cmd = Cmd::new(&program).args(["-g", &geometry]);
            if cursor {
                cmd = cmd.arg("-c");
            }
            let result = cmd.arg_path(output).timeout(Duration::from_secs(60)).run()?;
            check(ctx, &result, "grim")?;
        }
        Target::Full | Target::Monitor => {
            let mut cmd = Cmd::new(&program);
            if cursor {
                cmd = cmd.arg("-c");
            }
            let result = cmd.arg_path(output).timeout(Duration::from_secs(60)).run()?;
            check(ctx, &result, "grim")?;
        }
        Target::Window => {
            return Err(user("window capture is not possible on Wayland with grim")
                .hint("use region capture instead"));
        }
    }
    Ok(())
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > 0 {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn spectacle(ctx: &mut Ctx, target: Target, output: &Path) -> Result<()> {
    let program = ctx.tools.program(ToolId::Spectacle);
    let flag = match target {
        Target::Region => "-r",
        Target::Full => "-f",
        Target::Window => "-a",
        Target::Monitor => "-m",
    };
    let result = Cmd::new(&program)
        .args(["-b", "-n", flag, "-o"])
        .arg_path(output)
        .timeout(Duration::from_secs(120))
        .run()?;
    check(ctx, &result, "spectacle")?;
    if !wait_for_file(output, Duration::from_secs(20)) {
        // Spectacle returns before writing when it runs in background mode; a
        // missing file after the wait means the user cancelled the region.
        return Err(cancelled());
    }
    Ok(())
}

fn gnome(ctx: &mut Ctx, target: Target, output: &Path) -> Result<()> {
    let program = ctx.tools.program(ToolId::GnomeScreenshot);
    let mut cmd = Cmd::new(&program);
    match target {
        Target::Region => cmd = cmd.arg("-a"),
        Target::Window => cmd = cmd.arg("-w"),
        _ => {}
    }
    let result = cmd.arg("-f").arg_path(output).timeout(Duration::from_secs(120)).run()?;
    check(ctx, &result, "gnome-screenshot")?;
    if !wait_for_file(output, Duration::from_secs(5)) {
        return Err(cancelled());
    }
    Ok(())
}

/// X11 capture through ffmpeg's x11grab input.
///
/// A native `GetImage` implementation was tried first and rejected: XWayland
/// (and some X servers) answer `BadMatch` for a root-window grab, while
/// x11grab works everywhere and is already a supported dependency.
fn x11(ctx: &mut Ctx, target: Target, output: &Path) -> Result<()> {
    if target == Target::Window || target == Target::Region {
        return Err(user(format!("{} is not available on the X11 backend", target.as_str())));
    }
    let display = std::env::var("DISPLAY").map_err(|_| {
        user("X11 capture needs DISPLAY").hint("run ss-belt inside a graphical session")
    })?;

    let result = Cmd::new(ctx.tools.program(ToolId::Ffmpeg))
        .args(["-hide_banner", "-loglevel", "error", "-f", "x11grab", "-i", &display])
        .args(["-frames:v", "1", "-y"])
        .arg_path(output)
        .timeout(Duration::from_secs(60))
        .run()?;

    if !result.success() {
        return Err(user(format!("ffmpeg x11grab failed: {}", result.failure())));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn no_tools_no_backend() {
        let tools = testing::tools(&[]);
        let desktop = testing::wayland();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(!region_support(&probe).ready());
    }

    #[test]
    fn grim_on_wlroots_needs_slurp_for_regions() {
        let tools = testing::tools(&[ToolId::Grim]);
        let desktop = testing::wayland();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(full_support(&probe).ready());
        let region = region_support(&probe);
        assert!(!region.ready());
        assert!(region.detail().unwrap().contains("slurp"));
    }

    #[test]
    fn explicit_backend_is_honoured() {
        let tools = testing::tools(&[ToolId::Grim]);
        let desktop = testing::wayland();
        let mut config = crate::config::Config::default();
        config.capture.backend = "grim".into();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(matches!(select(&probe, Target::Full), Ok(Backend::Grim)));
    }

    #[test]
    fn x11_backend_needs_ffmpeg_and_cannot_select_a_region() {
        let tools = testing::tools(&[ToolId::Ffmpeg]);
        let desktop = testing::x11();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(full_support(&probe).ready());
        assert!(!region_support(&probe).ready());
        assert!(!window_support(&probe).ready());
    }

    #[test]
    fn without_ffmpeg_there_is_no_x11_backend() {
        let tools = testing::tools(&[]);
        let desktop = testing::x11();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        assert!(!full_support(&probe).ready());
    }
}
