use anyhow::Result;
use std::time::Duration;

use crate::pipeline::{self, Pipeline, Report};
use crate::steps::{Ctx, Kind};

/// Built-in pipelines. Users can override any of them in `[actions.<name>]`.
pub static DEFAULT_ACTIONS: &[(&str, &[&str])] = &[
    ("region", &["capture.region", "save", "clipboard.copy", "notify"]),
    ("full", &["capture.full", "save", "clipboard.copy", "notify"]),
    ("window", &["capture.window", "save", "clipboard.copy", "notify"]),
    ("monitor", &["capture.monitor", "save", "clipboard.copy", "notify"]),
    ("annotate", &["capture.region", "annotate", "save", "clipboard.copy", "notify"]),
    ("clipboard_edit", &["clipboard.paste", "annotate", "save", "clipboard.copy", "notify"]),
    ("qr_scan", &["capture.region", "qr.scan", "clipboard.copy", "notify"]),
    ("qr_make", &["clipboard.paste", "qr.make", "save", "clipboard.copy", "notify"]),
    ("ocr", &["capture.region", "ocr", "clipboard.copy", "notify"]),
    ("upload", &["last.restore", "upload", "clipboard.copy", "notify"]),
    ("annotate_last", &["last.restore", "annotate", "save", "clipboard.copy", "notify"]),
];

/// Actions that toggle a background recording instead of running a pipeline.
pub static RECORD_ACTIONS: &[(&str, &str)] = &[
    ("gif", "record a GIF (press again to stop)"),
    ("video", "record a video (press again to stop)"),
];

const GIF_FINISH: &[&str] = &["gif.convert", "save", "clipboard.copy", "notify"];
const VIDEO_FINISH: &[&str] = &["save", "clipboard.copy", "notify"];

pub fn describe_action(name: &str) -> &'static str {
    for (candidate, description) in ACTIONS {
        if *candidate == name {
            return description;
        }
    }
    "custom action"
}

pub static ACTIONS: &[(&str, &str)] = &[
    ("region", "capture a region, save it and copy it"),
    ("full", "capture every monitor, save and copy"),
    ("window", "capture the active window, save and copy"),
    ("monitor", "capture the current monitor, save and copy"),
    ("annotate", "region capture, then annotate before saving"),
    ("clipboard_edit", "edit the image currently on the clipboard"),
    ("qr_scan", "decode a QR code from a region and copy the result"),
    ("qr_make", "turn text (or the clipboard) into a QR image"),
    ("ocr", "extract text from a region and copy it"),
    ("upload", "upload the last capture and copy the link"),
    ("annotate_last", "annotate the last capture again"),
    ("gif", "record a GIF (press again to stop)"),
    ("video", "record a video (press again to stop)"),
];

pub fn is_record_action(name: &str) -> bool {
    RECORD_ACTIONS.iter().any(|(candidate, _)| *candidate == name)
}

pub struct Execution {
    pub report: Option<Report>,
    pub message: String,
    pub recording_started: bool,
}

pub fn execute(ctx: &mut Ctx, name: &str, duration: Option<u64>) -> Result<Execution> {
    if is_record_action(name) {
        return execute_record(ctx, name, duration);
    }
    let pipeline = Pipeline::resolve(&ctx.config, name)?;
    let report = pipeline::run(ctx, &pipeline)?;
    Ok(Execution { report: Some(report), message: String::new(), recording_started: false })
}

fn execute_record(ctx: &mut Ctx, name: &str, duration: Option<u64>) -> Result<Execution> {
    let recording = crate::adapters::record::is_recording(&ctx.paths);

    if recording {
        let stop = Pipeline { name: name.to_string(), steps: vec!["record.stop".into()], custom: false, start: Kind::None };
        let report = pipeline::run(ctx, &stop)?;
        let finish_steps = if name == "gif" { GIF_FINISH } else { VIDEO_FINISH };
        let finish = Pipeline {
            name: name.to_string(),
            steps: finish_steps.iter().map(|s| (*s).to_string()).collect(),
            custom: false,
            start: Kind::Video,
        };
        let finish_report = pipeline::run(ctx, &finish)?;
        return Ok(Execution {
            message: format!("{} finished in {}ms", name, report.ms + finish_report.ms),
            report: Some(finish_report),
            recording_started: false,
        });
    }

    let start = Pipeline { name: name.to_string(), steps: vec!["record.start".into()], custom: false, start: Kind::None };
    pipeline::run(ctx, &start)?;

    let seconds = duration.unwrap_or(ctx.config.record.duration_secs);
    if seconds == 0 {
        return Ok(Execution {
            message: if name == "gif" {
                "recording; run the action again to stop and build the GIF".to_string()
            } else {
                "recording; run the action again to stop".to_string()
            },
            report: None,
            recording_started: true,
        });
    }

    std::thread::sleep(Duration::from_secs(seconds));

    let stop = Pipeline { name: name.to_string(), steps: vec!["record.stop".into()], custom: false, start: Kind::None };
    pipeline::run(ctx, &stop)?;
    let finish_steps = if name == "gif" { GIF_FINISH } else { VIDEO_FINISH };
    let finish = Pipeline {
        name: name.to_string(),
        steps: finish_steps.iter().map(|s| (*s).to_string()).collect(),
        custom: false,
        start: Kind::Video,
    };
    let report = pipeline::run(ctx, &finish)?;
    Ok(Execution {
        message: format!("{} recorded for {seconds}s", name),
        report: Some(report),
        recording_started: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn record_finish_chains_are_valid() {
        for steps in [GIF_FINISH, VIDEO_FINISH] {
            let chain: Vec<String> = steps.iter().map(|s| (*s).to_string()).collect();
            crate::steps::validate_chain_from(&chain, Kind::Video).unwrap();
        }
    }

    #[test]
    fn every_listed_action_resolves_or_is_a_record_action() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = testing::ctx_for("region", dir.path());
        for (name, _) in ACTIONS {
            if is_record_action(name) {
                continue;
            }
            let pipeline = Pipeline::resolve(&ctx.config, name)
                .unwrap_or_else(|err| panic!("{name}: {err}"));
            pipeline.validate().unwrap_or_else(|err| panic!("{name}: {err}"));
        }
    }

    #[test]
    fn every_action_has_a_description() {
        for (name, _) in ACTIONS {
            assert_ne!(describe_action(name), "custom action", "{name}");
        }
    }

    #[test]
    fn dry_run_capture_pipeline_succeeds_without_tools() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("region", dir.path());
        ctx.dry_run = true;
        let execution = execute(&mut ctx, "region", None).unwrap();
        assert!(!execution.recording_started);
        assert_eq!(execution.report.unwrap().action, "region");
    }
}
