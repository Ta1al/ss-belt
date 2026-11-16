use anyhow::Result;
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};

use crate::capabilities::{Desktop, ToolSet};
use crate::config::Config;
use crate::error::user;
use crate::paths::Paths;
use crate::util;
use crate::error::HintExt;

/// The artifact flowing through a pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Empty,
    Image(PathBuf),
    Video(PathBuf),
    Text(String),
    Url(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    None,
    Image,
    Video,
    Text,
    Url,
    /// Only known at run time (for example the clipboard, which can hold an
    /// image or text). Validation is permissive; the runtime step enforces.
    Any,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::None => "none",
            Kind::Image => "image",
            Kind::Video => "video",
            Kind::Text => "text",
            Kind::Url => "url",
            Kind::Any => "any",
        }
    }

    /// Can a step that needs `self` consume a value of kind `produced`?
    pub fn accepts(self, produced: Kind) -> bool {
        if self == Kind::Any || produced == Kind::Any {
            return true;
        }
        if self == Kind::None {
            return true;
        }
        // Text is a valid Url producer and vice versa only through explicit steps.
        self == produced
    }
}

impl Value {
    pub fn kind(&self) -> Kind {
        match self {
            Value::Empty => Kind::None,
            Value::Image(_) => Kind::Image,
            Value::Video(_) => Kind::Video,
            Value::Text(_) => Kind::Text,
            Value::Url(_) => Kind::Url,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Value::Image(path) | Value::Video(path) => Some(path),
            _ => None,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Value::Empty => "nothing".to_string(),
            Value::Image(path) => format!("image {}", path.display()),
            Value::Video(path) => format!("video {}", path.display()),
            Value::Text(text) => format!("text ({} chars)", text.chars().count()),
            Value::Url(url) => format!("url {url}"),
        }
    }
}

pub struct Ctx {
    pub paths: Paths,
    pub config: Config,
    pub tools: ToolSet,
    pub desktop: Desktop,
    pub action: String,
    pub value: Value,
    pub run_dir: PathBuf,
    pub dry_run: bool,
    pub keep_run_dir: bool,
    pub upload_destination: Option<String>,
    pub started: DateTime<Local>,
    pub log: Vec<String>,
}

impl Ctx {
    pub fn new(
        paths: Paths,
        config: Config,
        tools: ToolSet,
        desktop: Desktop,
        action: impl Into<String>,
    ) -> Result<Self> {
        let action = action.into();
        let started = Local::now();
        let run_id = format!("{}-{}", started.format("%Y%m%d-%H%M%S"), std::process::id());
        let run_dir = paths.run_dir(&run_id);
        std::fs::create_dir_all(&run_dir)?;
        Ok(Self {
            paths,
            config,
            tools,
            desktop,
            action,
            value: Value::Empty,
            run_dir,
            dry_run: false,
            keep_run_dir: false,
            upload_destination: None,
            started,
            log: Vec::new(),
        })
    }

    pub fn note(&mut self, message: impl Into<String>) {
        let message = message.into();
        // In a dry run the notes are the product: print them. Otherwise they are
        // diagnostics, available with `-v`.
        if self.dry_run {
            eprintln!("{message}");
        } else {
            tracing::debug!("{}", message);
        }
        self.log.push(message);
    }

    pub fn sleep_delay(&self) {
        let delay = self.config.capture.delay_ms;
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
    }

    pub fn require_image(&self, step: &str) -> Result<PathBuf> {
        match &self.value {
            Value::Image(path) => Ok(path.clone()),
            other => Err(user(format!(
                "step `{step}` needs an image but received {}",
                other.describe()
            ))),
        }
    }

    pub fn require_video(&self, step: &str) -> Result<PathBuf> {
        match &self.value {
            Value::Video(path) => Ok(path.clone()),
            other => Err(user(format!(
                "step `{step}` needs a video but received {}",
                other.describe()
            ))),
        }
    }

    pub fn require_text(&self, step: &str) -> Result<String> {
        match &self.value {
            Value::Text(text) => Ok(text.clone()),
            Value::Url(url) => Ok(url.clone()),
            other => Err(user(format!(
                "step `{step}` needs text but received {}",
                other.describe()
            ))),
        }
    }

    pub fn require_path(&self, step: &str) -> Result<PathBuf> {
        self.value
            .path()
            .map(Path::to_path_buf)
            .ok_or_else(|| user(format!("step `{step}` needs a file but received {}", self.value.describe())))
    }

    /// A staging path inside this run's directory.
    pub fn stage(&self, name: &str) -> PathBuf {
        self.run_dir.join(name)
    }

    /// Where a new capture should be written for this run.
    pub fn output_path(&self, ext: &str) -> PathBuf {
        let name = util::expand_template(
            &self.config.general.filename_template,
            &self.action.replace('.', "-"),
            ext,
        );
        let name = util::sanitize_component(&name);
        util::unique_path(&self.config.save_dir().join(format!("{name}.{ext}")))
    }

    pub fn notify(&self, summary: &str, body: &str) {
        if !self.config.general.notify {
            return;
        }
        crate::adapters::misc::send(self, summary, body);
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Support {
    Ready,
    /// Works, with a documented limitation.
    Degraded(String),
    /// Cannot work here; the detail explains why and how to fix it.
    Unavailable(String),
}

impl Support {
    pub fn ready(&self) -> bool {
        matches!(self, Support::Ready | Support::Degraded(_))
    }

    pub fn label(&self) -> &'static str {
        match self {
            Support::Ready => "ready",
            Support::Degraded(_) => "degraded",
            Support::Unavailable(_) => "unavailable",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Support::Ready => None,
            Support::Degraded(detail) | Support::Unavailable(detail) => Some(detail),
        }
    }
}

/// What a step needs from the host, resolved without running anything.
pub struct Probe<'a> {
    pub tools: &'a ToolSet,
    pub desktop: &'a Desktop,
    pub config: &'a Config,
}

pub type StepFn = fn(&mut Ctx) -> Result<()>;
pub type SupportFn = fn(&Probe<'_>) -> Support;

pub struct StepSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub input: Kind,
    pub output: Kind,
    pub support: Option<SupportFn>,
    pub run: StepFn,
}

impl std::fmt::Debug for StepSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepSpec").field("name", &self.name).finish()
    }
}

impl StepSpec {
    pub fn support(&self, probe: &Probe<'_>) -> Support {
        match self.support {
            Some(check) => check(probe),
            None => Support::Ready,
        }
    }
}

pub fn registry() -> &'static [StepSpec] {
    crate::adapters::STEPS
}

pub fn find(name: &str) -> Option<&'static StepSpec> {
    registry().iter().find(|spec| spec.name == name)
}

/// Validate that a list of step names forms a legal chain.
pub fn validate_chain(steps: &[String]) -> Result<Vec<&'static StepSpec>> {
    validate_chain_from(steps, Kind::None)
}

/// Validate a chain that starts with an artifact already in hand, which is how
/// the second half of a recording pipeline works.
pub fn validate_chain_from(steps: &[String], start: Kind) -> Result<Vec<&'static StepSpec>> {
    let mut specs = Vec::with_capacity(steps.len());
    let mut kind = start;
    for name in steps {
        let spec = find(name).ok_or_else(|| {
            user(format!("unknown step `{name}`")).hint("run `ss-belt steps` for the list")
        })?;
        if !spec.input.accepts(kind) {
            return Err(user(format!(
                "step `{name}` needs {} but the pipeline has {} at that point",
                spec.input.as_str(),
                kind.as_str()
            )));
        }
        kind = spec.output;
        specs.push(spec);
    }
    Ok(specs)
}

