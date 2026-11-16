# Changelog

## 0.1.0 — 2026-11-16

First working release. Local-only; not published.

### Capture

- Region, full-screen, window and monitor capture with backend selection:
  `grim` (wlroots), `spectacle` (KDE), `gnome-screenshot` (GNOME) and a built-in
  X11/XWayland backend using `ffmpeg x11grab`.
- Capture delay and cursor options.
- Capability matrix that reports degraded and unavailable modes with reasons and
  per-distribution install commands instead of failing mid-pipeline.

### Editing and tools

- Post-capture annotation through `satty` or `swappy`, including editing an image
  already on the clipboard.
- Built-in QR encoding and decoding (no `zbarimg`/`zbar-tools` dependency).
- OCR through `tesseract`.
- SHA-256 hashing of any artifact.
- GIF conversion through `gifski` or `ffmpeg`'s palette pipeline.

### Recording

- GIF and video recording through `wl-screenrec`, `wf-recorder` or `ffmpeg`.
- Toggle semantics: the action starts a recording, the same action stops it and
  runs the finishing pipeline (convert, save, copy, notify).
- Fixed-duration recording via `--duration`.

### Upload

- ShareX `.sxcu` custom uploaders: `MultipartFormData`, `FormURLEncoded`,
  `Binary` and `JSON` (text) bodies; `$filename$`, `$input$`, `$response$` and
  `$json:path$` templates; `ErrorMessage` support.
- Pre-signed URL destinations (S3/R2/GCS) with `{filename}` substitution.
- EXIF/GPS stripping before upload by default.

### Notifications

- One notification per action (previously a capture produced two).
- Clicking the notification opens the capture, or the uploaded URL, through a
  detached helper that survives the command that created it.

### Shell and workflow

- Typed pipeline with pre-flight chain validation and custom actions in
  `[actions.<name>]`.
- `--dry-run` for every action.
- SQLite history with retention, `last` capture tracking, upload listing.
- `doctor` with JSON output, `steps --available`, `config --check`.
- X11 global hotkey grabbing, plus generated snippets for niri, sway, Hyprland,
  GNOME and KDE.
- Background daemon with a control socket, worker thread and log file.
- StatusNotifierItem tray: embedded icon, left click captures a region, middle
  click uploads the last capture, right click opens every tool grouped by
  Capture / Create / Record / Share. Title and record entries follow the
  recording state, including recordings started outside the daemon.
- `doctor` reports whether a tray host can draw the icon.
- The icon is installed into `$XDG_DATA_HOME/ss-belt/icons/` and `hicolor` on
  first run and advertised through `IconThemePath`, so it resolves without a
  package install.

### Packaging

- `cargo-deb` and `cargo-generate-rpm` metadata, Arch PKGBUILD, AppImage script,
  source installer, systemd user unit, desktop entry, icon and shell completions.
