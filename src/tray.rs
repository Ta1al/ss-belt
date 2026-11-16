use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};
use ksni::{ToolTip, Tray};
use std::sync::mpsc::Sender;

use crate::daemon::Event;

pub struct BeltTray {
    pub sender: Sender<Event>,
    pub recording: bool,
}

impl Tray for BeltTray {
    fn id(&self) -> String {
        "ss-belt".to_string()
    }

    fn title(&self) -> String {
        if self.recording {
            "ss-belt (recording)".to_string()
        } else {
            "ss-belt".to_string()
        }
    }

    fn icon_name(&self) -> String {
        "camera-photo-symbolic".to_string()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: self.title(),
            description: "Capture, annotate, record, OCR, QR and upload".to_string(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = Vec::new();
        for (action, label) in [
            ("region", "Capture region"),
            ("full", "Capture screen"),
            ("annotate", "Capture and annotate"),
            ("qr_scan", "Scan QR code"),
            ("ocr", "OCR region"),
            ("gif", "Record GIF"),
            ("video", "Record video"),
            ("upload", "Upload last"),
        ] {
            items.push(menu_item(label, Some(action), self.sender.clone()));
        }
        items.push(MenuItem::Separator);
        items.push(menu_item("Reload config", None, self.sender.clone()));
        items.push(MenuItem::Separator);
        items.push(quit_item(self.sender.clone()));
        items
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(Event::Action("region".into()));
    }
}

/// `action == None` means "reload".
fn menu_item(label: &str, action: Option<&str>, sender: Sender<Event>) -> MenuItem<BeltTray> {
    let event = match action {
        Some(name) => Event::Action(name.to_string()),
        None => Event::Reload,
    };
    MenuItem::Standard(StandardItem {
        label: label.to_string(),
        activate: Box::new(move |_tray: &mut BeltTray| {
            let _ = sender.send(event.clone());
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

/// Spawn the StatusNotifierItem service.
///
/// A missing tray host (niri, stock GNOME without the AppIndicator extension) is
/// not fatal: there is simply nobody to render the icon, and every action stays
/// available through hotkeys and the CLI.
pub fn spawn(sender: Sender<Event>, recording: bool) -> Option<ksni::blocking::Handle<BeltTray>> {
    match (BeltTray { sender, recording }).spawn() {
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::debug!("no StatusNotifierItem host: {err}");
            None
        }
    }
}
