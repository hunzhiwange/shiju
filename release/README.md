# Release Packaging

Release publishing is driven by cargo-dist and GitHub Actions.

## Outputs

- `shiju-<target>` archives contain the CLI (`shiju`), ACP binary (`acp`), desktop binary (`shi-ju`), and webview helper (`vw-webview`) for that target.
- `vwacp-<target>` archives contain only the ACP binary.
- macOS publishes per-architecture DMG files for `x86_64-apple-darwin` and `aarch64-apple-darwin`.
- Linux publishes portable archives plus `.deb` and `.rpm` packages for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.
- Windows publishes portable zip archives plus MSI installers for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- npm publishes the CLI installer package as `shiju`, so users can run `npm install -g shiju`.

Linux `.deb`/`.rpm` packages are built by `scripts/bundle_linux.sh` with `dpkg-deb` and `rpmbuild` after cargo-dist has produced the target binaries.

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
