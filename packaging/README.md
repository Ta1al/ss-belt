# Packaging

Every format below installs the same files:

| Path | File |
| --- | --- |
| `/usr/bin/ss-belt` | binary |
| `/usr/share/applications/ss-belt.desktop` | launcher with actions |
| `/usr/share/icons/hicolor/scalable/apps/ss-belt.svg` | icon |
| `/usr/lib/systemd/user/ss-belt.service` | optional daemon |
| `/usr/share/{bash-completion,zsh,fish}/…` | completions |

External tools are **recommendations, never hard dependencies**. ss-belt starts
on a bare system and explains what is missing (`ss-belt doctor`).

## Debian / Ubuntu

```sh
cargo install cargo-deb
cargo deb
sudo apt install ./target/debian/ss-belt_*_amd64.deb
```

Metadata lives in `[package.metadata.deb]` in `Cargo.toml`. `Depends` is spelled
out (`libc6`, `libgcc-s1`) instead of `$auto`, because `$auto` requires
`dpkg-shlibdeps`, which only exists on Debian hosts — a package built on Fedora
would silently end up with no dependencies at all.

Verify without installing:

```sh
ar t target/debian/*.deb
dpkg-deb -I target/debian/*.deb      # control fields
dpkg-deb -c target/debian/*.deb      # file list
```

## Fedora / RHEL

```sh
cargo install cargo-generate-rpm
cargo generate-rpm
sudo dnf install ./target/generate-rpm/ss-belt-*.rpm
```

Metadata lives in `[package.metadata.generate-rpm]`. `Recommends` maps to weak
dependencies, so `dnf install ss-belt --with-recommends` pulls the toolbelt.

```sh
rpm -qip target/generate-rpm/*.rpm
rpm -qlp target/generate-rpm/*.rpm
```

## Arch

```sh
cd packaging
makepkg -si
```

The PKGBUILD builds from the checkout it sits in, so it can be used before the
project has a public source tarball. `optdepends` lists the optional tools.

## AppImage

```sh
./packaging/appimage.sh
```

Needs `appimagetool` (or `APPIMAGETOOL=/path/to/appimagetool`). The image
bundles only ss-belt; capture tools stay system packages so the user can choose
grim vs spectacle vs ffmpeg.

## From source

```sh
PREFIX=$HOME/.local ./packaging/install.sh   # or sudo ./packaging/install.sh
./packaging/install.sh --uninstall
```

## Shell completions

`packaging/completions/` holds generated completions. Regenerate them after
changing the CLI:

```sh
for shell in bash zsh fish; do
  ./target/release/ss-belt completion "$shell" > "packaging/completions/ss-belt.$shell"
done
```

## Checking a package

Beyond the format-specific listing commands, a smoke test that needs no display:

```sh
ss-belt --version
ss-belt config --check
ss-belt doctor --json
```
