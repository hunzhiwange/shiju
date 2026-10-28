# Build & Package (macOS / Windows)

This document is located under `scripts/` for daily development and release.

## Release permission preflight (no build)

Run **Actions → Release preflight → Run workflow**, selecting the branch with
the fix and supplying an existing tag without a release (for example `v0.5.6`).
This uses the workflow's `GITHUB_TOKEN` to create a draft, upload a tiny text
asset, and delete that draft by ID. It preserves the tag and refuses tags that
already have a release. Cleanup failure reports the draft ID for manual removal.
It does not compile Rust or publish a release. The job has a five-minute timeout.

Tag-triggered release builds require this preflight to succeed before starting
the platform build matrix. A manual run requires the workflow to be present on
the default branch. Rerunning an old release run still uses its old workflow.

Local failure-handling checks (no network writes):

```bash
python3 -m unittest discover -s scripts/tests -p test_release_permissions.py
```

The local checks cannot validate GitHub token permissions. The online probe
checks draft creation and upload; final publication and downstream Homebrew/npm
publishing can still fail independently.

## npm distribution

The four npm installers use the `@shiju-ai` scope, configured by `npm-scope`
in `dist-workspace.toml`:

```bash
npm install @shiju-ai/shiju@0.5.6
npm install @shiju-ai/shiju-acp@0.5.6
npm install @shiju-ai/shiju-desktop@0.5.6
npm install @shiju-ai/shiju-webview@0.5.6
```

These installers download existing GitHub Release binaries. Automated publishing
requires a trusted publisher on each npm package for `hunzhiwange/shiju` and
`release.yml`, with direct publishing allowed. No npm password or long-lived npm
token is used. Homebrew uses `HOMEBREW_TAP_DEPLOY_KEY`, a write deploy key limited
to `hunzhiwange/homebrew-tap` (deploy key title: `shiju-release-homebrew`).

Tag releases check npm OIDC token exchange for all four packages and Homebrew
SSH write access before building. Both publishers download the completed GitHub
Release artifacts and verify checksums. Matching npm versions and unchanged tap
formulas succeed without republishing or creating empty commits.

For a lightweight online check, run **Actions → Release → Run workflow** with
`mode=check`. For a retry after a partial publishing failure, choose
`mode=publish-existing` and set `tag` to the existing release tag. Both manual
modes skip planning, building and packaging. The retry rejects published npm
versions whose contents differ from the Release artifact.

To rotate the Homebrew key, create a new write deploy key on the tap, replace
the `HOMEBREW_TAP_DEPLOY_KEY` secret on `hunzhiwange/shiju`, verify with
`mode=check`, then delete the old deploy key.

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

Release CI sets `SKIP_BUILD=1 PROFILE=dist TARGET=<triple>` to package the
executables already produced by cargo-dist in `target-dir/<triple>/dist`
(or the selected `CARGO_TARGET_DIR`). In this app packaging mode, all enabled
executables must already exist and be executable; missing artifacts fail without
rebuilding. The bundled CLI therefore uses the same features as the dist CLI.
Local builds retain their existing compilation and feature selection by default.

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

Credential checks, notarization submissions, result polling, and ticket stapling
retry failed commands up to five attempts, waiting 15, 30, 60, and 120 seconds.
Polling retries reuse the submission ID without uploading the archive again;
each wait attempt has a 10-minute timeout. An `Invalid` or `Rejected` notarization
result fails immediately, and only `Accepted` proceeds to stapling. Exhausted
retries or failed signature, ticket, or Gatekeeper validation still fail the build.

#### Reuse the shiju-agent signing keychain

Run on rsungM1, where the existing certificate, private key and notarization
profile are stored. No certificate export or private-key copy is required:

```bash
export APPLE_SIGNING_IDENTITY='Developer ID Application: Chengdu Rsung Information Technology Co., Ltd. (C6J8T6SJPD)'
export SHIJU_NOTARY_PROFILE=shiju-notary
export SHIJU_NOTARY_KEYCHAIN="$HOME/Library/Keychains/shiju-build.keychain-db"
export SHIJU_NOTARY_KEYCHAIN_PASSWORD_FILE="$HOME/.config/shiju-remote-build/keychain-password"
MACOS_SIGN_MODE=developer-id MACOS_NOTARIZE=1 PACKAGE_APP=1 CREATE_DMG=1 \
CARGO_TARGET_DIR=target ./scripts/bundle_macos.sh
```

The script accepts these existing shiju-agent variables. Explicit
`MACOS_SIGN_IDENTITY`, `NOTARY_KEYCHAIN_PROFILE`, `NOTARY_KEYCHAIN_PATH` and
`NOTARY_KEYCHAIN_PASSWORD_FILE` take precedence over their shiju-agent equivalents.
It checks the selected keychain and notarization authentication before compiling,
unlocks again before signing and notarization, and signs the DMG before submitting
it. Missing credentials fail the release; they never select ad-hoc signing.
Keep the unlock file private (mode `600`) and outside the repository. Each new
package still requires its own signature and notarization.

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
cargo install cargo-xwin --locked
```

Install LLVM and add it to PATH for the linker and the `llvm-rc` resource compiler used to embed the Windows executable icon:
```bash
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"
```

On macOS the desktop build also discovers keg-only LLVM through `brew --prefix llvm`.
On other hosts, ensure `llvm-rc` is on PATH; Windows SDK `rc.exe` is also supported.
Missing or failing resource compilers stop the build so packages never silently
lose their executable icon.

### 3.2 Cross-compile

```bash
cargo xwin build --release --target x86_64-pc-windows-msvc
```

Output location:
- `target/x86_64-pc-windows-msvc/release/shi-ju.exe`
- `target/x86_64-pc-windows-msvc/release/vw-webview.exe`

### 3.3 One-click Zip Bundle

The script always runs incremental Cargo builds before packaging the desktop, WebView helper, CLI and ACP binaries. Unchanged code reuses the build cache; existing executable files never bypass the freshness check. Building on macOS/Linux requires cargo-xwin; native Windows can use cargo build:
```bash
bash scripts/bundle_windows.sh
```

Default output:
- `dist/windows/ShiJu-x86_64-pc-windows-msvc-release.zip`

Optional environment variables:
```bash
TARGET=x86_64-pc-windows-msvc PROFILE=release OUT_DIR=dist/windows bash scripts/bundle_windows.sh
```
