# Release Packaging

Release publishing is driven by cargo-dist and GitHub Actions.

## Outputs

- CI publishes `shiju-<target>` archives containing the CLI (`shiju`) and compatibility entry point (`vibe-agent`).
- CI publishes separate `shiju-acp-<target>`, `shiju-desktop-<target>`, and `shiju-webview-<target>` archives containing `acp`, `shi-ju`, and `vw-webview`, respectively.
- Unix archives use `.tar.gz`; Windows archives use `.zip`.
- `scripts/package-release.sh --kind all` produces a local `shiju-<target>` archive containing the CLI, ACP, desktop, and webview binaries. Its individual package kinds produce `shiju-cli-<target>`, `shiju-acp-<target>`, and `shiju-desktop-<target>` archives.
- macOS publishes per-architecture DMG files for `x86_64-apple-darwin` and `aarch64-apple-darwin`.
- Linux publishes portable archives plus `.deb` and `.rpm` packages for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.
- Windows publishes portable zip archives plus MSI installers for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- npm publishes the CLI installer package as `shiju`, so users can run `npm install -g shiju`.

Install the CLI with any of these commands, then run `shiju --help`:

```sh
brew install hunzhiwange/tap/shiju
npm install -g shiju
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/hunzhiwange/shiju/releases/latest/download/shiju-installer.sh | sh
```

On Windows:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/hunzhiwange/shiju/releases/latest/download/shiju-installer.ps1 | iex"
```

Each application's `crates/vw-*/dist.toml` overrides its distribution name. Cargo crate names stay unchanged; cargo-dist uses the distribution names for archives, installer filenames, Homebrew formulas, npm packages, and release instructions. Check these outputs with `cargo dist plan --output-format=json` before tagging a release.

Linux `.deb`/`.rpm` packages are built by `scripts/bundle_linux.sh` with `dpkg-deb` and `rpmbuild` after cargo-dist has produced the target binaries.

## Windows MSI

The MSI wizard lets users choose the install folder (default: `C:\Program Files\ShiJu`),
shows installation progress, and stays on a completion page until they click Finish.
The completion page explains how to open ShiJu and use the CLI in a new terminal.
Silent installations (`/qn`) intentionally omit these dialogs.

The installer creates desktop and Start menu shortcuts, updates the system PATH,
and registers the product with Windows Installer for repair and uninstall.
Shortcut markers are stored under `HKCU\Software\ShiJu`; the selected folder is
stored in the 64-bit `HKLM\Software\ShiJu\InstallLocation` value and reused on upgrades.
Uninstall removes these installer-owned values and the installed PATH entry.
An explicit folder supplied by the wizard or `INSTALLFOLDER` takes precedence:

```powershell
msiexec /i ShiJu-x86_64-pc-windows-msvc-dist.msi INSTALLFOLDER="D:\Apps\ShiJu"
```

MSI builds require WiX 5.0.2 and its matching UI extension (also installed by CI):

```sh
dotnet tool install --global wix --version 5.0.2
wix extension add --global WixToolset.UI.wixext/5.0.2
```

Before publishing, verify on Windows: initial install to a path containing spaces
and Chinese characters, visible completion page, shortcut/CLI launch, upgrade
preserving that folder, repair, cancellation, and uninstall cleanup. Test x64 and ARM64.

## Required GitHub Secrets

- `NPM_TOKEN`: required for publishing the generated npm installer package.
- `HOMEBREW_TAP_TOKEN`: optional later; required only after creating `hunzhiwange/homebrew-tap` and uncommenting the Homebrew publish blocks.
- `MACOS_SIGN_P12`, `MACOS_SIGN_PASSWORD`, `MACOS_SIGN_IDENTITY`, `KEYCHAIN_PASSWORD`: optional macOS signing inputs.
- `MACOS_NOTARY_KEY`, `MACOS_NOTARY_KEY_ID`, `MACOS_NOTARY_ISSUER_ID`, `MACOS_NOTARY_KEYCHAIN_PROFILE`: optional notarization inputs.

Push a semver tag such as `v0.1.0` to publish a release. Run the workflow manually to publish or refresh the rolling `nightly` release.

## GitHub Workflows

- `release.yml`: builds and publishes release artifacts.
- `test.yml`: runs clippy, native tests, and a wasm desktop build.
- `security.yml`: runs `cargo-deny` using the repository `deny.toml`.
- `nix-build.yml` and `update-flake.yml`: mirror the reference workflow shape and skip explicitly until this repository adds `flake.nix`.

## Local Make Targets

- `make release-local`: build the current host portable package.
- `make release-cli-local`, `make release-acp-local`, `make release-desktop-local`: build one local package family.
- `make release-macos-packages`: build the local macOS `.app` zip and DMG with helper, CLI, and ACP bundled.
- `make release-linux`: build Linux portable packages through `cross` and Docker.
- `make release-windows`: build Windows portable packages through `cargo-xwin`, plus the x64 desktop zip.
- `make release-package-target RELEASE_KIND=acp RELEASE_TARGET=x86_64-unknown-linux-gnu RELEASE_BUILD_FLAGS=--use-cross`: build an ad-hoc target/kind package.
