#!/bin/bash
set -euo pipefail

# MODE: app | cli
MODE="${MODE:-app}"
PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-dist/macos}"
TARGET="${TARGET:-}"
CLI_BIN_NAME="vibewindow"
CLI_COMPAT_BIN_NAME="vibe-agent"
ACP_BIN_NAME="acp"
MACOS_ICON_SOURCE="${MACOS_ICON_SOURCE:-assets/logo/icon.icns}"
MACOS_ICON_NAME="VibeWindow.icns"
PACKAGE_APP="${PACKAGE_APP:-0}"
CREATE_DMG="${CREATE_DMG:-0}"

if [ -n "$TARGET" ]; then
  TARGET_DIR="target/$TARGET/$PROFILE"
  TARGET_SUFFIX="-$TARGET"
else
  TARGET_DIR="target/$PROFILE"
  TARGET_SUFFIX=""
fi

if [ "$PROFILE" = "release" ]; then
  CARGO_PROFILE_ARGS=(--release)
else
  CARGO_PROFILE_ARGS=(--profile "$PROFILE")
fi

cargo_build() {
  if [ -n "$TARGET" ]; then
    cargo build "${CARGO_PROFILE_ARGS[@]}" --target "$TARGET" "$@"
  else
    cargo build "${CARGO_PROFILE_ARGS[@]}" "$@"
  fi
}

cargo_bundle() {
  if [ -n "$TARGET" ]; then
    cargo bundle "${CARGO_PROFILE_ARGS[@]}" --target "$TARGET" "$@"
  else
    cargo bundle "${CARGO_PROFILE_ARGS[@]}" "$@"
  fi
}

need_cargo_bundle() {
  if cargo bundle --help >/dev/null 2>&1; then
    return 0
  fi

  cat >&2 <<'EOF'
Error: cargo-bundle is required to build the macOS .app package.

Install it with:
  cargo install cargo-bundle --locked

Then retry:
  make release-macos-dmg
EOF
  exit 1
}

ensure_app_icon() {
  if [ ! -f "$MACOS_ICON_SOURCE" ]; then
    echo "Error: macOS icon not found at $MACOS_ICON_SOURCE"
    exit 1
  fi

  local resources_dir="$APP_BUNDLE/Contents/Resources"
  local plist="$APP_BUNDLE/Contents/Info.plist"
  mkdir -p "$resources_dir"
  cp "$MACOS_ICON_SOURCE" "$resources_dir/$MACOS_ICON_NAME"

  if [ -f "$plist" ] && [ -x /usr/libexec/PlistBuddy ]; then
    /usr/libexec/PlistBuddy -c "Set :CFBundleIconFile $MACOS_ICON_NAME" "$plist" 2>/dev/null \
      || /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string $MACOS_ICON_NAME" "$plist"
  fi
}

if [ "$MODE" = "cli" ]; then
  if [ ! -f "$TARGET_DIR/${CLI_BIN_NAME}" ] || [ ! -f "$TARGET_DIR/${CLI_COMPAT_BIN_NAME}" ]; then
    echo "Building CLI (${CLI_BIN_NAME})..."
    cargo_build -p vw-cli --bin "$CLI_BIN_NAME" --bin "$CLI_COMPAT_BIN_NAME" --all-features
  fi
  mkdir -p "$OUT_DIR"
  SRC_BIN="$TARGET_DIR/${CLI_BIN_NAME}"
  COMPAT_SRC_BIN="$TARGET_DIR/${CLI_COMPAT_BIN_NAME}"
  if [ ! -f "$SRC_BIN" ]; then
    echo "Error: CLI binary not found at $SRC_BIN"
    exit 1
  fi
  if [ ! -f "$COMPAT_SRC_BIN" ]; then
    echo "Error: CLI compatibility binary not found at $COMPAT_SRC_BIN"
    exit 1
  fi
  cp "$SRC_BIN" "$OUT_DIR/${CLI_BIN_NAME}"
  cp "$COMPAT_SRC_BIN" "$OUT_DIR/${CLI_COMPAT_BIN_NAME}"
  ZIP_NAME="VibeWindow-CLI-macos${TARGET_SUFFIX}-${PROFILE}.zip"
  (cd "$OUT_DIR" && /usr/bin/zip -qry "$ZIP_NAME" "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME")
  echo "$OUT_DIR/$ZIP_NAME"
  exit 0
fi

BUNDLE_DIR="$TARGET_DIR/bundle/osx"
APP_BUNDLE_PRIMARY="$BUNDLE_DIR/VibeWindow.app"
APP_BUNDLE_FALLBACK="$BUNDLE_DIR/vibe-window.app"

need_cargo_bundle

if [ ! -d "$APP_BUNDLE_PRIMARY" ] && [ ! -d "$APP_BUNDLE_FALLBACK" ]; then
  echo "Bundling main application..."
  cargo_bundle -p vw-desktop --bin vibe-window --all-features
fi

if [ ! -f "$TARGET_DIR/vw-webview" ]; then
  echo "Building helper binary..."
  cargo_build -p vw-webview --bin vw-webview --all-features
fi
if [ "${INCLUDE_CLI_IN_APP:-0}" = "1" ]; then
  if [ ! -f "$TARGET_DIR/${CLI_BIN_NAME}" ]; then
    echo "Building CLI binary (to include in app bundle)..."
    cargo_build -p vw-cli --bin "$CLI_BIN_NAME" --all-features
  fi
  if [ "${INCLUDE_CLI_COMPAT_IN_APP:-0}" = "1" ] && [ ! -f "$TARGET_DIR/${CLI_COMPAT_BIN_NAME}" ]; then
    echo "Building CLI compatibility binary (to include in app bundle)..."
    cargo_build -p vw-cli --bin "$CLI_COMPAT_BIN_NAME" --all-features
  fi
fi
if [ "${INCLUDE_ACP_IN_APP:-0}" = "1" ]; then
  if [ ! -f "$TARGET_DIR/${ACP_BIN_NAME}" ]; then
    echo "Building ACP binary (to include in app bundle)..."
    cargo_build -p vw-acp --bin "$ACP_BIN_NAME" --all-features
  fi
fi

HELPER_BIN="$TARGET_DIR/vw-webview"
if [ -d "$APP_BUNDLE_PRIMARY" ]; then
    APP_BUNDLE="$APP_BUNDLE_PRIMARY"
elif [ -d "$APP_BUNDLE_FALLBACK" ]; then
    APP_BUNDLE="$APP_BUNDLE_FALLBACK"
else
    echo "Error: App bundle not found at $APP_BUNDLE_PRIMARY or $APP_BUNDLE_FALLBACK"
    exit 1
fi
DEST_DIR="$APP_BUNDLE/Contents/MacOS"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi

echo "Copying helper binary to bundle..."
cp "$HELPER_BIN" "$DEST_DIR/"
if [ "${INCLUDE_CLI_IN_APP:-0}" = "1" ]; then
  echo "Copying CLI binary to bundle..."
  AGENT_BIN="$TARGET_DIR/${CLI_BIN_NAME}"
  cp "$AGENT_BIN" "$DEST_DIR/${CLI_BIN_NAME}"
  if [ "${INCLUDE_CLI_COMPAT_IN_APP:-0}" = "1" ]; then
    cp "$TARGET_DIR/${CLI_COMPAT_BIN_NAME}" "$DEST_DIR/${CLI_COMPAT_BIN_NAME}"
  else
    rm -f "$DEST_DIR/${CLI_COMPAT_BIN_NAME}"
  fi
fi
if [ "${INCLUDE_ACP_IN_APP:-0}" = "1" ]; then
  echo "Copying ACP binary to bundle..."
  cp "$TARGET_DIR/${ACP_BIN_NAME}" "$DEST_DIR/${ACP_BIN_NAME}"
fi

ensure_app_icon

ENTITLEMENTS="scripts/macos-entitlements.plist"
echo "Code signing app bundle without sandbox entitlements..."
codesign --force --deep --sign - "$APP_BUNDLE"
echo "Verifying code signature..."
codesign --verify --deep --strict "$APP_BUNDLE" || true

echo "Done! App bundle is ready at: $APP_BUNDLE"
echo "You can run it with: open \"$APP_BUNDLE\""

if [ "$PACKAGE_APP" = "1" ]; then
  mkdir -p "$OUT_DIR"
  ZIP_NAME="VibeWindow-macos${TARGET_SUFFIX}-${PROFILE}.zip"
  rm -f "$OUT_DIR/$ZIP_NAME"
  ditto -c -k --keepParent "$APP_BUNDLE" "$OUT_DIR/$ZIP_NAME"
  echo "Zip package: $OUT_DIR/$ZIP_NAME"
fi

if [ "$CREATE_DMG" = "1" ]; then
  if ! command -v hdiutil >/dev/null 2>&1; then
    echo "Error: hdiutil is required to create a DMG on macOS"
    exit 1
  fi
  mkdir -p "$OUT_DIR"
  DMG_NAME="VibeWindow-macos${TARGET_SUFFIX}-${PROFILE}.dmg"
  DMG_STAGE="$OUT_DIR/dmg-stage"
  rm -f "$OUT_DIR/$DMG_NAME"
  rm -rf "$DMG_STAGE"
  mkdir -p "$DMG_STAGE"
  ditto "$APP_BUNDLE" "$DMG_STAGE/$(basename "$APP_BUNDLE")"
  ln -s /Applications "$DMG_STAGE/Applications"
  hdiutil create -volname "VibeWindow" -srcfolder "$DMG_STAGE" -ov -format UDZO "$OUT_DIR/$DMG_NAME"
  echo "DMG package: $OUT_DIR/$DMG_NAME"
fi
