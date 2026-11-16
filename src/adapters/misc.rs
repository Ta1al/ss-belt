use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::capabilities::ToolId;
use crate::error::user;
use crate::exec::Cmd;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::util;
use crate::error::HintExt;

pub fn notify_support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

/// Fire-and-forget notification. A missing notifier must never fail a pipeline.
pub fn send(ctx: &Ctx, summary: &str, body: &str) {
    if !ctx.tools.has(ToolId::NotifySend) {
        tracing::info!("{summary}: {body}");
        return;
    }
    let _ = Cmd::new(ctx.tools.program(ToolId::NotifySend))
        .args(["-a", crate::APP_NAME, "-t", "4000", summary, body])
        .timeout(Duration::from_secs(3))
        .quiet()
        .run();
}

/// One notification per capture, and clicking it opens the result.
pub fn notify_run(ctx: &mut Ctx) -> Result<()> {
    let (summary, body, target) = match &ctx.value {
        Value::Image(path) | Value::Video(path) => (
            if matches!(ctx.value, Value::Video(_)) { "Recording saved" } else { "Capture saved" },
            path.display().to_string(),
            Some(path.display().to_string()),
        ),
        Value::Url(url) => ("Uploaded", url.clone(), Some(url.clone())),
        Value::Text(text) => (
            "Copied to clipboard",
            util::truncate_text(text, 140),
            None,
        ),
        Value::Empty => ("ss-belt", "done".to_string(), None),
    };

    if ctx.dry_run {
        ctx.note(format!("dry-run: would notify `{summary}: {body}`"));
        return Ok(());
    }
    if !ctx.config.general.notify {
        return Ok(());
    }

    match target {
        Some(target) => crate::notify_click::spawn_helper(&target, summary, &body),
        None => crate::adapters::misc::send(ctx, summary, &body),
    }
    Ok(())
}

pub fn open_support(probe: &Probe<'_>) -> Support {
    if probe.tools.has(ToolId::XdgOpen) {
        Support::Ready
    } else {
        Support::Unavailable("xdg-open is missing (install xdg-utils)".into())
    }
}

pub fn open_run(ctx: &mut Ctx) -> Result<()> {
    let target = match &ctx.value {
        Value::Image(path) | Value::Video(path) => path.display().to_string(),
        Value::Url(url) | Value::Text(url) => url.clone(),
        Value::Empty => return Err(user("nothing to open")),
    };
    if ctx.dry_run {
        ctx.note(format!("dry-run: would open {target}"));
        return Ok(());
    }
    // xdg-open can block for the lifetime of the application it launches.
    let _ = Cmd::new(ctx.tools.program(ToolId::XdgOpen))
        .arg(&target)
        .timeout(Duration::from_secs(2))
        .quiet()
        .run();
    ctx.note(format!("opened {target}"));
    Ok(())
}

pub fn hash_support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

pub fn hash_run(ctx: &mut Ctx) -> Result<()> {
    let digest = match &ctx.value {
        Value::Image(path) | Value::Video(path) => {
            let bytes = fs::read(path)?;
            hex::encode(Sha256::digest(&bytes))
        }
        Value::Text(text) | Value::Url(text) => hex::encode(Sha256::digest(text.as_bytes())),
        Value::Empty => return Err(user("nothing to hash")),
    };
    ctx.note(format!("sha256 {digest}"));
    ctx.value = Value::Text(digest);
    Ok(())
}

pub fn last_support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

pub fn last_run(ctx: &mut Ctx) -> Result<()> {
    let record = fs::read_to_string(ctx.paths.last_file()).map_err(|_| {
        user("there is no previous capture yet")
            .hint("take a capture first, or point `ss-belt upload <file>` at one")
    })?;
    let path = PathBuf::from(record.trim());
    if !path.exists() {
        return Err(user(format!("the last capture {} no longer exists", path.display())));
    }
    ctx.value = if util::is_video_ext(&path) {
        Value::Video(path)
    } else {
        Value::Image(path)
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, b"abc").unwrap();
        let mut ctx = crate::adapters::testing::ctx_for("hash", dir.path());
        ctx.value = Value::Image(file);
        hash_run(&mut ctx).unwrap();
        assert_eq!(
            ctx.value,
            Value::Text("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into())
        );
    }
}
