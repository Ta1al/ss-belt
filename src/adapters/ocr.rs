use anyhow::Result;
use std::time::Duration;

use crate::capabilities::ToolId;
use crate::error::user;
use crate::exec::Cmd;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::error::HintExt;

pub fn support(probe: &Probe<'_>) -> Support {
    if probe.tools.has(ToolId::Tesseract) {
        Support::Ready
    } else {
        Support::Unavailable("OCR needs tesseract".into())
    }
}

pub fn run(ctx: &mut Ctx) -> Result<()> {
    let path = ctx.require_image("ocr")?;

    if !ctx.tools.has(ToolId::Tesseract) {
        return Err(user("OCR needs tesseract").hint("install tesseract, then retry"));
    }

    if ctx.dry_run {
        ctx.note(format!("dry-run: would run OCR on {}", path.display()));
        ctx.value = Value::Text("<ocr>".into());
        return Ok(());
    }

    let output = Cmd::new(ctx.tools.program(ToolId::Tesseract))
        .arg_path(&path)
        .args(["stdout", "--psm", "6"])
        .timeout(Duration::from_secs(120))
        .checked()?;

    let text = output.stdout_str();
    if text.trim().is_empty() {
        return Err(user("OCR found no text in the capture")
            .hint("try a tighter selection or a higher resolution capture"));
    }

    ctx.note(format!("extracted {} characters", text.chars().count()));
    ctx.value = Value::Text(text);
    Ok(())
}
