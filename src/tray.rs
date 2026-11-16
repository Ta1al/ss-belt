//! StatusNotifierItem (system tray) integration.
//!
//! Left click captures a region; middle click re-uploads the last capture;
//! right click opens the full tool menu. The icon is embedded so it renders even
//! when the desktop's icon theme has no matching name.

use std::sync::mpsc::Sender;

use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, ToolTip, Tray};

use crate::daemon::Event;

const ICON_32: &[u8] = include_bytes!("../assets/ss-belt-32.png");
const ICON_64: &[u8] = include_bytes!("../assets/ss-belt-64.png");
const ICON_SVG: &str = include_str!("../packaging/ss-belt.svg");

pub struct BeltTray {
    pub sender: Sender<Event>,
    pub recording: bool,
    /// Directory handed to the host through `IconThemePath`.
    pub icon_theme: String,
}

/// Short, menu-friendly labels. Every action in `action::ACTIONS` must have one;
/// a unit test enforces it.
pub fn label_for(action: &str) -> Option<&'static str> {
    Some(match action {
        "region" => "Capture region",
        "full" => "Capture screen",
        "window" => "Capture window",
        "monitor" => "Capture monitor",
        "annotate" => "Capture and annotate",
        "clipboard_edit" => "Edit clipboard image",
        "qr_scan" => "Scan QR code",
        "qr_make" => "Make QR code",
        "ocr" => "OCR region",
        "upload" => "Upload last capture",
        "annotate_last" => "Annotate last capture",
        "gif" => "Record GIF",
        "video" => "Record video",
        _ => return None,
    })
}

/// Groups used by the menu, in display order.
const GROUPS: &[(&str, &[&str])] = &[
    ("Capture", &["region", "full", "window", "monitor"]),
    ("Create", &["annotate", "clipboard_edit", "qr_scan", "qr_make", "ocr"]),
    ("Record", &["gif", "video"]),
    ("Share", &["upload", "annotate_last"]),
];

impl Tray for BeltTray {
    fn id(&self) -> String {
        "ss-belt".to_string()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn title(&self) -> String {
        if self.recording {
            "ss-belt (recording)".to_string()
        } else {
            "ss-belt".to_string()
        }
    }

    fn icon_name(&self) -> String {
        "ss-belt".to_string()
    }

    fn icon_theme_path(&self) -> String {
        self.icon_theme.clone()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        icons()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: self.title(),
            description: if self.recording {
                "Recording — click to capture a region".to_string()
            } else {
                "Click: capture region · Right click: tools".to_string()
            },
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = Vec::new();

        // A disabled status line keeps the state visible without a tooltip.
        items.push(disabled(if self.recording {
            "Recording…"
        } else {
            "Ready"
        }));
        items.push(MenuItem::Separator);

        for (index, (group, actions)) in GROUPS.iter().enumerate() {
            if index > 0 {
                items.push(MenuItem::Separator);
            }
            items.push(disabled(group));
            for action_name in *actions {
                let Some(label) = label_for(action_name) else { continue };
                // While recording, the record entries become stop entries.
                let label = if self.recording && matches!(*action_name, "gif" | "video") {
                    match *action_name {
                        "gif" => "Stop GIF recording".to_string(),
                        _ => "Stop video recording".to_string(),
                    }
                } else {
                    label.to_string()
                };
                items.push(item(&label, action_name, self.sender.clone()));
            }
        }

        items.push(MenuItem::Separator);
        items.push(reload_item(self.sender.clone()));
        items.push(quit_item(self.sender.clone()));
        items
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(Event::Action(activate_action().to_string()));
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(Event::Action(secondary_action().to_string()));
    }
}

/// Left click on the tray icon.
pub fn activate_action() -> &'static str {
    "region"
}

/// Middle click on the tray icon.
pub fn secondary_action() -> &'static str {
    "upload"
}

fn disabled(label: &str) -> MenuItem<BeltTray> {
    MenuItem::Standard(StandardItem {
        label: label.to_string(),
        enabled: false,
        ..Default::default()
    })
}

fn item(label: &str, action_name: &str, sender: Sender<Event>) -> MenuItem<BeltTray> {
    let event = Event::Action(action_name.to_string());
    MenuItem::Standard(StandardItem {
        label: label.to_string(),
        activate: Box::new(move |_tray: &mut BeltTray| {
            let _ = sender.send(event.clone());
        }),
        ..Default::default()
    })
}

fn reload_item(sender: Sender<Event>) -> MenuItem<BeltTray> {
    MenuItem::Standard(StandardItem {
        label: "Reload config".to_string(),
        activate: Box::new(move |_tray: &mut BeltTray| {
            let _ = sender.send(Event::Reload);
        }),
        ..Default::default()
    })
}

fn quit_item(sender: Sender<Event>) -> MenuItem<BeltTray> {
    MenuItem::Standard(StandardItem {
        label: "Quit".to_string(),
        activate: Box::new(move |_tray: &mut BeltTray| {
            let _ = sender.send(Event::Quit);
        }),
        ..Default::default()
    })
}

/// Decode the embedded PNGs into the ARGB32 network-byte-order form the
/// StatusNotifierItem spec requires.
fn icons() -> Vec<Icon> {
    [ICON_32, ICON_64]
        .iter()
        .filter_map(|png| icon_from_png(png).ok())
        .collect()
}

fn icon_from_png(png: &[u8]) -> anyhow::Result<Icon> {
    let image = image::load_from_memory(png)?.to_rgba8();
    let (width, height) = image.dimensions();
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        data.extend_from_slice(&[a, r, g, b]);
    }
    Ok(Icon { width: width as i32, height: height as i32, data })
}

/// Spawn the tray service.
///
/// Returns `None` when the desktop has no StatusNotifierItem host (niri, or
/// stock GNOME without the AppIndicator extension): the daemon still runs, but
/// there is nowhere to draw the icon.
pub fn spawn(sender: Sender<Event>, recording: bool) -> Option<ksni::blocking::Handle<BeltTray>> {
    // Hosts that resolve the icon by name need it to exist in an icon theme.
    // Installing it into the user's hicolor theme also fixes bars that ignore
    // the pixmap entirely.
    let icon_theme = ensure_icon_installed()
        .map(|icons| icons.theme_path.display().to_string())
        .unwrap_or_default();

    match (BeltTray { sender, recording, icon_theme }).spawn() {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!("no StatusNotifierItem host: {err}");
            None
        }
    }
}

fn data_home() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/share")))
}

pub struct InstalledIcons {
    /// Directory containing files named exactly `ss-belt.png` / `ss-belt.svg`.
    /// This is what `IconThemePath` points at, because hosts (quickshell among
    /// them) resolve `IconThemePath + "/" + IconName` as a literal path.
    pub theme_path: std::path::PathBuf,
    /// The regular hicolor theme location, for hosts that do a real theme lookup.
    pub hicolor: std::path::PathBuf,
}

/// Make the icon resolvable by name, without requiring a package install.
pub fn ensure_icon_installed() -> Option<InstalledIcons> {
    ensure_icon_installed_in(&data_home()?).ok()
}

pub fn ensure_icon_installed_in(data_home: &std::path::Path) -> anyhow::Result<InstalledIcons> {
    let hicolor = data_home.join("icons/hicolor");
    let theme_path = data_home.join("ss-belt/icons");

    for (path, bytes) in [
        (theme_path.join("ss-belt.png"), ICON_64),
        (theme_path.join("ss-belt.svg"), ICON_SVG.as_bytes()),
        (hicolor.join("32x32/apps/ss-belt.png"), ICON_32),
        (hicolor.join("64x64/apps/ss-belt.png"), ICON_64),
        (hicolor.join("scalable/apps/ss-belt.svg"), ICON_SVG.as_bytes()),
    ] {
        if std::fs::read(&path).map(|existing| existing == bytes).unwrap_or(false) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
    }
    Ok(InstalledIcons { theme_path, hicolor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action;

    #[test]
    fn every_action_has_a_short_label() {
        for (name, _) in action::ACTIONS {
            assert!(label_for(name).is_some(), "no tray label for `{name}`");
        }
    }

    #[test]
    fn the_menu_covers_every_action() {
        let listed: Vec<&str> = GROUPS.iter().flat_map(|(_, actions)| actions.iter().copied()).collect();
        for (name, _) in action::ACTIONS {
            assert!(listed.contains(name), "`{name}` is missing from the tray menu");
        }
        assert_eq!(listed.len(), action::ACTIONS.len());
    }

    #[test]
    fn clicks_map_to_region_and_upload() {
        assert_eq!(activate_action(), "region");
        assert_eq!(secondary_action(), "upload");
    }

    #[test]
    fn the_embedded_icon_is_argb32_network_order() {
        let icons = icons();
        assert_eq!(icons.len(), 2);
        let icon = &icons[1];
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.data.len(), 64 * 64 * 4);
        // Corners are transparent (rounded background), the centre is opaque.
        assert_eq!(icon.data[3], 0, "corner should be transparent");
        let centre = ((32 * 64) + 32) * 4;
        assert_eq!(icon.data[centre], 255, "centre should be opaque");
    }

    #[test]
    fn the_icon_is_installed_for_both_lookup_styles() {
        let dir = tempfile::tempdir().unwrap();
        let icons = ensure_icon_installed_in(dir.path()).unwrap();

        // IconThemePath + IconName, which quickshell resolves literally.
        assert_eq!(icons.theme_path, dir.path().join("ss-belt/icons"));
        assert_eq!(std::fs::read(icons.theme_path.join("ss-belt.png")).unwrap(), ICON_64);
        assert!(icons.theme_path.join("ss-belt.svg").exists());

        // A real theme lookup through hicolor.
        assert!(icons.hicolor.ends_with("icons/hicolor"));
        assert_eq!(
            std::fs::read(icons.hicolor.join("64x64/apps/ss-belt.png")).unwrap(),
            ICON_64
        );
        // Idempotent: a second run leaves the files in place.
        ensure_icon_installed_in(dir.path()).unwrap();
        assert_eq!(
            std::fs::read(icons.hicolor.join("32x32/apps/ss-belt.png")).unwrap(),
            ICON_32
        );
    }

    #[test]
    fn the_menu_has_a_status_line_and_quit() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let tray = BeltTray { sender: tx, recording: false, icon_theme: String::new() };
        let menu = tray.menu();
        assert!(format!("{menu:?}").contains("Ready"));
        assert!(format!("{menu:?}").contains("Quit"));
        assert!(format!("{menu:?}").contains("Capture region"));
    }

    #[test]
    fn recording_switches_the_record_entries_to_stop() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let tray = BeltTray { sender: tx, recording: true, icon_theme: String::new() };
        let text = format!("{:?}", tray.menu());
        assert!(text.contains("Recording…"));
        assert!(text.contains("Stop GIF recording"));
        assert!(text.contains("Stop video recording"));
    }
}
