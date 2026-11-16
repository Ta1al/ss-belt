#!/bin/sh
# Install ss-belt from a local checkout.
#
#   ./packaging/install.sh              # /usr/local, needs sudo
#   PREFIX=$HOME/.local ./packaging/install.sh
#   ./packaging/install.sh --uninstall
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX=${PREFIX:-/usr/local}
BINDIR="$PREFIX/bin"
APPDIR="$PREFIX/share/applications"
ICONDIR="$PREFIX/share/icons/hicolor/scalable/apps"
UNITDIR="$PREFIX/lib/systemd/user"
BASH_COMP="$PREFIX/share/bash-completion/completions"
ZSH_COMP="$PREFIX/share/zsh/site-functions"
FISH_COMP="$PREFIX/share/fish/vendor_completions.d"

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BINDIR/ss-belt" "$APPDIR/ss-belt.desktop" "$ICONDIR/ss-belt.svg" \
        "$UNITDIR/ss-belt.service" "$BASH_COMP/ss-belt" "$ZSH_COMP/_ss-belt" \
        "$FISH_COMP/ss-belt.fish"
  echo "removed ss-belt from $PREFIX"
  exit 0
fi

echo "==> building release binary"
cargo build --release --locked --manifest-path "$root/Cargo.toml"

echo "==> installing into $PREFIX"
install -d "$BINDIR" "$APPDIR" "$ICONDIR" "$UNITDIR" "$BASH_COMP" "$ZSH_COMP" "$FISH_COMP"
install -m755 "$root/target/release/ss-belt" "$BINDIR/ss-belt"
install -m644 "$root/packaging/ss-belt.desktop" "$APPDIR/ss-belt.desktop"
install -m644 "$root/packaging/ss-belt.svg" "$ICONDIR/ss-belt.svg"
install -m644 "$root/packaging/ss-belt.service" "$UNITDIR/ss-belt.service"
install -m644 "$root/packaging/completions/ss-belt.bash" "$BASH_COMP/ss-belt"
install -m644 "$root/packaging/completions/ss-belt.zsh" "$ZSH_COMP/_ss-belt"
install -m644 "$root/packaging/completions/ss-belt.fish" "$FISH_COMP/ss-belt.fish"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPDIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
fi

cat <<EOF

installed. next steps:
  ss-belt doctor                  # see what works on this machine
  ss-belt config --check          # validate the config
  ss-belt hotkeys --print niri    # or sway / hyprland / gnome / kde
  systemctl --user enable --now ss-belt   # optional background daemon
EOF
