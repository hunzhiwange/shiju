# Task 57 fixtures

Repository-authored synthetic packages use the repository license. No credentials, provider calls or user contents are involved.

`t57_tests.rs` creates unchanged original Codex manifests for `fixture@alpha` and `fixture@beta`, versioned packages, a real skill source, and separate workspace/config stores. The identity and original version must remain stable for the active turn despite live enabled-list changes or a concurrent upgrade. The next turn must see the new version/selection. Foreign workspace/config/tenant and expired context snapshots must fail visibly. Serialized invoked-skill records retain the exact market-qualified identity and audited source.

Reference: read-only `codex-rs/core/src/session/plugin_selection.rs::Session::activate_plugin_selection` (admitted turn selection and hook refresh); `codex-rs/app-server/src/plugin_config_reload.rs::for_cwd` (same workspace reload boundary). This implementation uses repository lifecycle locks rather than a copied Codex runtime.

Lifecycle leases belong to the request stack, not persistent MCP client contexts. Cancelled asynchronous hooks are joined before lease release. Existing MCP close/cancellation, skill checkpoint, and hook tests remain neighboring acceptance coverage; no complete remote service availability is claimed.

The real MCP process acceptance case reuses the repository-authored `t18/server.py` protocol fixture (same license). It records PIDs in an isolated state path, verifies the old process survives while its turn is active, then verifies retirement and a different PID on the next turn. No external service is contacted.

Neighbor correction: the t18 fixture must advertise its existing `hang` handler as well as `pid`. The catalog admission gate correctly rejects undeclared calls; advertising the handler restores the timeout test's intended path without changing its timeout assertion or runtime admission policy. The legacy test package uses literal audited installed paths and cwd, since legacy placeholder expansion is outside this task.
