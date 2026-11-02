# Task 52 upgrade transactions

Repository-authored fixtures (repository license): two original manifests, `fixture`, versions `1` and `2`, generated in t52_tests.rs. No external packages or credentials. Reference: frozen Codex marketplace_upgrade.rs and marketplace_upgrade/activation.rs. This host adds a persisted transaction journal and lifecycle file locking.

Preview materializes the source through existing install factories and leaves the installed package unchanged. Activation checks both package content and installed metadata, journals rollback intent, renames original package and sidecar into the backup, then publishes the staged pair and journals commit. A store lock covers discovery and lifecycle mutation; recovery executes before a reader can load a pending store. Data and disabled markers are not moved. Content fingerprints revoke trust after a package change.

Tests: preview conflict, explicit activation, unchanged other-market version, disabled/data retention, trust invalidation, injected partial-publication failure, journaled interruption after the first rename, and concurrent discovery seeing matching manifest/metadata versions. The interruption fixture is a deterministic persisted on-disk checkpoint, not a claim of physical power-cut testing.

Validation platform: macOS arm64. Windows directory handles use FILE_FLAG_BACKUP_SEMANTICS and flush errors propagate. Actual Windows/Linux behavior remains part of the separate platform acceptance matrix; no cross-platform or physical power-loss acceptance is claimed here.
