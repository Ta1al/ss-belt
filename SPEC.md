# ss-belt specification

This document is the design contract for V1. It explains *why* the code looks
the way it does, so changes can be judged against the intent.

## 1. Problem

Linux has excellent single-purpose capture tools and no ShareX: no single place
with one hotkey table, one config, one history and one task chain across
capture → annotate → OCR/QR → save/copy/upload.

ShareX itself cannot simply be ported: it is a Windows Forms application built
on GDI, DWM and `RegisterHotKey`. Wayland deliberately removes most of what it
assumes — global overlays, window enumeration, synthetic input, absolute window
placement.

## 2. Strategy: orchestrate, do not re-implement

Two kinds of work:

1. **Platform-heavy, already solved** — screen capture, recording, annotation
   UI, OCR. Delegate to tools that already solved Wayland portals, HiDPI,
   codecs and compositor quirks.
2. **Cheap and reliable in Rust** — QR encode/decode, hashing, image
   normalization, config, history, uploads, the task chain itself. Build these
   in.

Consequence: ss-belt inherits Wayland compatibility from `grim`, `wl-screenrec`
and `satty` instead of spending months re-deriving it, and always reports what
it cannot do rather than failing at the last step.

### Non-goals

Inline annotation during selection, pin-to-screen, mouse highlighter, scrolling
capture, window shadows/HDR capture, and any feature that requires global
overlays or synthetic input on Wayland. These are documented as limitations,
not bugs.

## 3. Architecture

```
main.rs (clap)                     CLI surface
  └── action.rs                    named actions → pipelines
        └── pipeline.rs            validate chain, run steps, record history
              └── steps.rs         typed Value, Ctx, StepSpec registry
                    └── adapters/  one module per capability
daemon.rs                          socket + worker + hotkeys + tray
hotkeys/                           accelerator parsing, X11 grabs, per-DE snippets
doctor.rs                          capability matrix + install hints
sxcu.rs                            ShareX custom uploader parsing/templating
```

One crate, library + binary. The library is the testable surface; the binary is
argument parsing and output formatting.

### Typed pipeline

```rust
enum Value { Empty, Image(PathBuf), Video(PathBuf), Text(String), Url(String) }
enum Kind  { None, Image, Video, Text, Url, Any }
```

Every step declares an input kind and an output kind. `validate_chain` rejects a
pipeline before it runs, so a config typo is reported as a config error instead
of failing after a capture. `Any` exists only for genuinely dynamic steps (the
clipboard can hold an image or text); those enforce the type at run time.

`Pipeline.start` allows a chain to begin with an artifact already in hand — this
is how a recording's second half (`gif.convert → save → copy`) validates without
pretending it starts from nothing.

### The step registry

`adapters::STEPS` is the single source of truth: name, summary, input, output,
`support()` and `run()`. Consequences:

- `ss-belt steps` and `ss-belt doctor` cannot drift from reality — they read the
  same table the runner uses.
- `config.validate()` rejects unknown step names at load time.
- Adding a capability means adding one entry plus one module.

### Capability model

```rust
enum Support { Ready, Degraded(String), Unavailable(String) }
```

`Degraded` means "works, with a documented limitation" (e.g. XWayland capture on
a Wayland session). `Unavailable` carries the reason and, where possible, the
install command for the user's distribution. `Probe` answers questions without
running anything, so `doctor` is safe to run anywhere, including over SSH.

Backend selection is explicit-first: `capture.backend = "grim"` is honoured even
if something else looks available, so a user can pin behaviour.

## 4. Security rules

- **No shell, ever.** `exec::Cmd` spawns a program with an argv array. A
  filename containing `;`, `$(...)` or backticks is just a filename. There is a
  test for this (`args_are_never_shell_expanded`).
- **Timeouts and kill.** Every external call has a deadline; a hung tool is
  killed and reported, never left to wedge the daemon.
- **No output capture for forking programs.** `wl-copy`, `xclip` and
  `notify-send` fork a long-lived child. Inheriting a pipe would keep ss-belt
  blocked forever, so those calls use `.quiet()` (null stdio). This was a real
  hang, not a theoretical one.
- **Sanitized filenames.** Templates are reduced to `[A-Za-z0-9._-]`, `..` is
  collapsed and leading/trailing dots are trimmed, so a template cannot escape
  the save directory.
- **Metadata stripping.** Uploads re-encode PNG/JPEG through the decoder to drop
  EXIF/GPS before sending, unless explicitly disabled.
- **Uploads only over HTTP(S)**; `.sxcu` URLs are validated at load time.
- **No telemetry, no background network activity.** The only network call is the
  upload the user asks for.

## 5. Failure policy

| Situation | Behaviour |
| --- | --- |
| User cancels a selection / annotator | `Cancelled` error, pipeline stops, exit 0, no partial file |
| Missing tool | `UserError` with a hint + distro install command |
| Step fails mid-pipeline | Steps already done stay done (a saved file is saved); error names the step |
| Recording already running | Refuse to start a second one, point at the running state file |
| Daemon self-probe | The daemon passes `daemon_running = true` explicitly; probing its own socket from the accept loop would deadlock |

## 6. State and files

| Path | Purpose |
| --- | --- |
| `~/.config/ss-belt/config.toml` | config, written with comments on first run |
| `~/.config/ss-belt/uploaders/*.sxcu` | ShareX custom uploaders |
| `~/.local/share/ss-belt/history.sqlite` | step history (WAL) |
| `~/.local/share/ss-belt/last` | path of the last saved capture |
| `~/.cache/ss-belt/staging/<run>/` | per-run artifacts (capture, annotated, GIF) |
| `$XDG_RUNTIME_DIR/ss-belt/daemon.sock` | daemon control socket |
| `$XDG_RUNTIME_DIR/ss-belt/state.json` | active recording (pid, path, kind) |
| `~/.local/share/ss-belt/daemon.log` | detached daemon log |

`SS_BELT_HOME` relocates all of them under one directory; the integration tests
depend on this to guarantee they never touch a real profile.

## 7. Testing strategy

- **Unit tests** per adapter and per data structure: parse/validate/serialize,
  argv construction, QR round-trip, `.sxcu` templating, HTTP upload against an
  in-process one-shot server, history pruning, X11 keysym mapping.
- **Integration tests** run the real binary with `tests/fixtures/bin` prefixed to
  `PATH`: fake `grim`, `slurp`, `wl-copy`, `wl-paste`, `xclip`, `satty`,
  `wl-screenrec`, `ffmpeg`, `notify-send`. They assert end-to-end effects: file
  saved, clipboard bytes, notification text, history rows, recording lifecycle,
  daemon absence handling.
- **Dry runs** execute the real chain with each step's simulation path, so
  capability selection is still exercised without side effects.

## 8. Packaging

`cargo-deb` and `cargo-generate-rpm` metadata live in `Cargo.toml`; Arch,
AppImage and a source installer live in `packaging/`. External tools are
`Recommends`/`optdepends`, never hard dependencies: ss-belt starts and explains
itself on a bare system.

## 9. Roadmap beyond V1

1. Wayland `GlobalShortcuts` portal for compositors that implement it.
2. X11 region selector (X11 is the one place a client-side overlay is possible).
3. A settings window (GTK4/libadwaita) over the same config file.
4. OCR language selection and bundled `tessdata` presets.
5. `$regex:` support in `.sxcu` templates.
6. S3 SigV4 signing for non-presigned destinations.
