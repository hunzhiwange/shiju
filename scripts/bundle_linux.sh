#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-target/distrib}"
PACKAGE_NAME="${PACKAGE_NAME:-shiju}"
LINUX_ICON_SOURCE="${LINUX_ICON_SOURCE:-assets/logo.png}"
LINUX_DESKTOP_SOURCE="${LINUX_DESKTOP_SOURCE:-release/linux/vibewindow.desktop}"
CREATE_DEB="${CREATE_DEB:-1}"
CREATE_RPM="${CREATE_RPM:-1}"

BIN_DIR="target/$TARGET/$PROFILE"
WORK_DIR="$OUT_DIR/linux-package-$TARGET"
ROOT_DIR="$WORK_DIR/root"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 1
  }
}

read_cargo_field() {
  local field="$1"
  awk -F '"' -v key="$field" '
    $1 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      print $2
      exit
    }
  ' crates/vw-desktop/Cargo.toml
}

target_arch() {
  case "$TARGET" in
    x86_64-unknown-linux-gnu) echo "amd64 x86_64" ;;
    aarch64-unknown-linux-gnu) echo "arm64 aarch64" ;;
    *)
      echo "error: unsupported Linux target for package build: $TARGET" >&2
      exit 1
      ;;
  esac
}

require_binary() {
  local name="$1"
  if [ ! -f "$BIN_DIR/$name" ]; then
    echo "error: expected binary not found: $BIN_DIR/$name" >&2
    exit 1
  fi
}

install_common_root() {
  rm -rf "$WORK_DIR"
  mkdir -p \
    "$ROOT_DIR/usr/bin" \
    "$ROOT_DIR/usr/share/applications" \
    "$ROOT_DIR/usr/share/icons/hicolor/256x256/apps" \
    "$ROOT_DIR/usr/share/doc/$PACKAGE_NAME"

  install -m 0755 "$BIN_DIR/shi-ju" "$ROOT_DIR/usr/bin/shi-ju"
  install -m 0755 "$BIN_DIR/vw-webview" "$ROOT_DIR/usr/bin/vw-webview"
  install -m 0755 "$BIN_DIR/shiju" "$ROOT_DIR/usr/bin/shiju"
  install -m 0755 "$BIN_DIR/acp" "$ROOT_DIR/usr/bin/acp"
  install -m 0644 "$LINUX_DESKTOP_SOURCE" "$ROOT_DIR/usr/share/applications/shiju.desktop"
  install -m 0644 "$LINUX_ICON_SOURCE" "$ROOT_DIR/usr/share/icons/hicolor/256x256/apps/shiju.png"

  cat > "$ROOT_DIR/usr/share/doc/$PACKAGE_NAME/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: ShiJu
Source: $HOMEPAGE
License: $LICENSE

Files: *
Copyright: ShiJu contributors
License: $LICENSE
EOF
}

write_deb_maintainer_scripts() {
  local debian_dir="$ROOT_DIR/DEBIAN"
  mkdir -p "$debian_dir"

  cat > "$debian_dir/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
exit 0
EOF

  cat > "$debian_dir/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
exit 0
EOF

  chmod 0755 "$debian_dir/postinst" "$debian_dir/postrm"
}

build_deb() {
  need_cmd dpkg-deb
  write_deb_maintainer_scripts

  cat > "$ROOT_DIR/DEBIAN/control" <<EOF
Package: $PACKAGE_NAME
Version: $DEB_VERSION
Section: devel
Priority: optional
Architecture: $DEB_ARCH
Maintainer: ShiJu contributors <actions@github.com>
Depends: libc6, libgcc-s1, libgtk-3-0, libwebkit2gtk-4.1-0, libfontconfig1, libxkbcommon0, hicolor-icon-theme
Homepage: $HOMEPAGE
Description: $DESCRIPTION
 Rust-first autonomous agent runtime with CLI, ACP, desktop client, and webview helper.
EOF

  local deb_name="$PACKAGE_NAME"_"$DEB_VERSION"_"$DEB_ARCH.deb"
  rm -f "$OUT_DIR/$deb_name"
  dpkg-deb --build --root-owner-group "$ROOT_DIR" "$OUT_DIR/$deb_name"
  echo "$OUT_DIR/$deb_name"
}

build_rpm() {
  need_cmd rpmbuild
  rm -rf "$ROOT_DIR/DEBIAN"

  local rpm_topdir="$WORK_DIR/rpm"
  local spec="$WORK_DIR/$PACKAGE_NAME.spec"
  rm -rf "$rpm_topdir"
  mkdir -p "$rpm_topdir/BUILD" "$rpm_topdir/BUILDROOT" "$rpm_topdir/RPMS" "$rpm_topdir/SOURCES" "$rpm_topdir/SPECS" "$rpm_topdir/SRPMS"
  local rpm_topdir_abs
  local root_dir_abs
  local spec_abs
  rpm_topdir_abs="$(cd "$rpm_topdir" && pwd -P)"
  root_dir_abs="$(cd "$ROOT_DIR" && pwd -P)"
  spec_abs="$(cd "$(dirname "$spec")" && pwd -P)/$(basename "$spec")"

  cat > "$spec" <<EOF
%global debug_package %{nil}

Name: $PACKAGE_NAME
Version: $RPM_VERSION
Release: 1
Summary: $DESCRIPTION
License: $LICENSE
URL: $HOMEPAGE
BuildArch: $RPM_ARCH
Requires: hicolor-icon-theme

%description
Rust-first autonomous agent runtime with CLI, ACP, desktop client, and webview helper.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a "$root_dir_abs/." %{buildroot}/

%post
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi

%postun
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi

%files
%doc /usr/share/doc/$PACKAGE_NAME/copyright
/usr/bin/shi-ju
/usr/bin/vw-webview
/usr/bin/shiju
/usr/bin/acp
/usr/share/applications/shiju.desktop
/usr/share/icons/hicolor/256x256/apps/shiju.png
EOF

  rpmbuild -bb "$spec_abs" --define "_topdir $rpm_topdir_abs"
  find "$rpm_topdir/RPMS" -type f -name '*.rpm' -exec cp {} "$OUT_DIR/" \;
  find "$OUT_DIR" -maxdepth 1 -type f -name "$PACKAGE_NAME-$RPM_VERSION-1.$RPM_ARCH.rpm" -print
}

read -r DEB_ARCH RPM_ARCH <<< "$(target_arch)"
VERSION="$(read_cargo_field version)"
DESCRIPTION="$(read_cargo_field description)"
LICENSE="$(read_cargo_field license)"
HOMEPAGE="$(read_cargo_field homepage)"
DEB_VERSION="$(printf '%s' "$VERSION" | tr '-' '~')"
RPM_VERSION="$(printf '%s' "$VERSION" | tr '-' '~')"

mkdir -p "$OUT_DIR"

require_binary shi-ju
require_binary vw-webview
require_binary shiju
require_binary acp

if [ ! -f "$LINUX_DESKTOP_SOURCE" ]; then
  echo "error: Linux desktop file not found: $LINUX_DESKTOP_SOURCE" >&2
  exit 1
fi
if [ ! -f "$LINUX_ICON_SOURCE" ]; then
  echo "error: Linux icon not found: $LINUX_ICON_SOURCE" >&2
  exit 1
fi

install_common_root

if [ "$CREATE_DEB" = "1" ]; then
  build_deb
fi
if [ "$CREATE_RPM" = "1" ]; then
  build_rpm
fi

rm -rf "$WORK_DIR"
