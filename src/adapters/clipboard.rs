use anyhow::Result;
use std::fs;
use std::time::Duration;

use crate::capabilities::{SessionType, ToolId};
use crate::error::{missing_tools, user};
use crate::exec::Cmd;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::util;
use crate::error::HintExt;

pub fn support(probe: &Probe<'_>) -> Support {
    let wl = probe.tools.has(ToolId::WlCopy) && probe.tools.has(ToolId::WlPaste);
    let x11 = probe.tools.has(ToolId::Xclip);

    match probe.desktop.session {
        SessionType::Wayland if wl => Support::Ready,
        SessionType::Wayland if x11 => Support::Degraded(
            "wl-clipboard is missing; falling back to the X11 clipboard (XWayland apps only)".into(),
        ),
        SessionType::X11 if x11 => Support::Ready,
        SessionType::None => Support::Unavailable("no graphical session found".into()),
        _ => Support::Unavailable(
            "install wl-clipboard (Wayland) or xclip (X11)".into(),
        ),
    }
}

fn use_wayland(ctx: &Ctx) -> bool {
    match ctx.config.clipboard.backend.as_str() {
        "wl" => true,
        "x11" => false,
        _ => {
            ctx.desktop.is_wayland()
                && ctx.tools.has(ToolId::WlCopy)
                && ctx.tools.has(ToolId::WlPaste)
        }
    }
}

pub fn copy_run(ctx: &mut Ctx) -> Result<()> {
    let value = ctx.value.clone();
    match &value {
        Value::Empty => return Err(user("nothing to copy")),
        Value::Image(path) => {
            if ctx.dry_run {
                ctx.note(format!("dry-run: would copy {} to the clipboard", path.display()));
                return Ok(());
            }
            copy_image(ctx, path)?;
            ctx.note("copied image to the clipboard");
        }
        Value::Video(path) => {
            if ctx.dry_run {
                ctx.note(format!("dry-run: would copy file {} to the clipboard", path.display()));
                return Ok(());
            }
            let uri = format!("file://{}", path.display());
            copy_text(ctx, &uri)?;
            ctx.note("copied video location to the clipboard");
        }
        Value::Text(text) | Value::Url(text) => {
            if ctx.dry_run {
                ctx.note(format!("dry-run: would copy {} chars to the clipboard", text.chars().count()));
                return Ok(());
            }
            copy_text(ctx, text)?;
            ctx.note(format!("copied {} characters to the clipboard", text.chars().count()));
        }
    }
    Ok(())
}

fn copy_image(ctx: &Ctx, path: &std::path::Path) -> Result<()> {
    let mime = util::mime_for_path(path);
    if use_wayland(ctx) {
        let bytes = fs::read(path)?;
        Cmd::new(ctx.tools.program(ToolId::WlCopy))
            .args(["--type", mime])
            .stdin(bytes)
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    } else if ctx.tools.has(ToolId::Xclip) {
        Cmd::new(ctx.tools.program(ToolId::Xclip))
            .args(["-selection", "clipboard", "-t", mime, "-i"])
            .arg_path(path)
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    } else {
        return Err(missing_tools("the clipboard", &[ToolId::WlCopy, ToolId::Xclip]));
    }
    Ok(())
}

fn copy_text(ctx: &Ctx, text: &str) -> Result<()> {
    if use_wayland(ctx) {
        Cmd::new(ctx.tools.program(ToolId::WlCopy))
            .stdin(text.as_bytes().to_vec())
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    } else if ctx.tools.has(ToolId::Xclip) {
        Cmd::new(ctx.tools.program(ToolId::Xclip))
            .args(["-selection", "clipboard", "-i"])
            .stdin(text.as_bytes().to_vec())
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    } else {
        return Err(missing_tools("the clipboard", &[ToolId::WlCopy, ToolId::Xclip]));
    }
    Ok(())
}

pub fn paste_run(ctx: &mut Ctx) -> Result<()> {
    if ctx.dry_run {
        ctx.note("dry-run: would read the clipboard");
        ctx.value = Value::Image(ctx.stage("clipboard.png"));
        return Ok(());
    }

    let pasted = if use_wayland(ctx) { paste_wayland(ctx)? } else { paste_x11(ctx)? };

    match pasted {
        Some(value) => {
            ctx.note(format!("clipboard holds {}", value.describe()));
            ctx.value = value;
            Ok(())
        }
        None => {
            // A Wayland clipboard tool can be present while only X11 clients own
            // the selection; try the other backend before giving up.
            let fallback = if use_wayland(ctx) { paste_x11(ctx)? } else { paste_wayland(ctx)? };
            match fallback {
                Some(value) => {
                    ctx.value = value;
                    Ok(())
                }
                None => Err(user("the clipboard is empty or holds an unsupported format")
                    .hint("copy an image or some text first")),
            }
        }
    }
}

fn paste_wayland(ctx: &Ctx) -> Result<Option<Value>> {
    if !(ctx.tools.has(ToolId::WlPaste) && ctx.tools.has(ToolId::WlCopy)) {
        return Ok(None);
    }
    let program = ctx.tools.program(ToolId::WlPaste);

    // Ask for the offered types so PNG, JPEG and plain text all work.
    let types = Cmd::new(&program)
        .arg("--list-types")
        .timeout(Duration::from_secs(5))
        .run()?;
    if !types.success() {
        return Ok(None);
    }
    let types = types.stdout_str();
    let image_mime = types
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("image/"));

    if let Some(mime) = image_mime {
        let out = Cmd::new(&program)
            .args(["--no-newline", "--type", mime])
            .timeout(Duration::from_secs(20))
            .run()?;
        if out.success() && !out.stdout.is_empty() {
            let ext = util::ext_for_mime(mime);
            let path = ctx.stage(&format!("clipboard.{ext}"));
            fs::write(&path, &out.stdout)?;
            return Ok(Some(Value::Image(path)));
        }
    }

    let out = Cmd::new(&program)
        .arg("--no-newline")
        .timeout(Duration::from_secs(5))
        .run()?;
    if out.success() && !out.stdout.is_empty() {
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let text = text.trim_end_matches('\0').to_string();
        if !text.trim().is_empty() {
            return Ok(Some(Value::Text(text)));
        }
    }
    Ok(None)
}

fn paste_x11(ctx: &Ctx) -> Result<Option<Value>> {
    if !ctx.tools.has(ToolId::Xclip) {
        return Ok(None);
    }
    let program = ctx.tools.program(ToolId::Xclip);

    for mime in ["image/png", "image/jpeg"] {
        let out = Cmd::new(&program)
            .args(["-selection", "clipboard", "-t", mime, "-o"])
            .timeout(Duration::from_secs(10))
            .run()?;
        if out.success() && out.stdout.len() > 8 {
            let ext = util::ext_for_mime(mime);
            let path = ctx.stage(&format!("clipboard.{ext}"));
            fs::write(&path, &out.stdout)?;
            return Ok(Some(Value::Image(path)));
        }
    }

    let out = Cmd::new(&program)
        .args(["-selection", "clipboard", "-o"])
        .timeout(Duration::from_secs(5))
        .run()?;
    if out.success() && !out.stdout.is_empty() {
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if !text.trim().is_empty() {
            return Ok(Some(Value::Text(text)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn wayland_without_tools_degrades_or_fails_clearly() {
        let tools = testing::tools(&[]);
        let desktop = testing::wayland();
        let config = crate::config::Config::default();
        let probe = Probe { tools: &tools, desktop: &desktop, config: &config };
        let support = support(&probe);
        assert!(!support.ready() || matches!(support, Support::Degraded(_)));
    }

    #[test]
    fn dry_run_copy_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("clipboard", dir.path());
        ctx.dry_run = true;
        ctx.value = Value::Text("hello".into());
        copy_run(&mut ctx).unwrap();
    }
}

/// A clipboard program, resolved without a [`Ctx`], for the notification helper.
#[derive(Debug, Clone)]
pub struct Writer {
    pub program: std::path::PathBuf,
    pub wayland: bool,
}

/// Pick the clipboard program that fits this session.
pub fn which_writer() -> Option<Writer> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty());
    if wayland {
        if let Some(program) = crate::exec::which("wl-copy") {
            return Some(Writer { program, wayland: true });
        }
    }
    crate::exec::which("xclip").map(|program| Writer { program, wayland: false })
}

pub fn write_text_with(writer: &Writer, text: &str) -> anyhow::Result<()> {
    if writer.wayland {
        Cmd::new(writer.program.display().to_string())
            .stdin(text.as_bytes().to_vec())
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    } else {
        Cmd::new(writer.program.display().to_string())
            .args(["-selection", "clipboard", "-i"])
            .stdin(text.as_bytes().to_vec())
            .timeout(Duration::from_secs(10))
            .quiet()
            .checked()?;
    }
    Ok(())
}
