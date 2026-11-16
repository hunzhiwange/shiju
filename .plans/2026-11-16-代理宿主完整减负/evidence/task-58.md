# Task 58: implementation complete; compilation and behavior unverified

Per plan, no Cargo, build, test or device commands were run. Static source and registration review only. Task 1 evidence contains no additional config path-mounted tests; receiver has no reverse host path/include dependency.

## Ownership and wiring

migration::services and migrate_user_config duplicated existing agent_config::runtime_migration composition. Remove the duplicate host file and re-export the existing domain module from config/mod.rs. Actual production caller: agent-config/runtime_schema/config_load.rs -> runtime_migration::migrate_user_config -> migration::migrate_user_config_with_database. No new production algorithm migration claimed. The database-specific test entry is cfg(test), using the same domain services.

Host ChannelConfigWatcher retains new (database initialization, inherited test database context and spawn_blocking), from_store (capture current tenant), poll and load_for_delivery (delegation). Remove its test-only store field. Domain tests use the real SystemConfigStore and an explicit default tenant; the local wrapper only supplies fixture wiring, not watcher logic.

Environment tests call runtime_schema::apply_env_overrides, preserving real EnvironmentHost wiring. Reuse the existing domain proxy mutex with crate-visible cfg(test) access. Restore all original environment keys and runtime proxy snapshot. Existing dev dependencies already provide tempfile, tokio macros and agent-database/test-support; SQLite/PostgreSQL fixture and explicit missing-URL skip behavior retained. No dependency changes.

## All source files (splitlines)

| Host config/ path | Before | After | Destination / retained responsibility |
|---|---:|---:|---|
| migration.rs | 32 | 0 | Removed duplicate composition; domain runtime_migration module alias |
| migration_tests.rs | 197 | 0 | All tests and helpers -> host_migration_58_tests/migration.rs |
| mod.rs | 23 | 20 | Compatibility API plus get_current tenant/project capture |
| traits.rs | 2 | 2 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/automation.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/channel_reload.rs | 40 | 37 | Host database and tenant capture; domain delegation |
| schema/channel_reload_tests.rs | 124 | 36 | Three domain tests -> host_migration_58_tests/channel_reload.rs; retain one host context test and set_telegram helper |
| schema/config_env_tests.rs | 512 | 0 | All tests and guard -> host_migration_58_tests/config_env.rs |
| schema/config_helpers.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/config_io.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/config_secrets.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/mod.rs | 8 | 6 | Domain schema exports plus host watcher registration |
| schema/provider.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/proxy.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/security.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/skills.rs | 1 | 1 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/workspace.rs | 4 | 4 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |
| schema/channels/mod.rs | 2 | 2 | Existing thin compatibility exports only; no business logic. Preserve public API and existing domain ownership. |

Host total 952 -> 115: reduction 837 (87.9%), above 70%. Before: 833 test-file lines, 119 non-test-file lines (compatibility/composition, not all business logic). After: 36 test-file lines, 79 compatibility/composition lines. Domain added test lines: 830. Domain production algorithm unchanged; runtime_migration adds 8 test-only entry lines; proxy changes test-only visibility. lib.rs adds 3 registration lines.

## Every test and helper

### migration_tests.rs (7 tests)
- `system_config_migration_reports_no_source_without_creating_storage` -> `host_migration_58_tests::migration::system_config_migration_reports_no_source_without_creating_storage`; full original function/assertions retained.
- `system_config_migration_refuses_to_overwrite_existing_backup` -> `host_migration_58_tests::migration::system_config_migration_refuses_to_overwrite_existing_backup`; full original function/assertions retained.
- `system_config_migration_imports_local_config_only_after_verification` -> `host_migration_58_tests::migration::system_config_migration_imports_local_config_only_after_verification`; full original function/assertions retained.
- `system_config_migration_finishes_when_database_already_has_same_config` -> `host_migration_58_tests::migration::system_config_migration_finishes_when_database_already_has_same_config`; full original function/assertions retained.
- `system_config_migration_preserves_conflicting_source` -> `host_migration_58_tests::migration::system_config_migration_preserves_conflicting_source`; full original function/assertions retained.
- `system_config_migration_preserves_invalid_source` -> `host_migration_58_tests::migration::system_config_migration_preserves_invalid_source`; full original function/assertions retained.
- `system_config_migration_accepts_legacy_toml_source` -> `host_migration_58_tests::migration::system_config_migration_accepts_legacy_toml_source`; full original function/assertions retained.
Helpers: `write_source`, `backup_path`, `system_config_migration_round_trip`, `for_each_migration_provider`, `system_config_migration_already_imported`, `system_config_migration_conflict`. Moved with group; set_telegram also remains for the host boundary.
### schema/config_env_tests.rs (14 tests)
- `apply_env_overrides_has_stable_in_place_signature` -> `host_migration_58_tests::config_env::apply_env_overrides_has_stable_in_place_signature`; full original function/assertions retained.
- `apply_env_overrides_prefers_vibewindow_names_over_generic_fallbacks` -> `host_migration_58_tests::config_env::apply_env_overrides_prefers_vibewindow_names_over_generic_fallbacks`; full original function/assertions retained.
- `apply_env_overrides_uses_generic_fallback_values` -> `host_migration_58_tests::config_env::apply_env_overrides_uses_generic_fallback_values`; full original function/assertions retained.
- `legacy_provider_replaces_builtin_shiju_default` -> `host_migration_58_tests::config_env::legacy_provider_replaces_builtin_shiju_default`; full original function/assertions retained.
- `valid_workspace_skills_and_storage_values_are_normalized` -> `host_migration_58_tests::config_env::valid_workspace_skills_and_storage_values_are_normalized`; full original function/assertions retained.
- `boolean_overrides_preserve_multi_alias_flags_but_disable_strict_flags` -> `host_migration_58_tests::config_env::boolean_overrides_preserve_multi_alias_flags_but_disable_strict_flags`; full original function/assertions retained.
- `blank_and_invalid_values_leave_structured_configuration_unchanged` -> `host_migration_58_tests::config_env::blank_and_invalid_values_leave_structured_configuration_unchanged`; full original function/assertions retained.
- `numeric_overrides_accept_inclusive_boundaries` -> `host_migration_58_tests::config_env::numeric_overrides_accept_inclusive_boundaries`; full original function/assertions retained.
- `reasoning_level_alias_prefers_prefixed_value_and_normalizes_fallback` -> `host_migration_58_tests::config_env::reasoning_level_alias_prefers_prefixed_value_and_normalizes_fallback`; full original function/assertions retained.
- `proxy_overrides_normalize_and_auto_enable_valid_service_scope` -> `host_migration_58_tests::config_env::proxy_overrides_normalize_and_auto_enable_valid_service_scope`; full original function/assertions retained.
- `explicit_proxy_disable_prevents_url_override_from_auto_enabling` -> `host_migration_58_tests::config_env::explicit_proxy_disable_prevents_url_override_from_auto_enabling`; full original function/assertions retained.
- `all_proxy_override_is_normalized_and_auto_enables_proxy` -> `host_migration_58_tests::config_env::all_proxy_override_is_normalized_and_auto_enables_proxy`; full original function/assertions retained.
- `environment_proxy_scope_applies_normalized_process_variables` -> `host_migration_58_tests::config_env::environment_proxy_scope_applies_normalized_process_variables`; full original function/assertions retained.
- `invalid_proxy_configurations_are_disabled_before_runtime_use` -> `host_migration_58_tests::config_env::invalid_proxy_configurations_are_disabled_before_runtime_use`; full original function/assertions retained.
Helpers: `env_lock`, `new`, `set`, `remove`, `drop`. Moved with group; set_telegram also remains for the host boundary.
### schema/channel_reload_tests.rs (4 tests)
- `channel_reload_detects_add_edit_and_disable_without_unrelated_reconnects` -> `host_migration_58_tests::channel_reload::channel_reload_detects_add_edit_and_disable_without_unrelated_reconnects`; full original function/assertions retained.
- `channel_reload_ignores_other_tenants_and_respects_project_overrides` -> `host_migration_58_tests::channel_reload::channel_reload_ignores_other_tenants_and_respects_project_overrides`; full original function/assertions retained.
- `channel_reload_isolates_invalid_secrets_and_recovers_on_next_save` -> `host_migration_58_tests::channel_reload::channel_reload_isolates_invalid_secrets_and_recovers_on_next_save`; full original function/assertions retained.
- `channel_reload_blocking_open_uses_callers_persisted_configuration` -> `config::schema::channel_reload::tests::channel_reload_blocking_open_uses_callers_persisted_configuration`; full original function/assertions retained.
Helpers: `fixture`, `set_telegram`, `save`. Moved with group; set_telegram also remains for the host boundary.

## Registration and future checks

Domain lib.rs registers host_migration_58_tests with cfg(test); migration/channel_reload native-only. Original host migration_tests and config_env_tests modules/files removed. Expected domain count: 24; host boundary: 1. No assertions removed.

- `python3 scripts/cargo_slot.py cargo test -p agent-config --lib host_migration_58_tests -- --test-threads=1`: expect 24 tests; configure VIBEWINDOW_TEST_POSTGRES_URL for PostgreSQL branch.
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib config::schema::channel_reload::tests::channel_reload_blocking_open_uses_callers_persisted_configuration -- --exact --test-threads=1`: expect 1 test.
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`: compare every migrated name above and confirm 24 original host tests absent. Zero tests is not a pass.

All compilation, formatting, behavior and runtime checks remain unrun by explicit plan instruction. Static review does not establish runtime success or compilation speed gains.
