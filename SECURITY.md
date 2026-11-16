# Security policy

## Scope

ss-belt runs with your user privileges and drives other programs. The security
properties it intends to guarantee:

- It never invokes a shell. Programs are spawned with an argv array, so file
  names and config values cannot become shell syntax.
- It never expands a filename template outside the configured save directory.
- It strips image metadata before upload unless you turn that off.
- It performs no network activity except uploads you ask for.

## Reporting

Report a vulnerability privately to the maintainers rather than in a public
issue. Include the ss-belt version (`ss-belt --version`), the session type
(Wayland/X11), the exact command, and a minimal reproduction.

If the issue is in an external tool ss-belt drives (`grim`, `satty`,
`wl-clipboard`, `ffmpeg`, …), report it to that project as well; ss-belt cannot
fix it, but a workaround may be possible.

## What ss-belt does not protect against

- **Malicious external tools.** ss-belt trusts the programs on your `PATH`, as
  any orchestrator must. A compromised `grim` or `satty` is a compromised
  capture.
- **Malicious `.sxcu` files.** They choose where your files are uploaded. Only
  load uploaders you understand; the URL and arguments are visible in the JSON.
- **Upload destinations.** Files leave your machine; the destination's privacy
  policy applies.
- **Clipboard contents.** Reading the clipboard is user-initiated, but anything
  that can read the X11 selection can read it too.
- **Screen content.** Captures may contain secrets. ss-belt writes them to
  `save_dir` (and the staging cache) with your normal file permissions.

## Supported versions

Only the latest release receives fixes. V1 is `0.1.x`.
