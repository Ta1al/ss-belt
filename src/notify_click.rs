//! Clickable notifications.
//!
//! `notify-send` can attach a `default` action that the desktop invokes when the
//! user clicks the notification body; the action name is printed on stdout when
//! it fires. Because the notification can outlive the command that created it,
//! the wait happens in a detached `ss-belt __notify-click` process.
//!
//! Two actions exist: open the capture/URL, or copy a command (the install
//! command for a missing package) to the clipboard.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use crate::adapters::clipboard;
use crate::error::user;
use crate::exec::Cmd;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Open(String),
    Copy(String),
}

impl Action {
    pub fn parse(kind: &str, payload: &str) -> Option<Self> {
        match kind {
            "open" => Some(Action::Open(payload.to_string())),
            "copy" => Some(Action::Copy(payload.to_string())),
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Action::Open(_) => "open",
            Action::Copy(_) => "copy",
        }
    }

    pub fn payload(&self) -> &str {
        match self {
            Action::Open(target) | Action::Copy(target) => target,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Action::Open(_) => "Open",
            Action::Copy(_) => "Copy",
        }
    }
}

/// Run the notification, wait for a click, then perform the action.
pub fn run(kind: &str, payload: &str, summary: &str, body: &str) -> Result<()> {
    let action = Action::parse(kind, payload)
        .ok_or_else(|| user(format!("unknown notification action `{kind}`")))?;
    let Some(notify) = crate::exec::which("notify-send") else {
        return Ok(());
    };
    execute(
        &notify,
        crate::exec::which("xdg-open").as_deref(),
        clipboard::which_writer(),
        action,
        summary,
        body,
    )
}

fn execute(
    notify: &Path,
    opener: Option<&Path>,
    writer: Option<clipboard::Writer>,
    action: Action,
    summary: &str,
    body: &str,
) -> Result<()> {
    let output = Cmd::new(notify.display().to_string())
        .args(["-a", crate::APP_NAME, "-t", "8000", "-p", "-A"])
        .arg(format!("default={}", action.label()))
        .arg("--")
        .arg(summary)
        .arg(body)
        .timeout(Duration::from_secs(3600))
        .run()?;

    // `-p` prints the notification id; `-A` prints the action name when clicked.
    let mut id = None;
    let mut activated = false;
    for line in output.stdout_str().lines() {
        let line = line.trim();
        if let Ok(value) = line.parse::<u32>() {
            id = Some(value);
        } else if line == "default" {
            activated = true;
        }
    }
    if !activated {
        return Ok(());
    }

    match action {
        Action::Open(target) => {
            if let Some(opener) = opener {
                let _ = Cmd::new(opener.display().to_string())
                    .arg(target)
                    .timeout(Duration::from_secs(2))
                    .quiet()
                    .run();
            }
        }
        Action::Copy(text) => {
            if let Some(writer) = writer {
                let _ = clipboard::write_text_with(&writer, &text);
            }
            // Replace the bubble instead of stacking a second one.
            if let Some(id) = id {
                let _ = Cmd::new(notify.display().to_string())
                    .args(["-a", crate::APP_NAME, "-t", "4000", "-r"])
                    .arg(id.to_string())
                    .arg("Copied to clipboard")
                    .arg(&text)
                    .timeout(Duration::from_secs(5))
                    .quiet()
                    .run();
            }
        }
    }
    Ok(())
}

/// Spawn the detached helper that owns the notification. Returns immediately:
/// the helper waits for the click even after ss-belt itself has exited.
pub fn spawn_helper(action: Action, summary: &str, body: &str) {
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = std::process::Command::new(exe)
        .arg("__notify-click")
        .arg(action.kind())
        .arg(action.payload())
        .arg(summary)
        .arg(body)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn open_action_opens_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let notify_log = dir.path().join("notify.log");
        let notify = script(
            dir.path(),
            "notify-send",
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\necho 1\necho default\n", notify_log.display()),
        );
        let opened = dir.path().join("opened.log");
        let opener = script(
            dir.path(),
            "xdg-open",
            &format!("#!/bin/sh\nprintf '%s' \"$1\" > {}\n", opened.display()),
        );
        execute(&notify, Some(&opener), None, Action::Open("/tmp/shot.png".into()), "Saved", "shot")
            .unwrap();

        let args = fs::read_to_string(dir.path().join("notify.log")).unwrap();
        assert!(args.contains("default=Open"), "{args}");
        assert_eq!(fs::read_to_string(&opened).unwrap(), "/tmp/shot.png");
    }

    #[test]
    fn copy_action_writes_the_clipboard_and_replaces_the_bubble() {
        let dir = tempfile::tempdir().unwrap();
        let notify_log = dir.path().join("notify.log");
        let notify = script(
            dir.path(),
            "notify-send",
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\necho 7\necho default\n", notify_log.display()),
        );
        let clipboard_file = dir.path().join("clipboard.txt");
        let writer = clipboard::Writer {
            program: script(
                dir.path(),
                "wl-copy",
                &format!("#!/bin/sh\ncat > {}\n", clipboard_file.display()),
            ),
            wayland: true,
        };

        execute(
            &notify,
            None,
            Some(writer),
            Action::Copy("sudo dnf install grim".into()),
            "Missing package",
            "Click to copy",
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(&clipboard_file).unwrap(),
            "sudo dnf install grim"
        );
        let args = fs::read_to_string(dir.path().join("notify.log")).unwrap();
        assert!(args.contains("default=Copy"), "{args}");
        // The second call replaces notification 7 rather than adding another.
        assert!(args.contains("-r 7"), "{args}");
    }

    #[test]
    fn no_click_means_no_side_effect() {
        let dir = tempfile::tempdir().unwrap();
        let notify = script(dir.path(), "notify-send", "#!/bin/sh\necho 3\nexit 0\n");
        let opened = dir.path().join("opened.log");
        let opener = script(
            dir.path(),
            "xdg-open",
            &format!("#!/bin/sh\nprintf '%s' \"$1\" > {}\n", opened.display()),
        );

        execute(&notify, Some(&opener), None, Action::Open("/tmp/x".into()), "S", "B").unwrap();
        assert!(!opened.exists());
    }

    #[test]
    fn actions_round_trip_through_cli_arguments() {
        let action = Action::parse("copy", "sudo dnf install grim").unwrap();
        assert_eq!(action.kind(), "copy");
        assert_eq!(action.payload(), "sudo dnf install grim");
        assert_eq!(action.label(), "Copy");
        assert!(Action::parse("nope", "x").is_none());
    }
}
