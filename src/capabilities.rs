use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use crate::exec::which;

/// Every external program ss-belt knows how to drive.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolId {
    Grim,
    Slurp,
    Spectacle,
    GnomeScreenshot,
    WlCopy,
    WlPaste,
    Xclip,
    Satty,
    Swappy,
    WlScreenrec,
    WfRecorder,
    Ffmpeg,
    Gifski,
    Tesseract,
    Exiftool,
    NotifySend,
    XdgOpen,
    Gdbus,
    Systemctl,
}

impl ToolId {
    pub const ALL: [ToolId; 19] = [
        ToolId::Grim,
        ToolId::Slurp,
        ToolId::Spectacle,
        ToolId::GnomeScreenshot,
        ToolId::WlCopy,
        ToolId::WlPaste,
        ToolId::Xclip,
        ToolId::Satty,
        ToolId::Swappy,
        ToolId::WlScreenrec,
        ToolId::WfRecorder,
        ToolId::Ffmpeg,
        ToolId::Gifski,
        ToolId::Tesseract,
        ToolId::Exiftool,
        ToolId::NotifySend,
        ToolId::XdgOpen,
        ToolId::Gdbus,
        ToolId::Systemctl,
    ];

    pub fn binary(self) -> &'static str {
        match self {
            ToolId::Grim => "grim",
            ToolId::Slurp => "slurp",
            ToolId::Spectacle => "spectacle",
            ToolId::GnomeScreenshot => "gnome-screenshot",
            ToolId::WlCopy => "wl-copy",
            ToolId::WlPaste => "wl-paste",
            ToolId::Xclip => "xclip",
            ToolId::Satty => "satty",
            ToolId::Swappy => "swappy",
            ToolId::WlScreenrec => "wl-screenrec",
            ToolId::WfRecorder => "wf-recorder",
            ToolId::Ffmpeg => "ffmpeg",
            ToolId::Gifski => "gifski",
            ToolId::Tesseract => "tesseract",
            ToolId::Exiftool => "exiftool",
            ToolId::NotifySend => "notify-send",
            ToolId::XdgOpen => "xdg-open",
            ToolId::Gdbus => "gdbus",
            ToolId::Systemctl => "systemctl",
        }
    }

    pub fn purpose(self) -> &'static str {
        match self {
            ToolId::Grim => "Wayland capture on wlroots compositors",
            ToolId::Slurp => "region selection on wlroots compositors",
            ToolId::Spectacle => "capture on KDE Plasma",
            ToolId::GnomeScreenshot => "capture on GNOME",
            ToolId::WlCopy => "copy to the Wayland clipboard",
            ToolId::WlPaste => "read the Wayland clipboard",
            ToolId::Xclip => "clipboard on X11",
            ToolId::Satty => "annotation window (post-capture)",
            ToolId::Swappy => "annotation window (post-capture)",
            ToolId::WlScreenrec => "screen recording on wlroots",
            ToolId::WfRecorder => "screen recording on wlroots (fallback)",
            ToolId::Ffmpeg => "recording fallback and GIF conversion",
            ToolId::Gifski => "higher quality GIF encoding",
            ToolId::Tesseract => "OCR",
            ToolId::Exiftool => "metadata inspection and stripping",
            ToolId::NotifySend => "desktop notifications",
            ToolId::XdgOpen => "opening files and folders",
            ToolId::Gdbus => "GNOME Shell screenshot fallback",
            ToolId::Systemctl => "installing the user service",
        }
    }

    /// (Fedora, Debian, Arch) package names. openSUSE reuses the Arch name.
    pub fn packages(self) -> (&'static str, &'static str, &'static str) {
        match self {
            ToolId::Grim => ("grim", "grim", "grim"),
            ToolId::Slurp => ("slurp", "slurp", "slurp"),
            ToolId::Spectacle => ("spectacle", "spectacle", "spectacle"),
            ToolId::GnomeScreenshot => ("gnome-screenshot", "gnome-screenshot", "gnome-screenshot"),
            ToolId::WlCopy | ToolId::WlPaste => ("wl-clipboard", "wl-clipboard", "wl-clipboard"),
            ToolId::Xclip => ("xclip", "xclip", "xclip"),
            ToolId::Satty => ("satty", "satty", "satty"),
            ToolId::Swappy => ("swappy", "swappy", "swappy"),
            ToolId::WlScreenrec => ("wl-screenrec", "wl-screenrec", "wl-screenrec"),
            ToolId::WfRecorder => ("wf-recorder", "wf-recorder", "wf-recorder"),
            ToolId::Ffmpeg => ("ffmpeg", "ffmpeg", "ffmpeg"),
            ToolId::Gifski => ("gifski", "gifski", "gifski"),
            ToolId::Tesseract => ("tesseract", "tesseract", "tesseract"),
            ToolId::Exiftool => {
                ("perl-Image-ExifTool", "libimage-exiftool-perl", "perl-image-exiftool")
            }
            ToolId::NotifySend => ("libnotify", "libnotify-bin", "libnotify"),
            ToolId::XdgOpen => ("xdg-utils", "xdg-utils", "xdg-utils"),
            ToolId::Gdbus => ("glib2", "libglib2.0-bin", "glib2"),
            ToolId::Systemctl => ("systemd", "systemd", "systemd"),
        }
    }

    pub fn package_for(self, distro: Distro) -> &'static str {
        let (fedora, debian, arch) = self.packages();
        match distro {
            Distro::Debian => debian,
            Distro::Arch | Distro::Suse => arch,
            _ => fedora,
        }
    }

    /// Reverse lookup for errors raised by a failed exec.
    pub fn from_binary(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|tool| tool.binary() == name)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Distro {
    Fedora,
    Debian,
    Arch,
    Suse,
    Unknown,
}

impl Distro {
    pub fn detect() -> Self {
        let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let mut id = String::new();
        let mut like = String::new();
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("ID=") {
                id = value.trim_matches('"').to_ascii_lowercase();
            } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
                like = value.trim_matches('"').to_ascii_lowercase();
            }
        }
        let haystack = format!("{id} {like}");
        if haystack.contains("fedora") || haystack.contains("rhel") || haystack.contains("centos") {
            Distro::Fedora
        } else if haystack.contains("debian")
            || haystack.contains("ubuntu")
            || haystack.contains("mint")
        {
            Distro::Debian
        } else if haystack.contains("arch") || haystack.contains("manjaro") {
            Distro::Arch
        } else if haystack.contains("suse") {
            Distro::Suse
        } else {
            Distro::Unknown
        }
    }

    pub fn install_command(self, packages: &[&str]) -> String {
        let list = packages.join(" ");
        match self {
            Distro::Fedora => format!("sudo dnf install {list}"),
            Distro::Debian => format!("sudo apt install {list}"),
            Distro::Arch => format!("sudo pacman -S {list}"),
            Distro::Suse => format!("sudo zypper install {list}"),
            Distro::Unknown => format!("install {list} with your package manager"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToolSet {
    paths: BTreeMap<ToolId, Option<PathBuf>>,
}

impl ToolSet {
    pub fn detect() -> Self {
        let mut paths = BTreeMap::new();
        for tool in ToolId::ALL {
            let found = which(tool.binary()).or_else(|| match tool {
                _ => None,
            });
            paths.insert(tool, found);
        }
        Self { paths }
    }

    /// Build a set from an explicit list, used by tests.
    pub fn from_paths(pairs: impl IntoIterator<Item = (ToolId, PathBuf)>) -> Self {
        let mut paths = BTreeMap::new();
        for tool in ToolId::ALL {
            paths.insert(tool, None);
        }
        for (tool, path) in pairs {
            paths.insert(tool, Some(path));
        }
        Self { paths }
    }

    pub fn has(&self, tool: ToolId) -> bool {
        self.paths.get(&tool).map(Option::is_some).unwrap_or(false)
    }

    pub fn path(&self, tool: ToolId) -> Option<&PathBuf> {
        self.paths.get(&tool).and_then(|p| p.as_ref())
    }

    /// The program name to invoke: the resolved path when known, else the bare
    /// binary name so the failure message still makes sense.
    pub fn program(&self, tool: ToolId) -> String {
        match self.path(tool) {
            Some(path) => path.display().to_string(),
            None => tool.binary().to_string(),
        }
    }

    pub fn missing(&self, tools: &[ToolId]) -> Vec<ToolId> {
        tools.iter().copied().filter(|tool| !self.has(*tool)).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ToolId, Option<&PathBuf>)> {
        self.paths.iter().map(|(tool, path)| (*tool, path.as_ref()))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SessionType {
    Wayland,
    X11,
    None,
}

impl SessionType {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionType::Wayland => "wayland",
            SessionType::X11 => "x11",
            SessionType::None => "none",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Desktop {
    pub session: SessionType,
    pub desktop: String,
    pub is_wlroots: bool,
    pub has_xwayland: bool,
}

impl Desktop {
    pub fn detect() -> Self {
        let session = if env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
            SessionType::Wayland
        } else if env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()) {
            SessionType::X11
        } else {
            SessionType::None
        };

        let desktop = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_lowercase();
        let wlroots_names = ["niri", "sway", "hyprland", "river", "wayfire", "labwc", "dwl", "cosmic"];

        Self {
            session,
            is_wlroots: wlroots_names.iter().any(|name| desktop.contains(name)),
            has_xwayland: env::var_os("DISPLAY").is_some(),
            desktop,
        }
    }

    pub fn is_wayland(&self) -> bool {
        self.session == SessionType::Wayland
    }

    pub fn is_x11(&self) -> bool {
        self.session == SessionType::X11
    }

    pub fn is_kde(&self) -> bool {
        self.desktop.contains("kde") || self.desktop.contains("plasma")
    }

    pub fn is_gnome(&self) -> bool {
        self.desktop.contains("gnome") || self.desktop.contains("unity")
    }

    pub fn label(&self) -> String {
        if self.desktop.is_empty() {
            self.session.as_str().to_string()
        } else {
            format!("{} ({})", self.desktop, self.session.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_reports_only_absent_tools() {
        let tools = ToolSet::from_paths([(ToolId::Grim, PathBuf::from("/usr/bin/grim"))]);
        assert!(tools.has(ToolId::Grim));
        assert!(!tools.has(ToolId::Slurp));
        assert_eq!(tools.missing(&[ToolId::Grim, ToolId::Slurp]), vec![ToolId::Slurp]);
    }

    #[test]
    fn every_tool_has_a_package_name_everywhere() {
        for tool in ToolId::ALL {
            let (fedora, debian, arch) = tool.packages();
            for name in [fedora, debian, arch] {
                assert!(!name.is_empty(), "{tool:?}");
            }
        }
    }

    #[test]
    fn package_names_follow_each_distro() {
        assert_eq!(ToolId::NotifySend.package_for(Distro::Fedora), "libnotify");
        assert_eq!(ToolId::NotifySend.package_for(Distro::Debian), "libnotify-bin");
        assert_eq!(ToolId::Exiftool.package_for(Distro::Arch), "perl-image-exiftool");
    }

    #[test]
    fn distro_install_commands_name_a_package_manager() {
        assert_eq!(
            Distro::Fedora.install_command(&["grim", "slurp"]),
            "sudo dnf install grim slurp"
        );
        assert_eq!(Distro::Debian.install_command(&["grim"]), "sudo apt install grim");
        assert_eq!(Distro::Arch.install_command(&["grim"]), "sudo pacman -S grim");
    }

    #[test]
    fn binaries_map_back_to_tools() {
        assert_eq!(ToolId::from_binary("grim"), Some(ToolId::Grim));
        assert_eq!(ToolId::from_binary("not-a-tool"), None);
    }
}
