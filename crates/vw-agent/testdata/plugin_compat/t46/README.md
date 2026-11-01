# Content trust fixtures
Synthetic test inputs are generated in temporary directories by t46_tests.rs under the repository license. No user data or credentials.
Reference: frozen baseline.json entries for hooks/src/engine/plugin_fingerprint.rs (plugin_content_hash) and plugin_trust.rs (PluginHookTrust::verify).
Expected: script changes invalidate approval; invalid roots fail without persisted approval; source name/path are isolated and package bytes are never written by trust operations.
Runtime artifact exclusions follow the reference (.venv, __pycache__, .pyc, .egg-info). Unsupported entries fail closed. No private allowlist or legacy automatic migration.
