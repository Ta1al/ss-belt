# ss-belt

One toolbelt for Linux screen capture: **region and full-screen capture,
annotation, GIF/video recording, OCR, QR encode and decode, and uploads** —
with one config, one hotkey table and one history.

ss-belt is an orchestrator, not another capture engine. It probes the tools you
already have (`grim`, `spectacle`, `satty`, `wl-screenrec`, `ffmpeg`,
`wl-clipboard`, `tesseract` …), exposes them as a typed task chain, and builds
the parts that are cheap and reliable in Rust itself — QR encoding/decoding,
hashing, history, uploads. When a tool is missing, ss-belt says so up front and
tells you the command that installs it, instead of failing halfway through a
capture.

```
$ ss-belt doctor
ss-belt 0.1.0 · Fedora Linux 44
desktop     niri (wayland)
...
CAPABILITY         STATUS       DETAIL
capture.region     unavailable  region capture needs a Wayland selector (grim + slurp) ...
capture.full       degraded     XWayland only: native Wayland windows will be missing ...
annotate           ready
qr.scan            ready
```

## Quick start

```sh
# build and install into ~/.local
PREFIX=$HOME/.local ./packaging/install.sh

ss-belt doctor                  # what works on this machine
ss-belt config --check          # validate the generated config
ss-belt hotkeys --print niri    # or sway / hyprland / gnome / kde
```

Optional but recommended tools:

| Session | Install |
| --- | --- |
| Wayland (sway, Hyprland, niri, river…) | `grim` `slurp` `wl-clipboard` `wl-screenrec` |
| GNOME | `gnome-screenshot` `wl-clipboard` |
| KDE Plasma | `spectacle` `wl-clipboard` |
| X11 / XWayland | `ffmpeg` `xclip` |
| Annotation | `satty` (recommended) or `swappy` |
| OCR | `tesseract` |
| GIF quality | `gifski` |

`ss-belt doctor` prints the exact install command for your distribution.

## What you can do

```sh
ss-belt region         # select a region, save it, copy it   (needs grim + slurp)
ss-belt full           # whole screen
ss-belt window         # active window                      (GNOME / KDE backends)
ss-belt monitor        # current monitor
ss-belt annotate       # select, then annotate before saving
ss-belt clipboard-edit # edit the image already on the clipboard
ss-belt qr-scan        # decode a QR code from a region and copy the text
ss-belt qr-make "text" # render text as a QR image
ss-belt ocr            # extract text from a region
ss-belt gif            # start/stop a GIF recording
ss-belt video          # start/stop a video recording
ss-belt upload FILE    # upload a file and copy the link
ss-belt upload         # re-upload the last capture
```

Every action prints a machine-friendly result on stdout (a path, a URL or the
text), so it composes:

```sh
ss-belt region | xargs -r ss-belt upload
```

`--dry-run` walks the whole chain without capturing, writing or uploading
anything, and still honours capability checks so it tells you when an action
would fail on this machine:

```sh
$ ss-belt full --dry-run
-> capture.full (capture every monitor)
XWayland only: native Wayland windows will be missing from the capture
dry-run: would capture full with ffmpeg x11grab
-> save (write the artifact into the save directory)
dry-run: would save .../staging/.../capture.png to ~/Pictures/ss-belt/full_....png
-> clipboard.copy (copy the current value to the clipboard)
dry-run: would copy .../capture.png to the clipboard
-> notify (show a desktop notification)
dry-run: would notify `image .../capture.png`
```

## Feature support

| Feature | Wayland native | X11 / XWayland | Notes |
| --- | --- | --- | --- |
| Region capture | ✅ grim + slurp | ❌ | Wayland has no client-side global overlays |
| Full screen | ✅ grim / spectacle / gnome-screenshot | ✅ ffmpeg x11grab | |
| Active window | ✅ spectacle / gnome-screenshot | ❌ | Wayland has no window enumeration |
| Per-monitor | ⚠️ backend-dependent | ⚠️ captures all screens | |
| Annotate | ✅ satty / swappy | ✅ | post-capture editor, not an in-overlay editor |
| GIF / video | ✅ wl-screenrec / wf-recorder | ✅ ffmpeg | |
| OCR | ✅ tesseract | ✅ | |
| QR scan / make | ✅ built in | ✅ | no external decoder needed |
| Clipboard | ✅ wl-clipboard | ✅ xclip | |
| Uploads | ✅ `.sxcu` + presigned URLs | ✅ | |
| Global hotkeys | ⚠️ compositor-owned | ✅ X11 grab | see below |

Known limitations, by design:

- **No inline annotation overlay.** `satty`/`swappy` edit a frozen capture, so
  annotation is a second window rather than drawing during selection.
- **No pin-to-screen or mouse highlighter.** Wayland gives clients no absolute
  window placement and no click-through overlays.
- **No scrolling capture.** It needs synthetic wheel events, which Wayland
  compositors refuse.
- **Windows-only features stay Windows-only**: window shadows, transparent
  window capture, HDR tone mapping, hiding the taskbar during capture.

## Hotkeys

Wayland compositors do not let unprivileged clients grab global keys, so the
compositor owns the binding and calls ss-belt:

```sh
ss-belt hotkeys --print niri      # KDL snippet for ~/.config/niri/config.kdl
ss-belt hotkeys --print sway      # sway/i3 config lines
ss-belt hotkeys --print hyprland  # hyprland.conf lines
ss-belt hotkeys --print gnome     # gsettings commands
ss-belt hotkeys --print kde       # commands to add in System Settings
```

On X11, `ss-belt daemon` grabs the configured keys itself; `ss-belt hotkeys
--check` validates every accelerator.

## The daemon

The daemon holds the hotkeys, the tray icon (StatusNotifierItem) and a socket
for other programs:

```sh
ss-belt daemon --detach      # start in the background (logs to ~/.local/share/ss-belt/daemon.log)
ss-belt daemon status
ss-belt trigger region       # ask the running daemon to run an action
ss-belt daemon reload        # re-read the config without a restart
ss-belt daemon stop
systemctl --user enable --now ss-belt   # if installed as a package
```

Actions other programs can trigger: `region`, `full`, `window`, `monitor`,
`annotate`, `clipboard_edit`, `qr_scan`, `qr_make`, `ocr`, `upload`,
`annotate_last`, `gif`, `video`.

### Tray icon

While the daemon runs it publishes a StatusNotifierItem:

| Click | What happens |
| --- | --- |
| **Left** | Capture a region immediately |
| **Right** | Menu with all thirteen tools, grouped Capture / Create / Record / Share, plus Reload config and Quit |
| **Middle** | Upload the last capture |

The icon is embedded in the binary (32px + 64px), so it renders even if your
icon theme has no `ss-belt` entry. While a recording runs the title becomes
`ss-belt (recording)` and the record entries switch to *Stop …* — including
recordings started from a hotkey or the CLI.

Some desktops need a host for the icon: KDE Plasma and quickshell/waybar show it
out of the box, stock GNOME needs the AppIndicator extension. `ss-belt doctor`
reports `tray available` / `unavailable` so you know which case you are in.

## Configuration

`~/.config/ss-belt/config.toml`, created with comments on first run. Every key
is optional; delete a line to get the documented default back.

```toml
[general]
save_dir = "~/Pictures/ss-belt"
filename_template = "{action}_{datetime}"   # {action} {date} {time} {unix} {ext}
annotator = "auto"                          # auto | satty | swappy | none
copy_after_save = true
notify = true

[capture]
backend = "auto"        # auto | grim | spectacle | gnome | x11
delay_ms = 0
cursor = false

[record]
fps = 30
gif_fps = 15
gif_width = 960
duration_secs = 0       # 0 = record until the hotkey is pressed again

[hotkeys]
region = "Mod4+Shift+4"
qr_scan = "Mod4+Shift+Q"
```

Custom pipelines replace built-in actions:

```toml
[actions.share]
steps = ["capture.region", "annotate", "upload", "clipboard.copy", "notify"]
```

`ss-belt steps` lists every step with its input/output type and whether it can
run on this host; `ss-belt actions` lists the built-in actions.

### Uploads

ShareX `.sxcu` custom uploaders work as-is: drop the file in
`~/.config/ss-belt/uploaders/` and reference it.

```toml
[upload]
default_destination = "catbox"

[upload.destinations.catbox]
type = "sxcu"
file = "~/.config/ss-belt/uploaders/catbox.sxcu"

[upload.destinations.r2]
type = "presigned"
url = "https://example.r2.cloudflarestorage.com/bucket/{filename}?X-Amz-..."
method = "PUT"
```

Supported `.sxcu` bodies: `MultipartFormData`, `FormURLEncoded`, `Binary`,
`JSON` (text). Templates: `$filename$`, `$input$`, `$response$`, `$json:path$`
(including `files[0].url`). `$regex:` and `$xml:` are rejected with a clear
message. EXIF and other metadata are stripped from images before upload unless
`upload.strip_metadata = false`.

## History

Every step is recorded in `~/.local/share/ss-belt/history.sqlite`:

```sh
ss-belt history --limit 20
ss-belt history --uploads
ss-belt history --clear
```

## Troubleshooting

- **"no capture backend available"** — run `ss-belt doctor`; on Wayland install
  `grim` and `slurp`, on GNOME `gnome-screenshot`, on KDE `spectacle`.
- **Region capture does nothing** — `slurp` is what draws the selection; check
  `ss-belt steps --available`.
- **Nothing on the clipboard** — install `wl-clipboard` (Wayland) or `xclip`
  (X11). ss-belt never writes the clipboard through a shell pipe, so a missing
  owner is reported instead of silently dropped.
- **Hotkeys do nothing under Wayland** — expected: use
  `ss-belt hotkeys --print <desktop>` and let the compositor bind them.
- **No tray icon** — `ss-belt doctor` reports the tray as unavailable when no
  StatusNotifierItem host is running; install waybar/quickshell, use KDE Plasma,
  or enable the GNOME AppIndicator extension.
- **Daemon won't start** — another instance owns the socket; `ss-belt daemon
  status`, then `ss-belt daemon stop`. Logs: `~/.local/share/ss-belt/daemon.log`.

## Building

```sh
cargo build --release --locked
cargo test
```

| Target | Command |
| --- | --- |
| Debian/Ubuntu | `cargo install cargo-deb && cargo deb` |
| Fedora/RHEL | `cargo install cargo-generate-rpm && cargo generate-rpm` |
| Arch | `cd packaging && makepkg -si` |
| AppImage | `./packaging/appimage.sh` |
| From source | `./packaging/install.sh` |

Everything is local: no telemetry, no network access except the upload
destination you configure.

## Design

See [SPEC.md](SPEC.md) for the architecture, the typed pipeline, the capability
model and the security rules (argv-only exec, no shell, secrets from the
keyring, metadata stripping).

## License

MIT. ss-belt is not affiliated with ShareX; the `.sxcu` format is supported for
interoperability with its ecosystem.
