#!/bin/bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-msvc}"
PROFILE="${PROFILE:-release}"
TARGET_ROOT="${CARGO_TARGET_DIR:-target-dir}"
OUT_DIR="${OUT_DIR:-dist/windows}"
MODE="${MODE:-app}"
CREATE_MSI="${CREATE_MSI:-0}"
CREATE_ZIP="${CREATE_ZIP:-1}"
DESKTOP_SOURCE_BIN_NAME="shi-ju.exe"
DESKTOP_BIN_NAME="shi-ju.exe"
CLI_SOURCE_BIN_NAME="shiju.exe"
CLI_BIN_NAME="shiju.exe"
CLI_COMPAT_BIN_NAME="vibe-agent.exe"
ACP_BIN_NAME="acp.exe"
WINDOWS_ICON_SOURCE="${WINDOWS_ICON_SOURCE:-assets/logo.png}"
WINDOWS_ICON_NAME="ShiJu.ico"
SERVICE_LAUNCHER_NAME="Start ShiJu Service.cmd"
INCLUDE_CLI_IN_APP="${INCLUDE_CLI_IN_APP:-1}"
INCLUDE_ACP_IN_APP="${INCLUDE_ACP_IN_APP:-1}"

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

write_service_launcher() {
  "$(python_bin)" - "$OUT_DIR/$SERVICE_LAUNCHER_NAME" <<'PY'
import sys
from pathlib import Path

dst = Path(sys.argv[1])
dst.write_bytes((
    '@echo off\r\n'
    'if not exist "%~dp0shiju.exe" (\r\n'
    '  echo Missing shiju.exe. Extract the complete ShiJu package into one folder.\r\n'
    '  pause\r\n'
    '  exit /b 1\r\n'
    ')\r\n'
    '"%~dp0shiju.exe" service start\r\n'
    'if not errorlevel 1 exit /b 0\r\n'
    '"%~dp0shiju.exe" service install\r\n'
    'if errorlevel 1 goto failed\r\n'
    '"%~dp0shiju.exe" service start\r\n'
    'if errorlevel 1 goto failed\r\n'
    'exit /b 0\r\n'
    ':failed\r\n'
    'echo Failed to start ShiJu service. See the error above.\r\n'
    'pause\r\n'
    'exit /b 1\r\n'
).encode("utf-8"))
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
text = text.replace("{{ .Binary }}", "shi-ju")
dst.write_text(text, encoding="utf-8")

# WixUI ships a placeholder EULA; display the repository's actual license instead.
license_text = (root / "LICENSE").read_text(encoding="utf-8")
license_text = license_text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")
license_rtf = "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Segoe UI;}}\\f0\\fs18\n"
license_rtf += license_text.replace("\n", "\\par\n") + "}"
(dst.parent / "License.rtf").write_text(license_rtf, encoding="ascii")
PY
}

build_msi() {
  if ! command -v wix >/dev/null 2>&1; then
    echo "Error: WiX Toolset 'wix' command is required to build MSI" >&2
    exit 1
  fi

  local wix_arch
  case "$TARGET" in
    x86_64-pc-windows-msvc) wix_arch=x64 ;;
    aarch64-pc-windows-msvc) wix_arch=arm64 ;;
    *)
      echo "Error: unsupported Windows MSI target: $TARGET" >&2
      return 1
      ;;
  esac

  render_wix_template
  local msi_name="ShiJu-$TARGET-$PROFILE.msi"
  rm -f "$OUT_DIR/$msi_name"
  (cd "$OUT_DIR" && wix build vibewindow.wxs -arch "$wix_arch" -ext WixToolset.UI.wixext -out "$msi_name")
  echo "$OUT_DIR/$msi_name"
}

# Plain cargo cannot configure the Windows SDK and C compiler on Unix hosts.
build_windows_package() {
  if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build "${PROFILE_ARGS[@]}" --target "$TARGET" "$@"
    return
  fi

  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      cargo build "${PROFILE_ARGS[@]}" --target "$TARGET" "$@"
      ;;
    *)
      echo "Error: cargo-xwin is required to build Windows targets on this host." >&2
      echo "Install it with: cargo install cargo-xwin --locked" >&2
      echo "See scripts/BUILDING.md for LLVM and Windows target prerequisites." >&2
      return 1
      ;;
  esac
}

build_cli_binaries() {
  build_windows_package -p vw-cli --bin shiju --bin vibe-agent --all-features
}

build_cli_binary() {
  build_windows_package -p vw-cli --bin shiju --all-features
}

build_cli_compat_binary() {
  build_windows_package -p vw-cli --bin vibe-agent --all-features
}

build_app_binaries() {
  build_windows_package -p vw-desktop --bin shi-ju --all-features
  build_windows_package -p vw-webview --bin vw-webview --all-features
}

build_acp_binary() {
  build_windows_package -p vw-acp --bin acp --all-features
}

# Parse the workflow before Cargo runs so edits during compilation cannot shift reads.
main() {
  BIN_DIR="$TARGET_ROOT/$TARGET/$PROFILE"

  if [ "$MODE" = "cli" ]; then
    build_cli_binaries
    mkdir -p "$OUT_DIR"
    generate_windows_icon
    cp "$BIN_DIR/$CLI_SOURCE_BIN_NAME" "$OUT_DIR/$CLI_BIN_NAME"
    cp "$BIN_DIR/$CLI_COMPAT_BIN_NAME" "$OUT_DIR/$CLI_COMPAT_BIN_NAME"
    ZIP_NAME="ShiJu-CLI-$TARGET-$PROFILE.zip"
    zip_from_out_dir "$ZIP_NAME" "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME" "$WINDOWS_ICON_NAME"
    echo "$OUT_DIR/$ZIP_NAME"
    exit 0
  fi

  if [ "$INCLUDE_CLI_IN_APP" != "1" ] || [ "$INCLUDE_ACP_IN_APP" != "1" ]; then
    echo "Error: desktop packages require bundled shiju.exe and acp.exe; INCLUDE_CLI_IN_APP and INCLUDE_ACP_IN_APP must be 1." >&2
    exit 1
  fi

  # Always ask Cargo to check source freshness; existing files can be stale.
  build_app_binaries

  mkdir -p "$OUT_DIR"
  generate_windows_icon
  write_service_launcher
  cp "$BIN_DIR/$DESKTOP_SOURCE_BIN_NAME" "$OUT_DIR/$DESKTOP_BIN_NAME"
  cp "$BIN_DIR/vw-webview.exe" "$OUT_DIR/"
  ZIP_FILES=("$DESKTOP_BIN_NAME" "vw-webview.exe" "$WINDOWS_ICON_NAME" "$SERVICE_LAUNCHER_NAME")
  if [ "${INCLUDE_CLI_IN_APP}" = "1" ]; then
    build_cli_binary
    cp "$BIN_DIR/$CLI_SOURCE_BIN_NAME" "$OUT_DIR/$CLI_BIN_NAME"
    ZIP_FILES+=("$CLI_BIN_NAME")
    if [ "${INCLUDE_CLI_COMPAT_IN_APP:-0}" = "1" ]; then
      build_cli_compat_binary
      cp "$BIN_DIR/$CLI_COMPAT_BIN_NAME" "$OUT_DIR/$CLI_COMPAT_BIN_NAME"
      ZIP_FILES+=("$CLI_COMPAT_BIN_NAME")
    fi
  fi
  if [ "${INCLUDE_ACP_IN_APP}" = "1" ]; then
    build_acp_binary
    cp "$BIN_DIR/$ACP_BIN_NAME" "$OUT_DIR/$ACP_BIN_NAME"
    ZIP_FILES+=("$ACP_BIN_NAME")
  fi

  if [ "$CREATE_ZIP" = "1" ]; then
    ZIP_NAME="ShiJu-$TARGET-$PROFILE.zip"
    zip_from_out_dir "$ZIP_NAME" "${ZIP_FILES[@]}"
    echo "$OUT_DIR/$ZIP_NAME"
  fi

  if [ "$CREATE_MSI" = "1" ]; then
    build_msi
  fi
}

# Exit in the same parsed list instead of reading a possibly modified script again.
main "$@"; exit
