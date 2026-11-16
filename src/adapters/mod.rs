//! Adapters: every external program is probed, never assumed.

pub mod annotate;
pub mod capture;
pub mod clipboard;
pub mod misc;
pub mod ocr;
pub mod qr;
pub mod record;
pub mod save;
pub mod upload;

use crate::steps::{Kind, StepSpec};

pub static STEPS: &[StepSpec] = &[
    StepSpec {
        name: "capture.region",
        summary: "select a region of the screen and capture it",
        input: Kind::None,
        output: Kind::Image,
        support: Some(capture::region_support),
        run: capture::region_run,
    },
    StepSpec {
        name: "capture.full",
        summary: "capture every monitor",
        input: Kind::None,
        output: Kind::Image,
        support: Some(capture::full_support),
        run: capture::full_run,
    },
    StepSpec {
        name: "capture.window",
        summary: "capture the active window",
        input: Kind::None,
        output: Kind::Image,
        support: Some(capture::window_support),
        run: capture::window_run,
    },
    StepSpec {
        name: "capture.monitor",
        summary: "capture the current monitor",
        input: Kind::None,
        output: Kind::Image,
        support: Some(capture::monitor_support),
        run: capture::monitor_run,
    },
    StepSpec {
        name: "clipboard.paste",
        summary: "read an image or text from the clipboard",
        input: Kind::None,
        output: Kind::Any,
        support: Some(clipboard::support),
        run: clipboard::paste_run,
    },
    StepSpec {
        name: "annotate",
        summary: "open the capture in satty or swappy",
        input: Kind::Image,
        output: Kind::Image,
        support: Some(annotate::support),
        run: annotate::run,
    },
    StepSpec {
        name: "qr.scan",
        summary: "decode QR codes from an image (built in)",
        input: Kind::Image,
        output: Kind::Text,
        support: Some(qr::support),
        run: qr::scan_run,
    },
    StepSpec {
        name: "ocr",
        summary: "extract text with tesseract",
        input: Kind::Image,
        output: Kind::Text,
        support: Some(ocr::support),
        run: ocr::run,
    },
    StepSpec {
        name: "qr.make",
        summary: "render text as a QR image (built in)",
        input: Kind::Text,
        output: Kind::Image,
        support: Some(qr::support),
        run: qr::make_run,
    },
    StepSpec {
        name: "clipboard.copy",
        summary: "copy the current value to the clipboard",
        input: Kind::Any,
        output: Kind::Any,
        support: Some(clipboard::support),
        run: clipboard::copy_run,
    },
    StepSpec {
        name: "save",
        summary: "write the artifact into the save directory",
        input: Kind::Any,
        output: Kind::Any,
        support: Some(save::support),
        run: save::run,
    },
    StepSpec {
        name: "upload",
        summary: "send the artifact to the configured destination",
        input: Kind::Any,
        output: Kind::Url,
        support: Some(upload::support),
        run: upload::run,
    },
    StepSpec {
        name: "gif.convert",
        summary: "convert a recording into a GIF",
        input: Kind::Video,
        output: Kind::Video,
        support: Some(record::gif_support),
        run: record::gif_run,
    },
    StepSpec {
        name: "record.start",
        summary: "start a screen recording in the background",
        input: Kind::None,
        output: Kind::Video,
        support: Some(record::start_support),
        run: record::start_run,
    },
    StepSpec {
        name: "record.stop",
        summary: "stop the running recording",
        input: Kind::None,
        output: Kind::Video,
        support: Some(record::stop_support),
        run: record::stop_run,
    },
    StepSpec {
        name: "last.restore",
        summary: "reuse the most recent capture",
        input: Kind::None,
        output: Kind::Any,
        support: Some(misc::last_support),
        run: misc::last_run,
    },
    StepSpec {
        name: "open",
        summary: "open the artifact with the default application",
        input: Kind::Any,
        output: Kind::Any,
        support: Some(misc::open_support),
        run: misc::open_run,
    },
    StepSpec {
        name: "hash",
        summary: "replace the value with its SHA-256",
        input: Kind::Any,
        output: Kind::Text,
        support: Some(misc::hash_support),
        run: misc::hash_run,
    },
    StepSpec {
        name: "notify",
        summary: "show a desktop notification",
        input: Kind::Any,
        output: Kind::Any,
        support: Some(misc::notify_support),
        run: misc::notify_run,
    },
];

/// Helpers shared by unit and integration tests. Not part of the public API.
#[doc(hidden)]
pub mod testing {
    use std::path::{Path, PathBuf};

    use crate::capabilities::{Desktop, SessionType, ToolId, ToolSet};
    use crate::config::Config;
    use crate::paths::Paths;
    use crate::steps::Ctx;

    pub fn paths(root: &Path) -> Paths {
        let paths = Paths {
            root: Some(root.to_path_buf()),
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            runtime_dir: root.join("run"),
        };
        paths.ensure().unwrap();
        paths
    }

    pub fn tools(ids: &[ToolId]) -> ToolSet {
        ToolSet::from_paths(ids.iter().map(|id| (*id, PathBuf::from("/bin/true"))))
    }

    pub fn wayland() -> Desktop {
        Desktop {
            session: SessionType::Wayland,
            desktop: "sway".into(),
            is_wlroots: true,
            has_xwayland: false,
        }
    }

    pub fn x11() -> Desktop {
        Desktop {
            session: SessionType::X11,
            desktop: "xfce".into(),
            is_wlroots: false,
            has_xwayland: true,
        }
    }

    pub fn ctx_for(action: &str, root: &Path) -> Ctx {
        let paths = paths(root);
        let mut config = Config::default();
        config.general.save_dir = root.join("shots").display().to_string();
        config.general.notify = false;
        Ctx::new(paths, config, ToolSet::default(), x11(), action).unwrap()
    }
}
