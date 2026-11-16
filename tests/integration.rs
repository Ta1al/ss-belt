//! End-to-end tests that drive the real binary against fake tools on PATH.
//!
//! Nothing here touches the real profile: `SS_BELT_HOME` relocates every file,
//! and `PATH` is prefixed with `tests/fixtures/bin` so capture/clipboard/
//! notification calls land in the test directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = Self { dir };
        fs::create_dir_all(fixture.shots()).unwrap();
        fs::create_dir_all(fixture.root().join("config")).unwrap();
        fs::write(
            fixture.config_file(),
            format!("[general]\nsave_dir = \"{}\"\n", fixture.shots().display()),
        )
        .unwrap();
        fs::write(fixture.types_file(), "image/png\ntext/plain\n").unwrap();
        fs::write(fixture.clipboard_file(), b"").unwrap();
        fixture
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn shots(&self) -> PathBuf {
        self.root().join("shots")
    }

    fn config_file(&self) -> PathBuf {
        self.root().join("config/config.toml")
    }

    fn types_file(&self) -> PathBuf {
        self.root().join("types.txt")
    }

    fn clipboard_file(&self) -> PathBuf {
        self.root().join("clipboard.bin")
    }

    fn notify_file(&self) -> PathBuf {
        self.root().join("notify.log")
    }

    fn fake_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin")
    }

    fn fixture_png() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/px.png")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ss-belt"));
        let path = format!(
            "{}:{}",
            Self::fake_bin().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .env("SS_BELT_HOME", self.root())
            .env("PATH", path)
            .env("SS_BELT_TEST_PNG", Self::fixture_png())
            .env("SS_BELT_TEST_CLIPBOARD", self.clipboard_file())
            .env("SS_BELT_TEST_TYPES", self.types_file())
            .env("SS_BELT_TEST_NOTIFY", self.notify_file())
            .env("WAYLAND_DISPLAY", "wayland-1")
            .env("XDG_CURRENT_DESKTOP", "niri")
            .env_remove("DISPLAY")
            .env("RUST_LOG", "error");
        command
    }

    /// An X11 session: no Wayland socket, a DISPLAY, and the ffmpeg fake.
    fn x11_command(&self) -> Command {
        let mut command = self.command();
        command
            .env("DISPLAY", ":0")
            .env("XDG_CURRENT_DESKTOP", "i3")
            .env_remove("WAYLAND_DISPLAY");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("running ss-belt")
    }

    /// Notifications are sent by a detached helper, so poll for the effect.
    fn wait_for_file_text(&self, path: &Path, needle: &str) -> String {
        for _ in 0..100 {
            if let Ok(text) = fs::read_to_string(path) {
                if text.contains(needle) {
                    return text;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        fs::read_to_string(path).unwrap_or_default()
    }

    fn text(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`ss-belt {}` failed:\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}

#[test]
fn prints_version_and_help() {
    let fixture = Fixture::new();
    let version = fixture.text(&["--version"]);
    assert!(version.starts_with("ss-belt "), "{version}");
    let help = fixture.text(&["--help"]);
    assert!(help.contains("Capture a region"));
    assert!(help.contains("doctor"));
}

#[test]
fn lists_actions_and_steps_with_capability_status() {
    let fixture = Fixture::new();
    let actions = fixture.text(&["actions"]);
    for action in ["region", "annotate", "qr_scan", "ocr", "gif", "video", "upload"] {
        assert!(actions.contains(action), "missing {action} in:\n{actions}");
    }

    let steps = fixture.text(&["steps"]);
    assert!(steps.contains("capture.region"));
    assert!(steps.contains("qr.scan"));
    assert!(steps.contains("ready"), "no ready step:\n{steps}");
}

#[test]
fn doctor_reports_json_for_the_scripted_environment() {
    let fixture = Fixture::new();
    let json = fixture.text(&["doctor", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["session"], "wayland");
    let capabilities = parsed["capabilities"].as_array().unwrap();
    assert!(capabilities.iter().any(|row| row["step"] == "capture.region"));
    let tools = parsed["tools"].as_array().unwrap();
    let grim = tools.iter().find(|row| row["name"] == "grim").unwrap();
    assert_eq!(grim["found"], true);
    assert!(parsed["daemon"].as_str().unwrap().contains("not running"));
}

#[test]
fn config_defaults_validate_and_print() {
    let fixture = Fixture::new();
    let check = fixture.text(&["config", "--check"]);
    assert!(check.contains("is valid"), "{check}");
    let default = fixture.text(&["config", "--default"]);
    assert!(default.contains("[general]"));
    assert!(default.contains("filename_template"));
    let path = fixture.text(&["config", "--path"]);
    assert_eq!(PathBuf::from(path), fixture.config_file());
}

#[test]
fn hotkeys_print_for_each_supported_desktop() {
    let fixture = Fixture::new();
    let niri = fixture.text(&["hotkeys", "--print", "niri"]);
    assert!(niri.contains("spawn"));
    assert!(niri.contains("ss-belt region"));

    let sway = fixture.text(&["hotkeys", "--print", "sway"]);
    assert!(sway.contains("bindsym Mod4+Shift+4 exec ss-belt region"));

    let gnome = fixture.text(&["hotkeys", "--print", "gnome"]);
    assert!(gnome.contains("gsettings set"));

    let check = fixture.text(&["hotkeys", "--check"]);
    assert!(check.contains("ok ("), "{check}");
    assert!(!check.contains("INVALID"), "{check}");
}

#[test]
fn dry_run_touches_nothing() {
    let fixture = Fixture::new();
    let output = fixture.text(&["region", "--dry-run"]);
    assert!(output.contains("capture.png"), "{output}");
    assert_eq!(fs::read_dir(fixture.shots()).unwrap().count(), 0);
    assert_eq!(fs::read(fixture.clipboard_file()).unwrap().len(), 0);
}

#[test]
fn region_capture_saves_copies_and_notifies() {
    let fixture = Fixture::new();
    let saved = PathBuf::from(fixture.text(&["region"]));

    assert!(saved.starts_with(fixture.shots()), "{}", saved.display());
    let bytes = fs::read(&saved).unwrap();
    assert_eq!(&bytes[..4], b"\x89PNG", "saved file is not a PNG");

    let clipboard = fs::read(fixture.clipboard_file()).unwrap();
    assert_eq!(&clipboard[..4], b"\x89PNG", "image was not copied");

    let notify = fixture.wait_for_file_text(&fixture.notify_file(), "Capture saved");
    assert!(notify.contains("Capture saved"), "{notify}");
    assert_eq!(
        notify.lines().count(),
        1,
        "exactly one notification per capture, got:\n{notify}"
    );

    let history = fixture.text(&["history", "--limit", "10"]);
    assert!(history.contains("capture.region"), "{history}");
    assert!(history.contains("save"), "{history}");
}

#[test]
fn x11_full_capture_goes_through_ffmpeg() {
    let fixture = Fixture::new();
    let output = fixture.x11_command().args(["full"]).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    assert!(saved.exists(), "{}", saved.display());
    let bytes = fs::read(&saved).unwrap();
    assert_eq!(&bytes[..4], b"\x89PNG");
    assert!(fixture.text(&["history", "--limit", "5"]).contains("capture.full"));
}

#[test]
fn annotate_pipeline_runs_the_annotator() {
    let fixture = Fixture::new();
    let saved = PathBuf::from(fixture.text(&["annotate"]));
    assert!(saved.exists());
    let history = fixture.text(&["history", "--limit", "10"]);
    assert!(history.contains("annotate"), "{history}");
}

#[test]
fn qr_make_then_scan_round_trips_through_the_screen() {
    let fixture = Fixture::new();
    let qr_path = PathBuf::from(fixture.text(&["qr-make", "https://example.com/belt"]));
    assert!(qr_path.exists());

    // The next capture "sees" the QR code we just rendered.
    let output = fixture
        .command()
        .env("SS_BELT_TEST_PNG", &qr_path)
        .args(["qr-scan"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "https://example.com/belt"
    );
    let clipboard = fs::read_to_string(fixture.clipboard_file()).unwrap();
    assert_eq!(clipboard, "https://example.com/belt");
}

#[test]
fn scanning_a_blank_region_fails_with_a_useful_message() {
    let fixture = Fixture::new();
    let output = fixture.run(&["qr-scan"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no QR code"), "{stderr}");
    assert!(stderr.contains("hint:"), "{stderr}");
}

#[test]
fn video_toggle_starts_then_finishes_a_recording() {
    let fixture = Fixture::new();

    let started = fixture.run(&["video"]);
    assert!(started.status.success(), "{}", String::from_utf8_lossy(&started.stderr));
    let stderr = String::from_utf8_lossy(&started.stderr);
    assert!(stderr.contains("recording"), "{stderr}");

    let running = fixture.text(&["status"]);
    assert!(running.contains("recording"), "{running}");

    let finished = fixture.run(&["video"]);
    assert!(finished.status.success(), "{}", String::from_utf8_lossy(&finished.stderr));
    let saved = PathBuf::from(String::from_utf8_lossy(&finished.stdout).trim());
    assert!(saved.exists(), "{}", saved.display());
    assert_eq!(fs::read_to_string(&saved).unwrap(), "fake-mp4-bytes");

    assert!(!fixture.text(&["status"]).contains("recording"));
}

#[test]
fn fixed_duration_recording_stops_by_itself() {
    let fixture = Fixture::new();
    let output = fixture.run(&["video", "--duration", "1"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    assert!(saved.exists(), "{}", saved.display());
    assert!(!fixture.text(&["status"]).contains("recording"));
}

#[test]
fn a_missing_package_error_names_the_install_command() {
    let fixture = Fixture::new();
    let image = Fixture::fixture_png();
    let output = fixture.run(&["ocr", "--file", image.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("OCR needs tesseract"), "{stderr}");
    assert!(stderr.contains("install with:"), "{stderr}");
    assert!(
        stderr.contains("dnf")
            || stderr.contains("apt")
            || stderr.contains("pacman")
            || stderr.contains("zypper")
            || stderr.contains("package manager"),
        "{stderr}"
    );
}

#[test]
fn daemon_commands_report_when_not_running() {
    let fixture = Fixture::new();
    let output = fixture.run(&["daemon", "status"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no daemon is running"), "{stderr}");
    assert!(stderr.contains("ss-belt daemon"), "{stderr}");
}

#[test]
fn history_clear_empties_the_database() {
    let fixture = Fixture::new();
    fixture.text(&["region"]);
    let cleared = fixture.text(&["history", "--clear"]);
    assert!(cleared.contains("cleared"), "{cleared}");
    let after = fixture.text(&["history"]);
    assert!(after.contains("no history yet"), "{after}");
}
