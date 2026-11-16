use anyhow::{Context, Result};
use std::fs;

use crate::error::user;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::util;

pub fn support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

/// Move the staged artifact into the configured save directory.
pub fn run(ctx: &mut Ctx) -> Result<()> {
    let (source, is_video) = match &ctx.value {
        Value::Image(path) => (path.clone(), false),
        Value::Video(path) => (path.clone(), true),
        other => {
            return Err(user(format!("nothing to save: the pipeline holds {}", other.describe())))
        }
    };

    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png").to_string();
    let destination = ctx.output_path(&ext);

    if ctx.dry_run {
        ctx.note(format!("dry-run: would save {} to {}", source.display(), destination.display()));
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating save directory {}", parent.display()))?;
    }
    fs::copy(&source, &destination)
        .with_context(|| format!("saving {} to {}", source.display(), destination.display()))?;

    let size = fs::metadata(&destination).map(|m| m.len()).unwrap_or(0);
    ctx.note(format!(
        "saved {} ({})",
        destination.display(),
        util::human_bytes(size)
    ));

    fs::write(ctx.paths.last_file(), destination.display().to_string())?;

    ctx.value = if is_video { Value::Video(destination.clone()) } else { Value::Image(destination.clone()) };

    if ctx.config.general.open_after_save {
        let _ = crate::exec::Cmd::new("xdg-open")
            .arg_path(&destination)
            .timeout(std::time::Duration::from_secs(2))
            .quiet()
            .run();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn saves_into_the_configured_directory_and_tracks_last() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("region", dir.path());
        let staged = ctx.stage("capture.png");
        fs::write(&staged, b"not really a png").unwrap();
        ctx.value = Value::Image(staged);

        run(&mut ctx).unwrap();

        let saved = match &ctx.value {
            Value::Image(path) => path.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert!(saved.starts_with(ctx.config.save_dir()));
        assert!(saved.exists());
        assert_eq!(fs::read_to_string(ctx.paths.last_file()).unwrap(), saved.display().to_string());
    }

    #[test]
    fn saving_text_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("ocr", dir.path());
        ctx.value = Value::Text("hello".into());
        assert!(run(&mut ctx).is_err());
    }
}
