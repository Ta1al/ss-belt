#!/bin/sh
# Build a single-file AppImage from the release binary.
#
# Needs: cargo, and appimagetool on PATH (or APPIMAGETOOL pointing at it).
# The AppImage bundles only ss-belt: capture/recording tools are optional
# dependencies installed by the distribution, which keeps the image small and
# lets the user choose grim vs spectacle vs ffmpeg.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out="$root/dist"
appdir="$out/AppDir"

command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
tool=${APPIMAGETOOL:-$(command -v appimagetool || true)}
if [ -z "$tool" ]; then
  cat >&2 <<'EOF'
appimagetool was not found.
Install it from https://github.com/AppImage/appimagetool/releases and either
put it on PATH or set APPIMAGETOOL=/path/to/appimagetool.
EOF
  exit 1
fi

echo "==> building release binary"
cargo build --release --locked --manifest-path "$root/Cargo.toml"

echo "==> staging AppDir"
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" \
         "$appdir/usr/share/applications" \
         "$appdir/usr/share/icons/hicolor/scalable/apps" \
         "$appdir/usr/share/bash-completion/completions" \
         "$appdir/usr/share/zsh/site-functions" \
         "$appdir/usr/share/fish/vendor_completions.d"

install -m755 "$root/target/release/ss-belt" "$appdir/usr/bin/ss-belt"
install -m644 "$root/packaging/ss-belt.desktop" "$appdir/usr/share/applications/ss-belt.desktop"
install -m644 "$root/packaging/ss-belt.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/ss-belt.svg"
install -m644 "$root/packaging/completions/ss-belt.bash" "$appdir/usr/share/bash-completion/completions/ss-belt"
install -m644 "$root/packaging/completions/ss-belt.zsh" "$appdir/usr/share/zsh/site-functions/_ss-belt"
install -m644 "$root/packaging/completions/ss-belt.fish" "$appdir/usr/share/fish/vendor_completions.d/ss-belt.fish"
install -m644 "$root/LICENSE" "$appdir/LICENSE"

cp "$appdir/usr/share/applications/ss-belt.desktop" "$appdir/ss-belt.desktop"
cp "$appdir/usr/share/icons/hicolor/scalable/apps/ss-belt.svg" "$appdir/ss-belt.svg"
ln -sf usr/bin/ss-belt "$appdir/AppRun"

echo "==> building AppImage"
ARCH=${ARCH:-x86_64} "$tool" "$appdir" "$out/ss-belt-$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)-$ARCH.AppImage"

echo "done: $out"
