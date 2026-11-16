use anyhow::Result;

use crate::config::Config;
use crate::error::HintExt;
use crate::hotkeys::Accelerator;
use crate::paths::Paths;

/// Print an integration snippet for a desktop environment.
///
/// Wayland compositors do not let an unprivileged client grab global keys, so
/// the honest answer is to give the compositor the binding and call ss-belt.
pub fn print(desktop: &str, config: &Config, paths: &Paths) -> Result<String> {
    let bindings = bindings(config);
    let out = match desktop.to_ascii_lowercase().as_str() {
        "niri" => niri(&bindings),
        "sway" | "i3" => sway(&bindings),
        "hyprland" | "hypr" => hyprland(&bindings),
        "gnome" => gnome(&bindings),
        "kde" | "plasma" => kde(&bindings),
        "x11" | "generic" | "other" => generic(&bindings),
        other => {
            return Err(crate::error::user(format!("unknown desktop `{other}`"))
                .hint("use niri, sway, hyprland, gnome, kde or generic"))
        }
    };
    Ok(format!(
        "# ss-belt hotkeys for {desktop}\n# config: {}\n#\n# {}\n\n{out}",
        paths.config_file().display(),
        header(desktop),
    ))
}

fn header(desktop: &str) -> &'static str {
    match desktop.to_ascii_lowercase().as_str() {
        "niri" => "Paste into ~/.config/niri/config.kdl, then `niri msg action reload-config`.",
        "sway" | "i3" => "Paste into ~/.config/sway/config, then `swaymsg reload`.",
        "hyprland" | "hypr" => "Paste into ~/.config/hypr/hyprland.conf, then `hyprctl reload`.",
        "gnome" => "Run these once; they survive logout.",
        "kde" | "plasma" => "Add each command in System Settings > Shortcuts > Custom Shortcuts.",
        _ => "Bind these commands however your desktop allows.",
    }
}

fn bindings(config: &Config) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for action in crate::config::HOTKEY_ACTIONS {
        let Some(accelerator) = config.hotkeys.get(action) else {
            continue;
        };
        if let Ok(parsed) = Accelerator::parse(accelerator) {
            out.push((
                action.to_string(),
                parsed.label(),
                format!("ss-belt {}", action.replace('_', "-")),
            ));
        }
    }
    out
}

fn niri(bindings: &[(String, String, String)]) -> String {
    let mut out = String::new();
    for (_, label, command) in bindings {
        let combo = label.replace("Mod4", "Mod");
        out.push_str(&format!("{combo} {{ spawn \"sh\" \"-c\" \"{command}\"; }}\n"));
    }
    out
}

fn sway(bindings: &[(String, String, String)]) -> String {
    let mut out = String::new();
    for (_, label, command) in bindings {
        out.push_str(&format!("bindsym {label} exec {command}\n"));
    }
    out
}

fn hyprland(bindings: &[(String, String, String)]) -> String {
    let mut out = String::new();
    for (_, label, command) in bindings {
        let (mods, key) = label.rsplit_once('+').unwrap_or(("", label));
        let mods = mods
            .split('+')
            .map(|m| match m {
                "Mod4" => "SUPER",
                "Shift" => "SHIFT",
                "Ctrl" => "CTRL",
                "Alt" => "ALT",
                other => other,
            })
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("bind = {mods}, {key}, exec, {command}\n"));
    }
    out
}

fn gnome(bindings: &[(String, String, String)]) -> String {
    let mut paths = Vec::new();
    for (action, _, _) in bindings {
        paths.push(format!(
            "'/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ss-belt-{action}/'"
        ));
    }
    let mut out = format!(
        "gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings \"[{}]\"\n",
        paths.join(", ")
    );
    for (action, label, command) in bindings {
        let base = format!(
            "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/ss-belt-{action}/"
        );
        out.push_str(&format!("gsettings set {base} name 'ss-belt {action}'\n"));
        out.push_str(&format!("gsettings set {base} command '{command}'\n"));
        out.push_str(&format!(
            "gsettings set {base} binding '{}'\n",
            gnome_binding(label)
        ));
    }
    out
}

fn gnome_binding(label: &str) -> String {
    let mut out = String::new();
    let mut key = String::new();
    for part in label.split('+') {
        match part {
            "Mod4" => out.push_str("<Super>"),
            "Shift" => out.push_str("<Shift>"),
            "Ctrl" => out.push_str("<Control>"),
            "Alt" => out.push_str("<Alt>"),
            other => key = other.to_string(),
        }
    }
    out.push_str(&key);
    out
}

fn kde(bindings: &[(String, String, String)]) -> String {
    let mut out = String::new();
    for (action, label, command) in bindings {
        out.push_str(&format!("# {action} ({label})\n#   command: {command}\n"));
    }
    out
}

fn generic(bindings: &[(String, String, String)]) -> String {
    let mut out = String::from("#!/bin/sh\n# Example: bind these with xbindkeys, sxhkd, or your WM.\n");
    for (_action, label, command) in bindings {
        out.push_str(&format!("# {label:<16} {command}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn niri_uses_spawn_blocks() {
        let out = print("niri", &config(), &Paths::discover()).unwrap();
        assert!(out.contains("Mod+Shift+4 {"));
        assert!(out.contains("spawn"));
        assert!(out.contains("ss-belt region"));
    }

    #[test]
    fn sway_uses_bindsym_exec() {
        let out = print("sway", &config(), &Paths::discover()).unwrap();
        assert!(out.contains("bindsym Mod4+Shift+4 exec ss-belt region"));
    }

    #[test]
    fn hyprland_translates_modifiers() {
        let out = print("hyprland", &config(), &Paths::discover()).unwrap();
        assert!(out.contains("bind = SUPER SHIFT, 4, exec, ss-belt region"));
    }

    #[test]
    fn gnome_emits_gsettings_commands() {
        let out = print("gnome", &config(), &Paths::discover()).unwrap();
        assert!(out.contains("gsettings set"));
        assert!(out.contains("<Super><Shift>4"));
    }

    #[test]
    fn unknown_desktop_is_a_user_error_with_hint() {
        let err = print("beos", &config(), &Paths::discover()).unwrap_err();
        assert!(crate::error::is_user_error(&err));
        assert!(format!("{err}").contains("hint"));
    }
}
