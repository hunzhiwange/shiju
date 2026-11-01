# Marketplace fixtures

Synthetic JSON authored for this repository, under the repository license; no credentials.
Inputs are embedded in `compat/t47_tests.rs` and materialized in temporary directories.
Reference: frozen `codex-rs/core-plugins/src/marketplace.rs` (`RawMarketplaceManifest`,
`resolve_plugin_source`, `marketplace_plugin_manifest_fallback`, manifest layout priority).
Expected: local paths relative to marketplace root, GitHub shorthand normalized,
npm version retained, explicit policy retained, duplicate identity and escaping paths rejected.
Security difference: malformed/unsupported entries fail the whole read instead of being silently skipped.
Reading has no installation, authentication, network, or package mutation side effects.
