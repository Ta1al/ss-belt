use anyhow::Result;
use std::time::Duration;

use crate::capabilities::ToolId;
use crate::error::{cancelled, user};
use crate::exec::Cmd;
use crate::steps::{Ctx, Probe, Support};
use crate::error::HintExt;

pub fn support(probe: &Probe<'_>) -> Support {
    match probe.config.general.annotator.as_str() {
        "none" => Support::Degraded("annotation is disabled in the config".into()),
        "satty" if probe.tools.has(ToolId::Satty) => Support::Ready,
        "satty" => Support::Unavailable("annotator = \"satty\" but satty is not installed".into()),
        "swappy" if probe.tools.has(ToolId::Swappy) => Support::Ready,
        "swappy" => Support::Unavailable("annotator = \"swappy\" but swappy is not installed".into()),
        _ if probe.tools.has(ToolId::Satty) || probe.tools.has(ToolId::Swappy) => Support::Ready,
        _ => Support::Unavailable(
            "install satty (recommended) or swappy for post-capture annotation".into(),
        ),
    }
}

fn pick(ctx: &Ctx) -> Option<ToolId> {
    match ctx.config.general.annotator.as_str() {
        "none" => None,
        "satty" => ctx.tools.has(ToolId::Satty).then_some(ToolId::Satty),
        "swappy" => ctx.tools.has(ToolId::Swappy).then_some(ToolId::Swappy),
        _ => {
            if ctx.tools.has(ToolId::Satty) {
                Some(ToolId::Satty)
            } else if ctx.tools.has(ToolId::Swappy) {
                Some(ToolId::Swappy)
            } else {
                None
            }
        }
    }
}

/// Open the image in the configured annotator.
///
/// Both supported annotators are post-capture editors: they receive a frozen
/// image. A cancelled window aborts the pipeline without saving anything.
pub fn run(ctx: &mut Ctx) -> Result<()> {
    let input = ctx.require_image("annotate")?;

    if ctx.config.general.annotator == "none" {
        ctx.note("annotation disabled by config, keeping the capture as-is");
        return Ok(());
    }

    let Some(tool) = pick(ctx) else {
        return Err(user("no annotator installed").hint(
            "install satty or swappy, or set general.annotator = \"none\"",
        ));
    };

    if ctx.dry_run {
        ctx.note(format!("dry-run: would open {} in {}", input.display(), tool.binary()));
        return Ok(());
    }

    let output = ctx.stage("annotated.png");
    let _ = std::fs::remove_file(&output);

    let program = ctx.tools.program(tool);
    let result = match tool {
        ToolId::Satty => Cmd::new(&program)
            .args(["-f"])
            .arg_path(&input)
            .args(["-o"])
            .arg_path(&output)
            .args(["--early-exit"])
            .timeout(Duration::from_secs(60 * 30))
            .run()?,
        _ => Cmd::new(&program)
            .args(["-f"])
            .arg_path(&input)
            .args(["-o"])
            .arg_path(&output)
            .timeout(Duration::from_secs(60 * 30))
            .run()?,
    };

    if !output.exists() || std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0) == 0 {
        if result.success() {
            return Err(cancelled());
        }
        return Err(user(format!("{} failed: {}", tool.binary(), result.failure())));
    }

    ctx.note(format!("annotated with {}", tool.binary()));
    ctx.value = crate::steps::Value::Image(output);
    Ok(())
}
