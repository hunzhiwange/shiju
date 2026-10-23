#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh [OPTIONS]

Build and package ShiJu release artifacts for one target.

Options:
  --kind <kind>        Package kind: cli, acp, desktop, all (default: all)
  --target <triple>   Rust target triple (default: current host)
  --profile <name>    Cargo profile: release or debug (default: release)
  --out-dir <dir>     Output directory (default: dist/release/<target>)
  --use-cross         Build through cross
  --use-xwin          Build Windows MSVC targets through cargo-xwin
  -h, --help          Show this help
EOF
}

host_target() {
  rustc -vV | awk '/^host:/ { print $2 }'
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: required command not found: $1" >&2
    exit 1
  }
}

KIND="all"
TARGET="$(host_target)"
PROFILE="release"
OUT_DIR=""
USE_CROSS=0
USE_XWIN=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --kind)
      KIND="$2"
      shift 2
      ;;
    --target)
      TARGET="$2"
      shift 2
      ;;
    --profile)
      PROFILE="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --use-cross)
      USE_CROSS=1
      shift
      ;;
    --use-xwin)
      USE_XWIN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

case "$KIND" in
  cli|acp|desktop|all) ;;
  *)
    echo "error: unsupported kind: $KIND" >&2
    exit 1
    ;;
esac

if [ -z "$OUT_DIR" ]; then
  OUT_DIR="dist/release/$TARGET"
fi

PROFILE_FLAG=()
if [ "$PROFILE" = "release" ]; then
  PROFILE_FLAG=(--release)
fi

EXE_SUFFIX=""
ARCHIVE_EXT="tar.gz"
if [[ "$TARGET" == *"-windows-"* ]]; then
  EXE_SUFFIX=".exe"
  ARCHIVE_EXT="zip"
fi

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
BIN_DIR="$CARGO_TARGET_DIR/$TARGET/$PROFILE"
STAGE_DIR="$OUT_DIR/stage-$KIND"

if [ "$USE_CROSS" -eq 1 ]; then
  need_cmd cross
  if ! docker info >/dev/null 2>&1; then
    echo "error: Docker is required when --use-cross is set" >&2
    exit 1
  fi
  CARGO_CMD=(cross build)
elif [ "$USE_XWIN" -eq 1 ]; then
  need_cmd cargo-xwin
  CARGO_CMD=(cargo xwin build)
else
  CARGO_CMD=(cargo build)
fi

echo "==> Target: $TARGET"
echo "==> Profile: $PROFILE"
echo "==> Kind: $KIND"
echo "==> Output: $OUT_DIR"

if [ "$USE_CROSS" -eq 0 ] && [ "$USE_XWIN" -eq 0 ]; then
  rustup target add "$TARGET" >/dev/null 2>&1 || true
fi

build_cli() {
  "${CARGO_CMD[@]}" "${PROFILE_FLAG[@]}" --target "$TARGET" -p vw-cli \
    --bin shiju --bin vibe-agent --all-features
}

build_primary_cli() {
  "${CARGO_CMD[@]}" "${PROFILE_FLAG[@]}" --target "$TARGET" -p vw-cli \
    --bin shiju --all-features
}

build_acp() {
  "${CARGO_CMD[@]}" "${PROFILE_FLAG[@]}" --target "$TARGET" -p vw-acp \
    --bin acp --all-features
}

build_desktop() {
  "${CARGO_CMD[@]}" "${PROFILE_FLAG[@]}" --target "$TARGET" -p vw-desktop \
    --bin shi-ju --all-features
  "${CARGO_CMD[@]}" "${PROFILE_FLAG[@]}" --target "$TARGET" -p vw-webview \
    --bin vw-webview --all-features
}

copy_binary() {
  local name="$1"
  copy_binary_as "$name" "$name"
}

copy_binary_as() {
  local source_name="$1"
  local destination_name="$2"
  local src="$BIN_DIR/$source_name$EXE_SUFFIX"
  if [ ! -f "$src" ]; then
    echo "error: expected binary not found: $src" >&2
    exit 1
  fi
  cp "$src" "$STAGE_DIR/$destination_name$EXE_SUFFIX"
}

copy_linux_desktop_files() {
  if [[ "$TARGET" != *"-linux-"* ]]; then
    return
  fi

  mkdir -p \
    "$STAGE_DIR/share/applications" \
    "$STAGE_DIR/share/icons/hicolor/256x256/apps"
  cp release/linux/vibewindow.desktop "$STAGE_DIR/share/applications/shiju.desktop"
  cp assets/logo.png "$STAGE_DIR/share/icons/hicolor/256x256/apps/shiju.png"
}

archive_name() {
  case "$KIND" in
    cli) echo "shiju-cli-$TARGET.$ARCHIVE_EXT" ;;
    acp) echo "shiju-acp-$TARGET.$ARCHIVE_EXT" ;;
    desktop) echo "shiju-desktop-$TARGET.$ARCHIVE_EXT" ;;
    all) echo "shiju-$TARGET.$ARCHIVE_EXT" ;;
  esac
}

case "$KIND" in
  cli)
    build_cli
    ;;
  acp)
    build_acp
    ;;
  desktop)
    build_desktop
    ;;
  all)
    build_primary_cli
    build_acp
    build_desktop
    ;;
esac

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR" "$OUT_DIR"

case "$KIND" in
  cli)
    copy_binary shiju
    copy_binary vibe-agent
    ;;
  acp)
    copy_binary acp
    ;;
  desktop)
    copy_binary shi-ju
    copy_binary vw-webview
    copy_linux_desktop_files
    ;;
  all)
    copy_binary shiju
    copy_binary acp
    copy_binary shi-ju
    copy_binary vw-webview
    copy_linux_desktop_files
    ;;
esac

ARCHIVE_NAME="$(archive_name)"
rm -f "$OUT_DIR/$ARCHIVE_NAME"
if [ "$ARCHIVE_EXT" = "zip" ]; then
  need_cmd zip
  (cd "$STAGE_DIR" && zip -qry "../$ARCHIVE_NAME" .)
else
  (cd "$STAGE_DIR" && tar -czf "../$ARCHIVE_NAME" .)
fi
rm -rf "$STAGE_DIR"

echo "==> Created: $OUT_DIR/$ARCHIVE_NAME"
