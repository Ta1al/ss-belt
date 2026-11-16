use std::fmt;

/// Errors that are the user's to fix (bad config, missing tool, bad input).
/// Anything else is a bug and surfaces as a plain `anyhow` chain.
#[derive(Debug)]
pub struct UserError {
    pub message: String,
    pub hint: Option<String>,
}

impl UserError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), hint: None }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for UserError {}

pub fn user(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(UserError::new(message))
}

pub fn user_hint(message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(UserError::new(message).with_hint(hint))
}

/// True when the error chain contains a [`UserError`], i.e. it should be printed
/// without a backtrace and without being reported as an internal failure.
pub fn is_user_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause.downcast_ref::<UserError>().is_some() || cause.downcast_ref::<MissingTools>().is_some()
    })
}

/// The user dismissed a window (annotation, region selection). Not a failure:
/// the pipeline stops and ss-belt exits quietly.
#[derive(Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cancelled")
    }
}

impl std::error::Error for Cancelled {}

pub fn cancelled() -> anyhow::Error {
    anyhow::Error::new(Cancelled)
}

pub fn is_cancelled(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.downcast_ref::<Cancelled>().is_some())
}

/// `anyhow::Error::hint(...)` — attach a fix hint to a user-facing error while
/// keeping it distinguishable from an internal failure.
pub trait HintExt {
    fn hint(self, hint: impl Into<String>) -> anyhow::Error;
}

impl HintExt for anyhow::Error {
    fn hint(self, hint: impl Into<String>) -> anyhow::Error {
        match self.downcast::<UserError>() {
            Ok(err) => anyhow::Error::new(UserError { message: err.message, hint: Some(hint.into()) }),
            Err(err) => err.context(hint.into()),
        }
    }
}

/// A capability is unavailable because one or more packages are missing.
///
/// Carrying the tools (rather than a formatted string) is what lets the daemon
/// offer the install command as a click-to-copy action.
#[derive(Debug)]
pub struct MissingTools {
    pub what: String,
    pub tools: Vec<crate::capabilities::ToolId>,
}

impl MissingTools {
    pub fn install_command(&self) -> String {
        let distro = crate::capabilities::Distro::detect();
        let packages: Vec<&str> = self.tools.iter().map(|tool| tool.package_for(distro)).collect();
        distro.install_command(&packages)
    }
}

impl fmt::Display for MissingTools {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.tools.iter().map(|tool| tool.binary()).collect();
        write!(f, "{} needs {}", self.what, names.join(", "))?;
        write!(f, "\n  install with: {}", self.install_command())
    }
}

impl std::error::Error for MissingTools {}

pub fn missing_tools(what: impl Into<String>, tools: &[crate::capabilities::ToolId]) -> anyhow::Error {
    anyhow::Error::new(MissingTools { what: what.into(), tools: tools.to_vec() })
}

/// Find a [`MissingTools`] anywhere in an error chain.
pub fn missing_tools_in(err: &anyhow::Error) -> Option<&MissingTools> {
    err.chain().find_map(|cause| cause.downcast_ref::<MissingTools>())
}
