# Build & Package (macOS / Windows / Web)

This document is located under `scripts/` for daily development and release.

## Relax live development

From the repository root, use `cargo dev-gateway` and `cargo dev-desktop` in separate
terminals. First use installs the pinned patched Relax automatically; `cargo dev-setup`
prepares it ahead of time. Requires Python 3.9+ and the existing Rust/native build toolchain.
See [team setup and validation](../docs/dev/Relax开发验证.md#团队一键入口当前仓库).

The desktop entry temporarily defaults `MAKEPAD_RUN_O3=0` after a reported
`SIGILL` in the O3 closure used by `session_row::local_time`. Hot reload remains
enabled. This disables the faulting optimization tier, but the compiler root cause
is still unconfirmed; it is not a guarantee against other JIT failures. Restart
`cargo dev-desktop` to apply it to an existing session. Gateway and live test
entries keep their existing optimization policy. For compiler reproduction only,
use `MAKEPAD_RUN_O3=1 cargo dev-desktop`; the entry prints the selected value.
An isolated chrono formatting loop passed even after reaching O3, so it does not
yet reproduce the full application failure. See the
[investigation evidence](../docs/dev/evidence/relax-jit-sigill-2026-11-13/report.md).

## Relax live focused tests

Keep one selected library test target warm while editing function bodies:

```sh
python3 scripts/cargo_slot.py cargo dev-test vw-desktop app::components::voice_chat::tests::voice_task_11_
# Or focus a validated gateway/config/client test family:
python3 scripts/cargo_slot.py cargo dev-test vw-config-types config_defaults_promote_shiju_as_primary_provider
```

`cargo dev-test <workspace-package> <test-filter>` uses the same pinned patched
Relax as the application entries, with `test --live`, default features, `--lib`,
`--nocapture`, and one test thread. Both arguments are required; empty filters and
extra flags are rejected before preparing the tool. This is a foreground process:
keep it running through the editing phase, inspect each new `testverdict`, then
stop it with Ctrl+C. A test counts as validated only when the new verdict has
`ran > 0`, `failed=0`, and the expected passed count; an old verdict or zero tests
is not evidence for the latest edit. A deliberate assertion failure is reported
and the next edit reruns tests in the same session.

Use this for test paths already verified under Relax. Type/layout, signature,
trait, macro, and dependency changes need fresh startup and native validation.
The final native focused tests and required full Clippy still apply. This entry
does not implement the Clippy, release, physical audio or permission checks.
Compiler diagnostics remain visible. Focused tests default to the runtime's
`minimal` ahead policy: compile reached code on demand and avoid speculative
callees competing with the next assertion. An explicitly set `MAKEPAD_RUN_AHEAD`
is preserved, for example `MAKEPAD_RUN_AHEAD=balanced cargo dev-test ...`.
Application entries retain their previous policy. This test default is validated
on the focused rendering/event path; it does not promise faster startup for every
test family.

## Reusing a Cargo slot during development

Run a whole foreground development phase inside one slot (macOS/Linux):

```sh
python3 scripts/cargo_slot.py sh -c '
  cargo check -p vw-desktop --lib &&
  cargo test -p vw-desktop --lib voice_task_11_ -- --test-threads=1 &&
  cargo clippy --all-targets --all-features -- -D warnings
'
```

The wrapper atomically claims the first free slot among `target/rust-slot-1` through
`rust-slot-20`, records its PID, and sets **both** `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` to that absolute path. Nested wrapper calls reuse the owner
receipt; separate concurrent commands receive separate slots. Keep related checks
inside one invocation to retain the slot across the phase. Finished slots keep their
cache. Locks are released after the foreground process group stops, including
failure or interruption; do not launch detached/background work from this wrapper.
Existing locks are never reclaimed automatically.

`cargo dev-setup`, `cargo dev-desktop`, `cargo dev-gateway`, and `cargo dev-test` claim a slot for
the Python preparation and Relax process lifetime. When run inside the wrapper,
they inherit its slot. The small Rust launcher is built by the initial Cargo alias
before Python can claim a slot; wrap the alias itself when every Cargo invocation
must be inside the slot. Aliases no longer override the caller's target directory.
Relax installation builds in the owned slot and copies the finished runner to its
versioned tool cache; runtime and metadata subprocesses inherit the same paths.

The repository no longer fixes `build.build-dir` independently of `target-dir`:
Cargo's default keeps intermediate and final artifacts together even for existing
scripts that only set `CARGO_TARGET_DIR`. An explicit external
`CARGO_BUILD_BUILD_DIR` still takes precedence for bare Cargo commands; use this
wrapper to normalize it. Existing caches are not migrated or deleted. Dev profile,
features, and the mandatory final Clippy check are unchanged.

## Web static archive

Tag releases also build the complete `vw-desktop` / `shi-ju` application for
`wasm32-unknown-unknown` using the `dist` profile. The shared `build-web.yml`
workflow pins wasm-bindgen CLI to the version in Cargo.lock, generates all local
bindings and snippets, and uploads `shiju-web-<version>.zip` plus `.zip.sha256`.
The Release host waits for this job; recovery validates the Web checksum before
uploading. The existing `allow-dirty = ["ci"]` keeps this custom release job.

After downloading and extracting the ZIP, run:

```sh
npm run start
```

Node.js 22+ with npm is sufficient; no npm dependencies, Python, Rust or source checkout
are needed to serve the archive at <http://127.0.0.1:8765/>. The UI is static;
agent operations still require a configured ShiJu gateway. See the bundled
README.md and [WASM local usage](../docs/dev/WASM本地运行.md).

Desktop users can choose **Help → Install Web version** (below Install CLI) to
install and start the independent archive. This uses the same saved update source
as desktop updates: ShiJu by default, or GitHub. Node.js 22+ must be on PATH.
The service uses a dynamically allocated loopback port; closing the dialog keeps
it running, and stopping it or exiting the desktop app terminates the process.
The installation is kept under the current build's home configuration directory,
in `web/current`; a verified replacement retains the old package in `web/previous`.

For the ShiJu source, `latest/manifest.json` supplies the stable version using the
existing desktop manifest schema. Mirror both `shiju-web-<version>.zip` and
`shiju-web-<version>.zip.sha256` unchanged into
`https://www.shiju.cloud/agents/shiju-agent/releases/v<version>/` before promoting
that manifest. The desktop-only `prepare_oss_release.py` currently prepares eight
native installers; the Web pair must also be mirrored by the release operator.
A missing Web pair fails explicitly; the app does not switch sources automatically.

Local packaging and relocated Node server checks (no publishing):

```sh
python3 -m unittest discover -s scripts/tests -p test_package_shiju_web.py
```

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

By default, the script runs incremental Cargo builds before packaging the desktop, WebView helper, CLI and ACP binaries. Unchanged code reuses the build cache. Building on macOS/Linux requires cargo-xwin; native Windows can use cargo build:
```bash
bash scripts/bundle_windows.sh
```

Default output:
- `dist/windows/ShiJu-x86_64-pc-windows-msvc-release.zip`

Optional environment variables:
```bash
TARGET=x86_64-pc-windows-msvc PROFILE=release OUT_DIR=dist/windows bash scripts/bundle_windows.sh
```

Release CI sets `SKIP_BUILD=1 PROFILE=dist TARGET=<triple>` to package the
binaries already produced by cargo-dist in `target-dir/<triple>/dist/`
(or under `CARGO_TARGET_DIR` when overridden). This skips all Cargo invocations
and fails if any required executable is missing, without rebuilding it.

### 电脑操控接入资源

macOS app 打包会在签名前将自有 Chrome/Office 资源、中继和用户安装器放到
`Contents/Resources/computer-control`，并生成相对路径 SHA-256 清单。
原生控制实现随桌面二进制交付；不复制 ChatGPT 私有运行时，不新增虚构 MCP。
安装与卸载、Chrome ID 限制、Office TLS 和旁加载要求见
`plugins/shiju-computer-control/README.md`。资源安装不授予系统或文档权限。

后续本地验证应通过 `python3 scripts/cargo_slot.py` 运行打包入口，继承
`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR`；例如在入口内设置
`PROFILE=release SKIP_BUILD=0 MACOS_SIGN_MODE=adhoc MACOS_NOTARIZE=0`
再执行 `bash scripts/bundle_macos.sh`。ad-hoc 仅用于开发，正式交付沿用既有
Developer ID 签名要求；本任务不公证、不发布。
