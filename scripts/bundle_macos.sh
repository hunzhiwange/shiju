#!/bin/bash
set -euo pipefail

# MODE: app | cli
MODE="${MODE:-app}"
PROFILE="${PROFILE:-release}"
# CI packages the binaries already produced by cargo-dist.
SKIP_BUILD="${SKIP_BUILD:-0}"
OUT_DIR="${OUT_DIR:-dist/macos}"
TARGET="${TARGET:-}"
TARGET_ROOT="${CARGO_TARGET_DIR:-target-dir}"
DESKTOP_BIN_NAME="shi-ju"
CLI_SOURCE_BIN_NAME="shiju"
CLI_BIN_NAME="shiju"
CLI_COMPAT_BIN_NAME="vibe-agent"
ACP_BIN_NAME="acp"
MACOS_ICON_SOURCE="${MACOS_ICON_SOURCE:-assets/logo/icon.icns}"
MACOS_ICON_NAME="ShiJu.icns"
PACKAGE_APP="${PACKAGE_APP:-0}"
CREATE_DMG="${CREATE_DMG:-0}"
INCLUDE_CLI_IN_APP="${INCLUDE_CLI_IN_APP:-1}"
INCLUDE_CLI_COMPAT_IN_APP="${INCLUDE_CLI_COMPAT_IN_APP:-0}"
INCLUDE_ACP_IN_APP="${INCLUDE_ACP_IN_APP:-0}"
MACOS_SIGN_MODE="${MACOS_SIGN_MODE:-adhoc}"
MACOS_SIGN_IDENTITY="${MACOS_SIGN_IDENTITY:-${APPLE_SIGNING_IDENTITY:-}}"
MACOS_NOTARIZE="${MACOS_NOTARIZE:-0}"
NOTARY_KEYCHAIN_PROFILE="${NOTARY_KEYCHAIN_PROFILE:-${SHIJU_NOTARY_PROFILE:-}}"
NOTARY_KEYCHAIN_PATH="${NOTARY_KEYCHAIN_PATH:-${SHIJU_NOTARY_KEYCHAIN:-}}"
NOTARY_KEYCHAIN_PASSWORD_FILE="${NOTARY_KEYCHAIN_PASSWORD_FILE:-${SHIJU_NOTARY_KEYCHAIN_PASSWORD_FILE:-}}"

codesign_with_identity() {
  if [ -n "$NOTARY_KEYCHAIN_PATH" ]; then
    codesign --sign "$MACOS_SIGN_IDENTITY" --keychain "$NOTARY_KEYCHAIN_PATH" "$@"
  else
    codesign --sign "$MACOS_SIGN_IDENTITY" "$@"
  fi
}

prepare_signing_keychain() {
  if [ -n "$NOTARY_KEYCHAIN_PATH" ] && [ ! -f "$NOTARY_KEYCHAIN_PATH" ]; then
    echo "Error: signing/notary keychain not found: $NOTARY_KEYCHAIN_PATH" >&2
    exit 1
  fi
  if [ -n "$NOTARY_KEYCHAIN_PASSWORD_FILE" ]; then
    if [ -z "$NOTARY_KEYCHAIN_PATH" ] || [ ! -r "$NOTARY_KEYCHAIN_PASSWORD_FILE" ]; then
      echo "Error: keychain unlock requires a keychain path and readable password file" >&2
      exit 1
    fi
    # Do not expose the unlock password if the caller enabled shell tracing.
    local restore_xtrace=0
    case "$-" in *x*) restore_xtrace=1; set +x ;; esac
    local unlock_status=0
    security unlock-keychain -p "$(cat "$NOTARY_KEYCHAIN_PASSWORD_FILE")" \
      "$NOTARY_KEYCHAIN_PATH" || unlock_status=$?
    if [ "$restore_xtrace" = "1" ]; then set -x; fi
    if [ "$unlock_status" != "0" ]; then
      echo "Error: unable to unlock signing/notary keychain" >&2
      exit 1
    fi
  fi
}

if [ -n "$TARGET" ]; then
  TARGET_DIR="$TARGET_ROOT/$TARGET/$PROFILE"
  TARGET_SUFFIX="-$TARGET"
else
  TARGET_DIR="$TARGET_ROOT/$PROFILE"
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

# Keep retries local to Apple requests, without rebuilding or resigning artifacts.
retry_apple_request() {
  local operation="$1"
  shift
  local attempt=1
  local max_attempts=5
  local delay=15
  local result_file exit_status notary_status
  result_file="$(mktemp -t vibewindow-apple-request)"

  while true; do
    if "$@" >"$result_file"; then
      cat "$result_file"
      rm -f "$result_file"
      return 0
    else
      exit_status=$?
    fi

    cat "$result_file" >&2
    notary_status="$(/usr/bin/plutil -extract status raw -o - "$result_file" 2>/dev/null || true)"
    case "$notary_status" in
      Invalid|Rejected)
        echo "Error: notarization was not accepted (status: $notary_status)" >&2
        rm -f "$result_file"
        return "$exit_status"
        ;;
    esac
    if [ "$attempt" -ge "$max_attempts" ]; then
      echo "Error: $operation failed after $attempt attempts (exit code: $exit_status)" >&2
      rm -f "$result_file"
      return "$exit_status"
    fi
    echo "Warning: $operation failed (attempt $attempt/$max_attempts); retrying in ${delay}s..." >&2
    sleep "$delay"
    attempt=$((attempt + 1))
    delay=$((delay * 2))
  done
}

preflight_app_signing() {
  case "$MACOS_SIGN_MODE" in
    adhoc)
      if [ "$MACOS_NOTARIZE" = "1" ]; then
        echo "Error: MACOS_NOTARIZE=1 requires MACOS_SIGN_MODE=developer-id" >&2
        exit 1
      fi
      ;;
    developer-id)
      if [ -z "$MACOS_SIGN_IDENTITY" ]; then
        cat >&2 <<'EOF'
Error: MACOS_SIGN_IDENTITY is required for Developer ID signing.

Example:
  MACOS_SIGN_IDENTITY="Developer ID Application: Example Corp (TEAMID)" make macos
EOF
        exit 1
      fi
      case "$MACOS_SIGN_IDENTITY" in
        "Developer ID Application:"*) ;;
        *)
          echo "Error: MACOS_SIGN_IDENTITY must be a Developer ID Application identity" >&2
          exit 1
          ;;
      esac
      prepare_signing_keychain
      if [ -n "$NOTARY_KEYCHAIN_PATH" ]; then
        SIGNING_IDENTITIES="$(security find-identity -v -p codesigning "$NOTARY_KEYCHAIN_PATH")"
      elif ! SIGNING_IDENTITIES="$(security find-identity -v -p codesigning)"; then
        echo "Error: unable to read code-signing identities from the keychain" >&2
        exit 1
      fi
      if ! grep -Fq "$MACOS_SIGN_IDENTITY" <<<"$SIGNING_IDENTITIES"; then
        echo "Error: signing identity not found in the keychain: $MACOS_SIGN_IDENTITY" >&2
        exit 1
      fi
      if [ "$MACOS_NOTARIZE" = "1" ]; then
        if [ -z "$NOTARY_KEYCHAIN_PROFILE" ]; then
          cat >&2 <<'EOF'
Error: NOTARY_KEYCHAIN_PROFILE is required for notarization.

Create it once with:
  xcrun notarytool store-credentials "vibewindow-notary"

Then run:
  NOTARY_KEYCHAIN_PROFILE="vibewindow-notary" MACOS_SIGN_IDENTITY="Developer ID Application: Example Corp (TEAMID)" make macos
EOF
          exit 1
        fi
        if [ "$PACKAGE_APP" != "1" ]; then
          echo "Error: MACOS_NOTARIZE=1 requires PACKAGE_APP=1" >&2
          exit 1
        fi
        command -v xcrun >/dev/null 2>&1 || {
          echo "Error: xcrun is required for notarization" >&2
          exit 1
        }
        local notary_auth_args=(--keychain-profile "$NOTARY_KEYCHAIN_PROFILE")
        if [ -n "$NOTARY_KEYCHAIN_PATH" ]; then
          notary_auth_args+=(--keychain "$NOTARY_KEYCHAIN_PATH")
        fi
        if ! retry_apple_request "Notarization credential check" \
          xcrun notarytool history "${notary_auth_args[@]}" --output-format json >/dev/null; then
          echo "Error: notarization credentials are unavailable; check the keychain profile and Apple connectivity" >&2
          exit 1
        fi
      fi
      ;;
    *)
      echo "Error: unsupported MACOS_SIGN_MODE: $MACOS_SIGN_MODE" >&2
      exit 1
      ;;
  esac
}

submit_for_notarization() {
  local artifact_path="$1"
  local notary_result
  local notary_status
  local submission_id
  local notary_auth_args=(--keychain-profile "$NOTARY_KEYCHAIN_PROFILE")

  prepare_signing_keychain

  if [ -n "$NOTARY_KEYCHAIN_PATH" ]; then
    notary_auth_args+=(--keychain "$NOTARY_KEYCHAIN_PATH")
  fi

  notary_result="$(mktemp -t vibewindow-notary-result)"
  echo "Submitting $(basename "$artifact_path") for notarization..."
  if ! retry_apple_request "Notarization submission" xcrun notarytool submit \
    "$artifact_path" \
    "${notary_auth_args[@]}" \
    --no-wait \
    --output-format json >"$notary_result"; then
    cat "$notary_result" >&2
    rm -f "$notary_result"
    exit 1
  fi

  cat "$notary_result"
  submission_id="$(/usr/bin/plutil -extract id raw -o - "$notary_result" 2>/dev/null || true)"
  if [ -z "$submission_id" ]; then
    echo "Error: notarization submission did not return a submission ID" >&2
    rm -f "$notary_result"
    exit 1
  fi

  # A network failure while polling must not upload the same archive again.
  echo "Waiting for notarization submission $submission_id..."
  if ! retry_apple_request "Notarization wait ($submission_id)" \
    xcrun notarytool wait "$submission_id" "${notary_auth_args[@]}" \
    --timeout 10m --output-format json >"$notary_result"; then
    cat "$notary_result" >&2
    rm -f "$notary_result"
    exit 1
  fi

  cat "$notary_result"
  notary_status="$(/usr/bin/plutil -extract status raw -o - "$notary_result" 2>/dev/null || true)"
  rm -f "$notary_result"
  if [ "$notary_status" != "Accepted" ]; then
    echo "Error: notarization was not accepted (status: ${notary_status:-unknown})" >&2
    exit 1
  fi
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
  echo "Building CLI (${CLI_BIN_NAME})..."
  cargo_build -p vw-cli --bin "$CLI_SOURCE_BIN_NAME" --bin "$CLI_COMPAT_BIN_NAME" \
    --no-default-features --features channel-lark
  mkdir -p "$OUT_DIR"
  SRC_BIN="$TARGET_DIR/${CLI_SOURCE_BIN_NAME}"
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
  ZIP_NAME="ShiJu-CLI-macos${TARGET_SUFFIX}-${PROFILE}.zip"
  (cd "$OUT_DIR" && /usr/bin/zip -qry "$ZIP_NAME" "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME")
  echo "$OUT_DIR/$ZIP_NAME"
  exit 0
fi

preflight_app_signing

BUNDLE_DIR="$TARGET_DIR/bundle/osx"
APP_BUNDLE="$BUNDLE_DIR/ShiJu.app"

need_cargo_bundle

if [ "$SKIP_BUILD" = "1" ]; then
  required_binaries=("$DESKTOP_BIN_NAME" vw-webview)
  if [ "$INCLUDE_CLI_IN_APP" = "1" ]; then
    required_binaries+=("$CLI_SOURCE_BIN_NAME")
    if [ "$INCLUDE_CLI_COMPAT_IN_APP" = "1" ]; then
      required_binaries+=("$CLI_COMPAT_BIN_NAME")
    fi
  fi
  if [ "$INCLUDE_ACP_IN_APP" = "1" ]; then
    required_binaries+=("$ACP_BIN_NAME")
  fi
  for binary_name in "${required_binaries[@]}"; do
    if [ ! -x "$TARGET_DIR/$binary_name" ]; then
      echo "Error: expected prebuilt executable not found: $TARGET_DIR/$binary_name" >&2
      exit 1
    fi
  done
  echo "Bundling prebuilt application from $TARGET_DIR..."
  CARGO_BUNDLE_SKIP_BUILD=1 cargo_bundle -p vw-desktop --bin shi-ju --format osx
else
  echo "Bundling main application..."
  cargo_bundle -p vw-desktop --bin shi-ju

  echo "Building helper binary..."
  cargo_build -p vw-webview --bin vw-webview
  if [ "$INCLUDE_CLI_IN_APP" = "1" ]; then
    echo "Building CLI binary (to include in app bundle)..."
    cargo_build -p vw-cli --bin "$CLI_SOURCE_BIN_NAME" --no-default-features --features channel-lark
    if [ "$INCLUDE_CLI_COMPAT_IN_APP" = "1" ]; then
      echo "Building CLI compatibility binary (to include in app bundle)..."
      cargo_build -p vw-cli --bin "$CLI_COMPAT_BIN_NAME" --no-default-features --features channel-lark
    fi
  fi
  if [ "$INCLUDE_ACP_IN_APP" = "1" ]; then
    echo "Building ACP binary (to include in app bundle)..."
    cargo_build -p vw-acp --bin "$ACP_BIN_NAME"
  fi
fi

HELPER_BIN="$TARGET_DIR/vw-webview"
DEST_DIR="$APP_BUNDLE/Contents/MacOS"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "Error: App bundle not found at $APP_BUNDLE"
    exit 1
fi

if [ -x /usr/libexec/PlistBuddy ]; then
  /usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $DESKTOP_BIN_NAME" \
    "$APP_BUNDLE/Contents/Info.plist"
fi

echo "Copying helper binary to bundle..."
cp "$HELPER_BIN" "$DEST_DIR/"
if [ "$INCLUDE_CLI_IN_APP" = "1" ]; then
  echo "Copying CLI binary to bundle..."
  AGENT_BIN="$TARGET_DIR/${CLI_SOURCE_BIN_NAME}"
  cp "$AGENT_BIN" "$DEST_DIR/${CLI_BIN_NAME}"
  if [ "$INCLUDE_CLI_COMPAT_IN_APP" = "1" ]; then
    cp "$TARGET_DIR/${CLI_COMPAT_BIN_NAME}" "$DEST_DIR/${CLI_COMPAT_BIN_NAME}"
  else
    rm -f "$DEST_DIR/${CLI_COMPAT_BIN_NAME}"
  fi
fi
if [ "$INCLUDE_ACP_IN_APP" = "1" ]; then
  echo "Copying ACP binary to bundle..."
  cp "$TARGET_DIR/${ACP_BIN_NAME}" "$DEST_DIR/${ACP_BIN_NAME}"
fi

ensure_app_icon

if [ "$MACOS_SIGN_MODE" = "developer-id" ]; then
  prepare_signing_keychain
  echo "Signing embedded binaries with Developer ID..."
  for binary_name in vw-webview "$CLI_BIN_NAME" "$CLI_COMPAT_BIN_NAME" "$ACP_BIN_NAME"; do
    binary_path="$DEST_DIR/$binary_name"
    if [ -f "$binary_path" ]; then
      codesign_with_identity \
        --force \
        --timestamp \
        --options runtime \
        "$binary_path"
    fi
  done

  echo "Signing app bundle with Developer ID..."
  codesign_with_identity \
    --force \
    --timestamp \
    --options runtime \
    "$APP_BUNDLE"
else
  echo "Applying ad-hoc signature for local development..."
  codesign --force --deep --sign - "$APP_BUNDLE"
fi

echo "Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"

echo "Done! App bundle is ready at: $APP_BUNDLE"
echo "You can run it with: open \"$APP_BUNDLE\""

if [ "$PACKAGE_APP" = "1" ]; then
  mkdir -p "$OUT_DIR"
  ZIP_NAME="ShiJu-macos${TARGET_SUFFIX}-${PROFILE}.zip"
  rm -f "$OUT_DIR/$ZIP_NAME"
  ditto -c -k --keepParent "$APP_BUNDLE" "$OUT_DIR/$ZIP_NAME"
  echo "Zip package: $OUT_DIR/$ZIP_NAME"
fi

if [ "$MACOS_NOTARIZE" = "1" ]; then
  ZIP_PATH="$OUT_DIR/$ZIP_NAME"
  submit_for_notarization "$ZIP_PATH"

  echo "Stapling notarization ticket to app bundle..."
  retry_apple_request "App ticket stapling" xcrun stapler staple "$APP_BUNDLE"
  xcrun stapler validate "$APP_BUNDLE"

  echo "Repacking stapled app bundle..."
  rm -f "$ZIP_PATH"
  ditto -c -k --keepParent "$APP_BUNDLE" "$ZIP_PATH"

  echo "Validating Gatekeeper acceptance..."
  spctl --assess --type execute --verbose=4 "$APP_BUNDLE"
  echo "Signed and notarized zip package: $ZIP_PATH"
fi

if [ "$CREATE_DMG" = "1" ]; then
  if ! command -v hdiutil >/dev/null 2>&1; then
    echo "Error: hdiutil is required to create a DMG on macOS"
    exit 1
  fi
  mkdir -p "$OUT_DIR"
  DMG_NAME="ShiJu-macos${TARGET_SUFFIX}-${PROFILE}.dmg"
  DMG_STAGE="$OUT_DIR/dmg-stage"
  rm -f "$OUT_DIR/$DMG_NAME"
  rm -rf "$DMG_STAGE"
  mkdir -p "$DMG_STAGE"
  ditto "$APP_BUNDLE" "$DMG_STAGE/$(basename "$APP_BUNDLE")"
  ln -s /Applications "$DMG_STAGE/Applications"
  hdiutil create -volname "ShiJu" -srcfolder "$DMG_STAGE" -ov -format UDZO "$OUT_DIR/$DMG_NAME"
  echo "DMG package: $OUT_DIR/$DMG_NAME"

  if [ "$MACOS_NOTARIZE" = "1" ]; then
    DMG_PATH="$OUT_DIR/$DMG_NAME"
    prepare_signing_keychain
    codesign_with_identity --force --timestamp "$DMG_PATH"
    codesign --verify --strict --verbose=2 "$DMG_PATH"
    submit_for_notarization "$DMG_PATH"
    echo "Stapling notarization ticket to disk image..."
    retry_apple_request "DMG ticket stapling" xcrun stapler staple "$DMG_PATH"
    xcrun stapler validate "$DMG_PATH"
    echo "Notarized DMG package: $DMG_PATH"
  fi
fi
