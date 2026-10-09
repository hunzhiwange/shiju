# Build & Package (macOS / Windows)

This document is located under `scripts/` for daily development and release.

## 0. Prerequisites

- Install Rust (recommend via rustup):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Navigate to project root:
  ```bash
  cd /path/to/vibe-window
  ```
- Install the parallel test runner:
  ```bash
  cargo install cargo-nextest --locked
  ```

## 1. macOS (Native Build + .app Bundle)

### 1.1 Native Build (Binary)
```bash
cargo build
make test
```

Release:
```bash
cargo build --release
```

### 1.2 macOS .app Bundle (including vw-webview helper)

Install `cargo-bundle` (provides `cargo bundle` command):
```bash
cargo install cargo-bundle
```

For local development, run:

```bash
make macos-dev
```

Output defaults to:
- `target/release/bundle/osx/ShiJu.app`

### 1.3 Signed and Notarized macOS Package

Public distribution requires a `Developer ID Application` certificate in the
login keychain. Verify its exact name with:

```bash
security find-identity -v -p codesigning
```

Store the notarization credentials in the keychain once:

```bash
xcrun notarytool store-credentials "vibewindow-notary"
```

Build, sign embedded binaries and the app bundle, notarize it, staple the
ticket, and produce the final zip:

```bash
MACOS_SIGN_IDENTITY="Developer ID Application: Example Corp (TEAMID)" \
NOTARY_KEYCHAIN_PROFILE="vibewindow-notary" \
make macos
```

Output defaults to:
- `dist/macos/ShiJu-macos-release.zip`

The Developer ID build enables Hardened Runtime without App Sandbox. Outbound
networking remains available, and credentials stay in the macOS keychain.

### 1.4 GitHub Actions Signing

Tag releases sign and notarize macOS ZIP and DMG artifacts in
`.github/workflows/release.yml`. Pull request builds use ad-hoc signing and
never receive release credentials.

Configure these GitHub Actions repository secrets:

- `MACOS_CERTIFICATE_P12_BASE64`: base64-encoded Developer ID Application
  certificate and private key exported as `.p12`.
- `MACOS_CERTIFICATE_PASSWORD`: password used when exporting the `.p12`.
- `MACOS_SIGN_IDENTITY`: exact `Developer ID Application: ... (TEAMID)` name.
- `APPLE_NOTARY_APPLE_ID`: Apple Account used for notarization.
- `APPLE_NOTARY_PASSWORD`: App-specific password for that Apple Account.
- `APPLE_TEAM_ID`: ten-character Apple Developer Team ID.

Encode the certificate without writing it to the repository:

```bash
/usr/bin/base64 -i DeveloperIDApplication.p12 | pbcopy
```

The workflow imports the certificate into an ephemeral keychain, validates the
notarization credentials, signs every embedded executable, notarizes the app
and final DMG, and removes temporary credentials even when the build fails.

## 2. Windows (Native Build on Windows Machine)

### 2.1 Tool Installation

- Install Rust (rustup): https://rustup.rs/
- Install MSVC build toolchain (choose one):
  - Visual Studio 2022 (select "Desktop development with C++")
  - Or Build Tools for Visual Studio (also requires C++ toolchain)
- WebView Runtime:
  - `vw-webview.exe` depends on Microsoft Edge WebView2 Runtime (Win10/11 usually comes with it; install it if missing)

### 2.2 Build

```powershell
cargo build --release
```

Output (example):
- `target\release\shi-ju.exe`
- `target\release\vw-webview.exe`

When distributing, ensure both exe files are in the same directory.

## 3. Cross-compilation for Windows on macOS (Recommended: MSVC target + cargo-xwin)

### 3.1 Install Target & Tools

Install Windows MSVC target:
```bash
rustup target add x86_64-pc-windows-msvc
```

Install `cargo-xwin`:
```bash
cargo install cargo-xwin
```

If you get `lld-link` / linker-related errors during build, install LLVM and add it to PATH:
```bash
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"
```

### 3.2 Cross-compile

```bash
cargo xwin build --release --target x86_64-pc-windows-msvc
```

Output location:
- `target/x86_64-pc-windows-msvc/release/shi-ju.exe`
- `target/x86_64-pc-windows-msvc/release/vw-webview.exe`

### 3.3 One-click Zip Bundle

The project provides a script (prefers cargo-xwin; falls back to cargo build):
```bash
bash scripts/bundle_windows.sh
```

Default output:
- `dist/windows/ShiJu-x86_64-pc-windows-msvc-release.zip`

Optional environment variables:
```bash
TARGET=x86_64-pc-windows-msvc PROFILE=release OUT_DIR=dist/windows bash scripts/bundle_windows.sh
```
