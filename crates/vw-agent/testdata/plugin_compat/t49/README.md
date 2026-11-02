# Task 49 registry fixture

Source: repository-authored deterministic HTTP registry in `compat/t49_tests.rs`, under the repository license. No upstream package or credentials are included.

Input: `@scope/pkg`, selector `1.2.3` or default `latest`; version metadata `{name, version, dist: {tarball, integrity}}`. Archive is the literal ASCII payload `original archive bytes` (task 49 does not extract it). Expected output: unchanged bytes, resolved `1.2.3`, package name and SHA-256; SHA-512 integrity mismatch fails. Redirect returns 302 and must never be followed. Invalid source syntax and production loopback HTTP fail before connecting.

Reference: frozen baseline `marketplace.npm.package`, `.version`, `.registry`, `.pack` and task 49 behavior. Codex `core-plugins/src/npm_source.rs::pack_npm_package` disables scripts. This implementation uses a credential-free registry download instead of the npm subprocess. Only exact versions and tags are accepted; semver ranges are explicitly unsupported. Only SHA-512/SHA-256 SRI is accepted (no SHA-1 fallback). All redirects are rejected; each production destination requires public HTTPS with pinned DNS addresses. The local HTTP exemption is only reachable in unit-test builds.
