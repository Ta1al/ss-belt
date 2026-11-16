//! Start the daemon at login.
//!
//! A user unit has to live where systemd looks, and it has to point at the
//! binary that is actually installed. Both were wrong when this was left to a
//! hand-written file in `~/.local/lib/systemd/user`, which systemd ignores.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::capabilities::Desktop;
use crate::error::{user, HintExt};
use crate::exec::{which, Cmd};

pub const UNIT_NAME: &str = "ss-belt.service";

#[derive(Debug, Clone)]
pub struct Status {
    pub unit_path: Option<PathBuf>,
    pub installed: bool,
    pub enabled: String,
    pub active: String,
}

/// `$XDG_CONFIG_HOME/systemd/user/ss-belt.service`
pub fn unit_path() -> Option<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(unit_path_in(&config_home))
}

pub fn unit_path_in(config_home: &Path) -> PathBuf {
    config_home.join("systemd/user").join(UNIT_NAME)
}

impl Status {
    pub fn line(&self) -> String {
        match &self.unit_path {
            Some(path) if self.installed => {
                format!("enabled: {} · active: {} · unit: {}", self.enabled, self.active, path.display())
            }
            Some(path) => format!("installed: no · unit would be {}", path.display()),
            None => "installed: no (no home directory)".to_string(),
        }
    }
}

pub fn status() -> Status {
    let unit_path = unit_path();
    let installed = unit_path.as_ref().map(|path| path.exists()).unwrap_or(false);
    let enabled = systemctl(&["is-enabled", UNIT_NAME]).unwrap_or_else(|| "unknown".into());
    let active = systemctl(&["is-active", UNIT_NAME]).unwrap_or_else(|| "unknown".into());
    Status { unit_path, installed, enabled, active }
}

fn systemctl(args: &[&str]) -> Option<String> {
    which("systemctl")?;
    let output = Cmd::new("systemctl").arg("--user").args(args.iter().copied()).timeout(Duration::from_secs(10)).run().ok()?;
    let text = output.stdout_str();
    Some(if text.is_empty() { "unknown".to_string() } else { text })
}

/// The unit text for a specific binary path.
pub fn unit_contents(exe: &Path) -> String {
    let exe = exe.display();
    format!(
        "[Unit]
Description=ss-belt capture daemon (hotkeys, tray, socket)
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart={exe} daemon
ExecReload={exe} daemon reload
Restart=on-failure
RestartSec=2

[Install]
WantedBy=graphical-session.target
"
    )
}

/// Install the unit, enable it and start it now.
pub fn install() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locating the ss-belt executable")?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let path = unit_path()
        .ok_or_else(|| user("cannot determine a config directory").hint("set HOME or XDG_CONFIG_HOME"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, unit_contents(&exe))
        .with_context(|| format!("writing {}", path.display()))?;

    if which("systemctl").is_none() {
        return Err(user("systemctl is not installed")
            .hint("the unit was written; start `ss-belt daemon` from your session instead"));
    }

    // A manually started daemon would hold the socket and make the service
    // fail; hand control over to systemd.
    if crate::daemon::is_running(&crate::paths::Paths::discover()) {
        let _ = crate::daemon::send(&crate::paths::Paths::discover(), "quit");
        std::thread::sleep(Duration::from_millis(400));
    }

    Cmd::new("systemctl").args(["--user", "daemon-reload"]).timeout(Duration::from_secs(20)).checked()?;
    Cmd::new("systemctl")
        .args(["--user", "enable", "--now", UNIT_NAME])
        .timeout(Duration::from_secs(30))
        .checked()?;

    Ok(path)
}

pub fn uninstall() -> Result<()> {
    if which("systemctl").is_some() {
        let _ = Cmd::new("systemctl")
            .args(["--user", "disable", "--now", UNIT_NAME])
            .timeout(Duration::from_secs(30))
            .run();
    }
    if let Some(path) = unit_path() {
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        if which("systemctl").is_some() {
            let _ = Cmd::new("systemctl")
                .args(["--user", "daemon-reload"])
                .timeout(Duration::from_secs(20))
                .run();
        }
    }
    Ok(())
}

/// For compositors that never start `graphical-session.target`, the session
/// itself has to spawn the daemon.
pub fn compositor_snippet(desktop: &Desktop) -> String {
    let name = desktop.desktop.as_str();
    if name.contains("niri") {
        "spawn-at-startup \"ss-belt\" \"daemon\"".to_string()
    } else if name.contains("hyprland") || name.contains("hypr") {
        "exec-once = ss-belt daemon".to_string()
    } else if name.contains("sway") || name.contains("i3") {
        "exec ss-belt daemon".to_string()
    } else {
        "ss-belt daemon".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unit_lands_where_systemd_looks() {
        let path = unit_path_in(Path::new("/home/user/.config"));
        assert_eq!(path, Path::new("/home/user/.config/systemd/user/ss-belt.service"));
    }

    #[test]
    fn the_unit_points_at_the_installed_binary() {
        let unit = unit_contents(Path::new("/home/user/.local/bin/ss-belt"));
        assert!(unit.contains("ExecStart=/home/user/.local/bin/ss-belt daemon"));
        assert!(unit.contains("ExecReload=/home/user/.local/bin/ss-belt daemon reload"));
        assert!(unit.contains("WantedBy=graphical-session.target"));
        assert!(unit.contains("After=graphical-session.target"));
    }

    #[test]
    fn compositor_fallbacks_match_each_desktop() {
        let niri = Desktop {
            session: crate::capabilities::SessionType::Wayland,
            desktop: "niri".into(),
            is_wlroots: true,
            has_xwayland: true,
        };
        assert_eq!(compositor_snippet(&niri), "spawn-at-startup \"ss-belt\" \"daemon\"");
        let sway = Desktop { desktop: "sway".into(), ..niri };
        assert_eq!(compositor_snippet(&sway), "exec ss-belt daemon");
    }
}
