#!/bin/bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-msvc}"
PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-dist/windows}"
MODE="${MODE:-app}"
if [[ "$PROFILE" == "release" || "$PROFILE" == "dist" ]]; then
  export VIBEWINDOW_RELEASE_BUILD=1
else
  export VIBEWINDOW_RELEASE_BUILD=0
fi
CREATE_MSI="${CREATE_MSI:-0}"
CREATE_ZIP="${CREATE_ZIP:-1}"
CLI_BIN_NAME="vibewindow.exe"
CLI_COMPAT_BIN_NAME="vibe-agent.exe"
ACP_BIN_NAME="acp.exe"
WINDOWS_ICON_SOURCE="${WINDOWS_ICON_SOURCE:-assets/logo.png}"
WINDOWS_ICON_NAME="VibeWindow.ico"
GATEWAY_LAUNCHER_NAME="Start VibeWindow Gateway.cmd"

if [ "$PROFILE" = "release" ]; then
  PROFILE_ARGS=(--release)
else
  PROFILE_ARGS=(--profile "$PROFILE")
fi

python_bin() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
  elif command -v python >/dev/null 2>&1; then
    command -v python
  else
    echo "Error: python is required to package Windows artifacts" >&2
    exit 1
  fi
}

generate_windows_icon() {
  local dst="$OUT_DIR/$WINDOWS_ICON_NAME"
  if [ ! -f "$WINDOWS_ICON_SOURCE" ]; then
    echo "Error: Windows icon source not found at $WINDOWS_ICON_SOURCE" >&2
    exit 1
  fi

  "$(python_bin)" - "$WINDOWS_ICON_SOURCE" "$dst" <<'PY'
import struct
import sys
from pathlib import Path

src = Path(sys.argv[1])
dst = Path(sys.argv[2])
data = src.read_bytes()
if data[:8] != b"\x89PNG\r\n\x1a\n":
    raise SystemExit(f"icon source must be a PNG: {src}")

dst.parent.mkdir(parents=True, exist_ok=True)
header = struct.pack("<HHH", 0, 1, 1)
entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(data), 22)
dst.write_bytes(header + entry + data)
PY
}

zip_from_out_dir() {
  local zip_name="$1"
  shift
  "$(python_bin)" - "$OUT_DIR" "$zip_name" "$@" <<'PY'
import sys
import zipfile
from pathlib import Path

out_dir = Path(sys.argv[1])
zip_name = sys.argv[2]
files = sys.argv[3:]
zip_path = out_dir / zip_name
if zip_path.exists():
    zip_path.unlink()
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as archive:
    for name in files:
        archive.write(out_dir / name, name)
PY
}

write_gateway_launcher() {
  "$(python_bin)" - "$OUT_DIR/$GATEWAY_LAUNCHER_NAME" <<'PY'
import sys
from pathlib import Path

dst = Path(sys.argv[1])
dst.write_text(
    '@echo off\r\n'
    "powershell.exe -NoExit -ExecutionPolicy Bypass -Command \"Set-Location -LiteralPath '%~dp0'; & '.\\vibewindow.exe' gateway\"\r\n",
    encoding="utf-8",
)
PY
}

render_wix_template() {
  local template="release/windows/vibewindow.wxs"
  local dst="$OUT_DIR/vibewindow.wxs"
  if [ ! -f "$template" ]; then
    echo "Error: WiX template not found at $template" >&2
    exit 1
  fi

  "$(python_bin)" - "$template" "$dst" <<'PY'
import re
import sys
from pathlib import Path

template = Path(sys.argv[1])
dst = Path(sys.argv[2])
root = Path.cwd()
cargo = (root / "crates" / "vw-desktop" / "Cargo.toml").read_text(encoding="utf-8")
match = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.MULTILINE)
if not match:
    raise SystemExit("failed to read vw-desktop package version")

text = template.read_text(encoding="utf-8")
text = text.replace("{{ .Version }}", match.group(1))
text = text.replace("{{ .Binary }}", "vibe-window")
dst.write_text(text, encoding="utf-8")
PY
}

build_msi() {
  if ! command -v wix >/dev/null 2>&1; then
    echo "Error: WiX Toolset 'wix' command is required to build MSI" >&2
    exit 1
  fi

  render_wix_template
  local msi_name="VibeWindow-$TARGET-$PROFILE.msi"
  rm -f "$OUT_DIR/$msi_name"
  (cd "$OUT_DIR" && wix build vibewindow.wxs -out "$msi_name")
  echo "$OUT_DIR/$msi_name"
}

missing_binaries() {
  for binary in "$@"; do
    if [ ! -f "$BIN_DIR/$binary" ]; then
      return 0
    fi
  done
  return 1
}

build_cli_binaries() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibewindow --bin vibe-agent --all-features
  else
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibewindow --bin vibe-agent --all-features
  fi
}

build_cli_binary() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibewindow --all-features
  else
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibewindow --all-features
  fi
}

build_cli_compat_binary() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibe-agent --all-features
  else
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-cli --bin vibe-agent --all-features
  fi
}

build_app_binaries() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-desktop --bin vibe-window --all-features
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-webview --bin vw-webview --all-features
  else
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-desktop --bin vibe-window --all-features
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-webview --bin vw-webview --all-features
  fi
}

build_acp_binary() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-acp --bin acp --all-features
  else
    cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" -p vw-acp --bin acp --all-features
  fi
}

BIN_DIR="target/$TARGET/$PROFILE"

if [ "$MODE" = "cli" ]; then
  if missing_binaries "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME"; then
    build_cli_binaries
  fi
  mkdir -p "$OUT_DIR"
  generate_windows_icon
  cp "$BIN_DIR/$CLI_BIN_NAME" "$OUT_DIR/$CLI_BIN_NAME"
  cp "$BIN_DIR/$CLI_COMPAT_BIN_NAME" "$OUT_DIR/$CLI_COMPAT_BIN_NAME"
  ZIP_NAME="VibeWindow-CLI-$TARGET-$PROFILE.zip"
  zip_from_out_dir "$ZIP_NAME" "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME" "$WINDOWS_ICON_NAME"
  echo "$OUT_DIR/$ZIP_NAME"
  exit 0
fi

if missing_binaries "vibe-window.exe" "vw-webview.exe"; then
  build_app_binaries
fi

mkdir -p "$OUT_DIR"
generate_windows_icon
write_gateway_launcher
cp "$BIN_DIR/vibe-window.exe" "$OUT_DIR/"
cp "$BIN_DIR/vw-webview.exe" "$OUT_DIR/"
ZIP_FILES=("vibe-window.exe" "vw-webview.exe" "$WINDOWS_ICON_NAME" "$GATEWAY_LAUNCHER_NAME")
if [ "${INCLUDE_CLI_IN_APP:-0}" = "1" ]; then
  if missing_binaries "$CLI_BIN_NAME"; then
    build_cli_binary
  fi
  cp "$BIN_DIR/$CLI_BIN_NAME" "$OUT_DIR/$CLI_BIN_NAME"
  ZIP_FILES+=("$CLI_BIN_NAME")
  if [ "${INCLUDE_CLI_COMPAT_IN_APP:-0}" = "1" ]; then
    if missing_binaries "$CLI_COMPAT_BIN_NAME"; then
      build_cli_compat_binary
    fi
    cp "$BIN_DIR/$CLI_COMPAT_BIN_NAME" "$OUT_DIR/$CLI_COMPAT_BIN_NAME"
    ZIP_FILES+=("$CLI_COMPAT_BIN_NAME")
  fi
fi
if [ "${INCLUDE_ACP_IN_APP:-0}" = "1" ]; then
  if missing_binaries "$ACP_BIN_NAME"; then
    build_acp_binary
  fi
  cp "$BIN_DIR/$ACP_BIN_NAME" "$OUT_DIR/$ACP_BIN_NAME"
  ZIP_FILES+=("$ACP_BIN_NAME")
fi

if [ "$CREATE_ZIP" = "1" ]; then
  ZIP_NAME="VibeWindow-$TARGET-$PROFILE.zip"
  zip_from_out_dir "$ZIP_NAME" "${ZIP_FILES[@]}"
  echo "$OUT_DIR/$ZIP_NAME"
fi

if [ "$CREATE_MSI" = "1" ]; then
  build_msi
fi
