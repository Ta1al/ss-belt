//! Clickable notifications.
//!
//! `notify-send` can attach a `default` action that the desktop invokes when the
//! user clicks the notification body; the action name is printed on stdout when
//! it fires. Because the notification can outlive the command that created it,
//! the wait happens in a detached `ss-belt __notify-open` process.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use crate::exec::Cmd;

/// Run the notification, wait for a click, then open the target.
pub fn run(target: &str, summary: &str, body: &str) -> Result<()> {
    let Some(notify) = crate::exec::which("notify-send") else {
        return Ok(());
    };
    let opener = crate::exec::which("xdg-open");
    execute(&notify, opener.as_deref(), target, summary, body)
}

fn execute(
    notify: &Path,
    opener: Option<&Path>,
    target: &str,
    summary: &str,
    body: &str,
) -> Result<()> {
    let output = Cmd::new(notify.display().to_string())
        .args(["-a", crate::APP_NAME, "-t", "8000", "-A", "default=Open", "--"])
        .arg(summary)
        .arg(body)
        .timeout(Duration::from_secs(3600))
        .run()?;

    if output.stdout_str().lines().any(|line| line.trim() == "default") {
        if let Some(opener) = opener {
            let _ = Cmd::new(opener.display().to_string())
                .arg(target)
                .timeout(Duration::from_secs(2))
                .quiet()
                .run();
        }
    }
    Ok(())
}

/// Spawn the detached helper that owns the notification. Returns immediately:
/// the helper waits for the click even after ss-belt itself has exited.
pub fn spawn_helper(target: &str, summary: &str, body: &str) {
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = std::process::Command::new(exe)
        .arg("__notify-open")
        .arg(target)
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
    fn a_default_action_opens_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let notify = script(
            dir.path(),
            "notify-send",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" > {}\necho default\n",
                dir.path().join("notify.log").display()
            ),
        );
        let opened = dir.path().join("opened.log");
        let opener = script(
            dir.path(),
            "xdg-open",
            &format!("#!/bin/sh\nprintf '%s' \"$1\" > {}\n", opened.display()),
        );

        execute(&notify, Some(&opener), "/tmp/shot.png", "Saved", "shot.png").unwrap();

        let args = fs::read_to_string(dir.path().join("notify.log")).unwrap();
        assert!(args.contains("default=Open"), "{args}");
        assert!(args.contains("Saved"), "{args}");
        assert_eq!(fs::read_to_string(&opened).unwrap(), "/tmp/shot.png");
    }

    #[test]
    fn no_action_click_opens_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let notify = script(dir.path(), "notify-send", "#!/bin/sh\nexit 0\n");
        let opened = dir.path().join("opened.log");
        let opener = script(
            dir.path(),
            "xdg-open",
            &format!("#!/bin/sh\nprintf '%s' \"$1\" > {}\n", opened.display()),
        );

        execute(&notify, Some(&opener), "/tmp/shot.png", "Saved", "shot.png").unwrap();
        assert!(!opened.exists());
    }
}
