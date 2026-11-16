use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::user;
use crate::paths::Paths;
use crate::error::HintExt;

pub const DEFAULT_CONFIG: &str = r#"# ss-belt configuration
# Every key is optional: delete a line to get the documented default back.
# Reload the daemon with `ss-belt daemon --reload` after editing.

[general]
# Where captures are saved. `~` is expanded.
save_dir = "~/Pictures/ss-belt"
# Tokens: {action} {date} {time} {datetime} {unix} {ext}
filename_template = "{action}_{datetime}"
# Keep a searchable history of every step in a local SQLite database.
history = true
history_limit = 1000
# Desktop notifications for captures, uploads and errors.
notify = true
# Copy the result to the clipboard as well as saving it.
copy_after_save = true
# Open the saved file in the default application.
open_after_save = false
# 0 disables pruning by age.
retain_days = 0
# auto | satty | swappy | none
annotator = "auto"

[capture]
# auto | grim | spectacle | gnome | x11
backend = "auto"
# auto | slurp | spectacle
region_tool = "auto"
# Draw the mouse cursor into the capture when the backend supports it.
cursor = false
# Wait before capturing, in milliseconds.
delay_ms = 0

[clipboard]
# auto | wl | x11
backend = "auto"

[record]
# auto | wl-screenrec | wf-recorder | ffmpeg
backend = "auto"
fps = 30
# GIF conversion settings.
gif_fps = 15
gif_width = 960
# 0 means record until the hotkey is pressed again.
duration_secs = 0
audio = false

[upload]
# Name of the destination used by `upload` steps and the upload action.
default_destination = ""
# Remove EXIF/metadata before uploading.
strip_metadata = true
timeout_secs = 60

# Example destination: drop a ShareX .sxcu file in ~/.config/ss-belt/uploaders/
# and reference it here.
#
# [upload.destinations.catbox]
# type = "sxcu"
# file = "~/.config/ss-belt/uploaders/catbox.sxcu"
#
# A pre-signed URL destination (S3/R2/GCS presigned PUT) needs no signing code:
#
# [upload.destinations.r2]
# type = "presigned"
# url = "https://example.r2.cloudflarestorage.com/bucket/{filename}?X-Amz-..."
# method = "PUT"

[hotkeys]
# Used for the X11 grabber and printed by `ss-belt hotkeys --print <desktop>`.
region = "Mod4+Shift+4"
full = "Mod4+Shift+3"
window = "Mod4+Shift+5"
monitor = "Mod4+Shift+6"
annotate = "Mod4+Shift+A"
clipboard_edit = "Mod4+Shift+E"
gif = "Mod4+Shift+G"
video = "Mod4+Shift+V"
qr_scan = "Mod4+Shift+Q"
qr_make = "Mod4+Shift+M"
ocr = "Mod4+Shift+O"

# Custom pipelines. Each step name is listed by `ss-belt steps`.
# Uncomment to replace a built-in action:
#
# [actions.share]
# steps = ["capture.region", "annotate", "upload", "copy.url", "notify"]
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub capture: Capture,
    pub clipboard: Clipboard,
    pub record: Record,
    pub upload: Upload,
    #[serde(default = "default_hotkeys")]
    pub hotkeys: BTreeMap<String, String>,
    pub actions: BTreeMap<String, ActionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    pub save_dir: String,
    pub filename_template: String,
    pub history: bool,
    pub history_limit: usize,
    pub notify: bool,
    pub copy_after_save: bool,
    pub open_after_save: bool,
    pub retain_days: u32,
    pub annotator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Capture {
    pub backend: String,
    pub region_tool: String,
    pub cursor: bool,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clipboard {
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Record {
    pub backend: String,
    pub fps: u32,
    pub gif_fps: u32,
    pub gif_width: u32,
    pub duration_secs: u64,
    pub audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Upload {
    pub default_destination: String,
    pub strip_metadata: bool,
    pub timeout_secs: u64,
    pub destinations: BTreeMap<String, Destination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Destination {
    Sxcu { file: String },
    Presigned { url: String, #[serde(default = "default_method")] method: String },
}

fn default_method() -> String {
    "PUT".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionConfig {
    pub steps: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            capture: Capture::default(),
            clipboard: Clipboard::default(),
            record: Record::default(),
            upload: Upload::default(),
            hotkeys: default_hotkeys(),
            actions: BTreeMap::new(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/ss-belt".into(),
            filename_template: "{action}_{datetime}".into(),
            history: true,
            history_limit: 1000,
            notify: true,
            copy_after_save: true,
            open_after_save: false,
            retain_days: 0,
            annotator: "auto".into(),
        }
    }
}

impl Default for Capture {
    fn default() -> Self {
        Self { backend: "auto".into(), region_tool: "auto".into(), cursor: false, delay_ms: 0 }
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self { backend: "auto".into() }
    }
}

impl Default for Record {
    fn default() -> Self {
        Self { backend: "auto".into(), fps: 30, gif_fps: 15, gif_width: 960, duration_secs: 0, audio: false }
    }
}

impl Default for Upload {
    fn default() -> Self {
        Self {
            default_destination: String::new(),
            strip_metadata: true,
            timeout_secs: 60,
            destinations: BTreeMap::new(),
        }
    }
}

pub const HOTKEY_ACTIONS: [&str; 11] = [
    "region",
    "full",
    "window",
    "monitor",
    "annotate",
    "clipboard_edit",
    "gif",
    "video",
    "qr_scan",
    "qr_make",
    "ocr",
];

fn default_hotkeys() -> BTreeMap<String, String> {
    [
        ("region", "Mod4+Shift+4"),
        ("full", "Mod4+Shift+3"),
        ("window", "Mod4+Shift+5"),
        ("monitor", "Mod4+Shift+6"),
        ("annotate", "Mod4+Shift+A"),
        ("clipboard_edit", "Mod4+Shift+E"),
        ("gif", "Mod4+Shift+G"),
        ("video", "Mod4+Shift+V"),
        ("qr_scan", "Mod4+Shift+Q"),
        ("qr_make", "Mod4+Shift+M"),
        ("ocr", "Mod4+Shift+O"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

impl Config {
    /// Load the config, writing a documented default on first run.
    pub fn load(paths: &Paths) -> Result<Self> {
        let path = paths.config_file();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
            }
            fs::write(&path, DEFAULT_CONFIG)
                .with_context(|| format!("writing default config to {}", path.display()))?;
        }
        Self::from_path(&path)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let config: Config = toml::from_str(&text).map_err(|err| {
            user(format!("invalid config {}:\n{err}", path.display()))
                .hint("compare with `ss-belt config --default`")
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.general.filename_template.trim().is_empty() {
            return Err(user("general.filename_template must not be empty"));
        }
        if self.general.save_dir.trim().is_empty() {
            return Err(user("general.save_dir must not be empty"));
        }
        if self.record.fps == 0 || self.record.gif_fps == 0 || self.record.gif_width < 16 {
            return Err(user("record.fps, record.gif_fps and record.gif_width must be positive"));
        }
        if self.upload.timeout_secs == 0 {
            return Err(user("upload.timeout_secs must be at least 1"));
        }

        for (name, action) in &self.actions {
            if action.steps.is_empty() {
                return Err(user(format!("actions.{name} has no steps")));
            }
            for step in &action.steps {
                if crate::steps::find(step).is_none() {
                    return Err(user(format!(
                        "actions.{name} references unknown step `{step}`"
                    ))
                    .hint("run `ss-belt steps` for the list"));
                }
            }
        }

        for (name, destination) in &self.upload.destinations {
            match destination {
                Destination::Sxcu { file } => {
                    if file.trim().is_empty() {
                        return Err(user(format!("upload.destinations.{name}.file is empty")));
                    }
                }
                Destination::Presigned { url, method } => {
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        return Err(user(format!(
                            "upload.destinations.{name}.url must start with http:// or https://"
                        )));
                    }
                    if !matches!(method.to_ascii_uppercase().as_str(), "PUT" | "POST") {
                        return Err(user(format!(
                            "upload.destinations.{name}.method must be PUT or POST"
                        )));
                    }
                }
            }
        }

        if !self.upload.default_destination.is_empty()
            && !self.upload.destinations.contains_key(&self.upload.default_destination)
        {
            return Err(user(format!(
                "upload.default_destination `{}` is not defined",
                self.upload.default_destination
            )));
        }

        for (action, accelerator) in &self.hotkeys {
            crate::hotkeys::parse_accelerator(accelerator).map_err(|err| {
                user(format!("hotkeys.{action}: {err}"))
            })?;
        }

        Ok(())
    }

    pub fn save_dir(&self) -> std::path::PathBuf {
        crate::util::expand_tilde(&self.general.save_dir)
    }

    pub fn destination(&self, name: Option<&str>) -> Result<(&String, &Destination)> {
        let name = name
            .filter(|n| !n.is_empty())
            .unwrap_or(self.upload.default_destination.as_str());
        if name.is_empty() {
            return Err(user("no upload destination configured").hint(
                "set upload.default_destination and define it under [upload.destinations]",
            ));
        }
        self.upload
            .destinations
            .get_key_value(name)
            .ok_or_else(|| user(format!("unknown upload destination `{name}`")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        config.validate().unwrap();
        assert_eq!(config.general.filename_template, "{action}_{datetime}");
        assert!(config.hotkeys.contains_key("region"));
    }

    #[test]
    fn documented_default_matches_the_typed_default() {
        let parsed: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        let typed = Config::default();
        assert_eq!(parsed.general.save_dir, typed.general.save_dir);
        assert_eq!(parsed.hotkeys, typed.hotkeys);
        assert_eq!(parsed.capture.backend, typed.capture.backend);
    }

    #[test]
    fn unknown_key_is_rejected_with_a_useful_message() {
        let err = toml::from_str::<Config>("[general]\nsave_dirr = \"x\"\n").unwrap_err();
        assert!(err.to_string().contains("save_dirr"));
    }

    #[test]
    fn bad_upload_destination_is_caught() {
        let mut config = Config::default();
        config.upload.default_destination = "nope".into();
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("not defined"));
    }

    #[test]
    fn unknown_step_in_custom_action_is_caught() {
        let mut config = Config::default();
        config
            .actions
            .insert("x".into(), ActionConfig { steps: vec!["capture.teleport".into()] });
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("unknown step"));
    }

    #[test]
    fn default_hotkeys_all_parse() {
        for (name, accel) in default_hotkeys() {
            crate::hotkeys::parse_accelerator(&accel)
                .unwrap_or_else(|err| panic!("{name}: {err}"));
        }
    }
}
