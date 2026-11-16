# Task 37 evidence

Code migration implemented; compilation and behavior NOT verified. No Cargo, unit tests, build or device checks executed.

## Ownership and production paths

Existing production implementations were already in plugin-mcp/plugin-install/plugin-hooks: NEW production migration benefit = 0. Host adapters retain current tenant, active snapshot validation, application approval and tool execution. Domain tests do not depend on vw-agent or load its Rust source. JSON/Python fixture bytes are reused.

agent-plugins tests call real discover_plugins, mcp merge, manifest/declaration readers and actual mcp-client transport. Client fixture uses actual runtime_cwd/runtime_headers/request; it does not synthesize authorization. No-turn tests do not claim host snapshot/app-selection coverage. OAuth question tests use explicit default tenant fixtures; actual current-tenant host wiring stays in host tests.

agent-plugins adds dev dependencies mcp-client, rmcp, tokio-util, agent-question; lockfile edges updated manually, no resolution run. Both receiving libraries self-register cfg(test) host_migration_37_tests. Old migrated host registrations removed; t17 remains only shared package fixture.

## Full source mapping and splitlines counts

| compat source | Before | After | Ownership / retained responsibility |
|---|---:|---:|---|
|dependencies.rs|23|23|Real host composition/port: inspect_plugin_dependencies, missing_mcp_executable|
|hook_mcp.rs|4|4|Real host composition/port: execute|
|mcp_app_execution.rs|49|49|Real host composition/port: session, root, abort_token, tool_id, contains_tool, prepare_approval, drain_prompts, execute, register, revoke, revoke_for_tenant, call|
|mcp_apps.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_auth.rs|29|29|Real host composition/port: load_manifest, load_declarations, source_config, discover_oauth, runtime_headers, runtime_oauth_headers, hook_source_matches|
|mcp_capabilities.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_catalog.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_catalog_refresh.rs|1|1|Thin re-export required by existing direct callers; no production migration credit|
|mcp_connection_sessions.rs|10|10|Real host composition/port: connections, close_session|
|mcp_connection_settings.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_connections.rs|61|61|Real host composition/port: host, request_abort, runtime_stdio_cwd, runtime_headers, oauth_access, oauth_cancellation, selection_is_current, open_peer, discover, revoke_app, is_current, mount_leases|
|mcp_content.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_elicitation.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_environment.rs|20|20|Real host composition/port: resolve_servers|
|mcp_legacy.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_notifications.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_oauth_discovery.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_oauth_flow.rs|12|12|Real host composition/port: authorize, authorize_with|
|mcp_oauth_tokens.rs|9|9|Real host composition/port: access, save|
|mcp_portable.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_portable_overlay.rs|19|19|Real host composition/port: resolve_component_path, parse_legacy_manifest, apply_env_overlay|
|mcp_public_auth.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mcp_resources.rs|2|2|Thin re-export required by existing direct callers; no production migration credit|
|mod.rs|242|206|Shared parent: remove only this task migrated test registrations|
|t07_tests.rs|61|0|Moved fully to domain host_migration_37_tests|
|t15_tests.rs|95|0|Moved fully to domain host_migration_37_tests|
|t16_tests.rs|227|0|Moved fully to domain host_migration_37_tests|
|t17_public_tests.rs|76|0|Moved fully to domain host_migration_37_tests|
|t17_tests.rs|400|25|Only write/options/plugin fixtures used by remaining host t28.|
|t18_tests.rs|389|389|Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.|
|t19_tests.rs|184|170|Actual McpTool routing/runtime registration. Pagination tests can still move.|
|t20_tests.rs|516|516|Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.|
|t21_tests.rs|278|278|Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).|
|t22_tests.rs|366|366|Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.|
|t23_tests.rs|191|191|Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.|
|t24_tests.rs|174|174|Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.|
|t26_tests.rs|22|0|Moved fully to domain host_migration_37_tests|
|t27_flow_tests.rs|381|55|Only actual host stdio URL question/reply request identity wiring remains.|
|t27_tests.rs|81|0|Moved fully to domain host_migration_37_tests|
|t28_tests.rs|172|99|Only ignored public OAuth -> real host grant/pool/logout acceptance remains.|

Source total: 4116 -> 2730; net reduction 1386 (33.7%). BELOW 70%. Per-file reasons and continuing migration candidates listed above must be reviewed by closure task; no global reduction success claim.

## Every original test mapping (names/assertions preserved)


- `codex_plugin_compat_t07_preserves_healthy_skill_and_mcp_consumers` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t07_tests.rs`
- `codex_plugin_compat_t15_preserves_isolation` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t15_tests.rs`
- `codex_plugin_compat_t15_keeps_manifest_format_authoritative` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t15_tests.rs`
- `codex_plugin_compat_t16_starts_in_contained_plugin_directory` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_preserves_isolation` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_uses_data_cwd_without_creating_package_directories` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_keeps_legacy_relative_cwd_and_local_environment` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_launches_contained_relative_command_and_reaudits_links` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_rejects_data_alias_between_plugins` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_allows_missing_cwd_prefix_but_does_not_create_it` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t16_reaudits_cwd_after_discovery` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t16_tests.rs`
- `codex_plugin_compat_t17_accepts_public_auth_declarations_without_claiming_login` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_public_tests.rs`
- `codex_plugin_compat_t17_sends_scoped_authorization_header` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_rejects_invalid_input` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_preserves_isolation` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_applies_header_precedence_without_serializing_values` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_rejects_header_injection_and_host_owned_fields` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_keeps_public_oauth_configuration_without_starting_login` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_reaudits_source_and_rejects_forged_context` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_filters_portable_client_owned_headers` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_does_not_forward_credentials_on_redirect` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t17_rejects_insecure_credential_endpoints` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t17_tests.rs`
- `codex_plugin_compat_t18_reuses_connection_until_session_ends` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_serializes_startup_and_survives_bridge_runtime_drop` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_times_out_initialization_and_catalog_without_orphans` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_closes_timed_out_and_cancelled_calls` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_cancels_startup_and_leaves_other_sessions_alive` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_retires_changed_disabled_and_exited_servers` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_reuses_streamable_http_and_legacy_sse_sessions` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_keeps_tool_bindings_but_releases_session_lease` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_plugin_disable_closes_only_its_live_service` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_required_failure_blocks_actual_tool_execution` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t18_applies_host_call_timeout_to_actual_dynamic_tool` -> Retained `crates/vw-agent/src/plugins/compat/t18_tests.rs`: Actual session toolset/context, disable and executor admission. Pool lifecycle assertions can still be split.
- `codex_plugin_compat_t19_paged_names_route_to_original_wire_names` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t19_rejects_loop_and_oversized_cursor` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t19_keeps_empty_allowlist_and_deny_precedence` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t19_retains_same_named_servers_from_different_packages` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t19_denied_tool_cannot_bypass_catalog_filter` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t19_same_named_plugin_tools_call_their_own_process` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t19_runtime_registry_applies_plugin_filters_and_parallel_limit` -> Retained `crates/vw-agent/src/plugins/compat/t19_tests.rs`: Actual McpTool routing/runtime registration. Pagination tests can still move.
- `codex_plugin_compat_t20_invalidates_catalog_after_notification` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_rejects_invalid_input` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_inspection_refreshes_only_notified_resource_and_prompt_catalogs` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_retries_notification_during_initial_listing_without_mixed_publication` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_rejects_persistent_notification_churn` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_reconnect_rejects_old_binding_without_closing_current_binding` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_notification_does_not_revoke_an_already_admitted_call` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_equal_notification_preserves_bindings_and_shutdown_closes_catalogs` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_unadvertised_changes_do_not_enable_persistent_caching` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_rejects_resource_template_and_prompt_cursor_cycles` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t20_shutdown_cancels_inflight_resource_refresh` -> Retained `crates/vw-agent/src/plugins/compat/t20_tests.rs`: Actual McpTool invocation shares notification/resource-cache fixture. Cache contracts can still split.
- `codex_plugin_compat_t21_preserves_structured_and_media_content` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_rejects_invalid_input` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_preserves_large_and_empty_results` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_null_structured_content_does_not_expose_host_metadata` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_rejects_malformed_results_without_exposing_payload` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_keeps_existing_stdio_notification_compatibility` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_preserves_extensions_without_response_or_host_collision` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_distinguishes_call_timeout_from_tool_failure` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t21_preserves_results_over_http_json_and_both_sse_paths` -> Retained `crates/vw-agent/src/plugins/compat/t21_tests.rs`: Actual McpTool -> result -> DTO transport chain. Continue toward tool-mcp (outside this task candidate targets).
- `codex_plugin_compat_t22_preserves_resource_origin_and_prompt_roles` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_rejects_invalid_input` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_lists_all_pages_and_preserves_metadata` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_rejects_invalid_actions_uris_and_parameter_shapes` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_rejects_pagination_cycles_and_limits` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_rejects_remote_malformed_content_and_roles` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_keeps_package_and_host_files_unchanged` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_disabled_and_cancelled_sessions_reject_before_connecting` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_reads_and_gets_over_http_json_and_sse` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_registers_resource_tool_in_actual_runtime` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_inspection_retains_resource_and_prompt_metadata` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t22_preserves_admission_timeouts_and_sanitizes_remote_errors` -> Retained `crates/vw-agent/src/plugins/compat/t22_tests.rs`: Actual McpResourceTool, runtime registration and inspection. Protocol cases can still split.
- `codex_plugin_compat_t23_advertises_only_implemented_capabilities` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t23_rejects_invalid_input` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t23_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t23_retains_known_fallbacks_without_advertising_roots` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t23_rejects_unknown_versions_without_reflecting_payloads` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t23_sends_negotiated_http_version_on_each_request` -> Retained `crates/vw-agent/src/plugins/compat/t23_tests.rs`: Actual host ConnectionHost -> handshake/version propagation. Protocol assertions can still move with a thin host wiring test retained.
- `codex_plugin_compat_t24_returns_validated_user_response` -> Retained `crates/vw-agent/src/plugins/compat/t24_tests.rs`: Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.
- `codex_plugin_compat_t24_rejects_invalid_input` -> Retained `crates/vw-agent/src/plugins/compat/t24_tests.rs`: Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.
- `codex_plugin_compat_t24_preserves_isolation` -> Retained `crates/vw-agent/src/plugins/compat/t24_tests.rs`: Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.
- `codex_plugin_compat_t24_timeout_removes_pending_form` -> Retained `crates/vw-agent/src/plugins/compat/t24_tests.rs`: Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.
- `codex_plugin_compat_t24_desktop_roundtrip` -> Retained `crates/vw-agent/src/plugins/compat/t24_tests.rs`: Actual question events, ConnectionHost, desktop gateway. Form/timeout cases can still split.
- `codex_plugin_compat_t26_host_entry_revalidates_package_identity` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t26_tests.rs`
- `codex_plugin_compat_t27_callback_exchanges_matching_pkce` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_denial_never_exchanges_token` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_cancellation_cleans_question` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_timeout_closes_callback` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_forged_callback_never_exchanges_token` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_source_change_never_exchanges_token` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_url_stdio_completes_exact_request` -> Retained `crates/vw-agent/src/plugins/compat/t27_flow_tests.rs`: Only actual host stdio URL question/reply request identity wiring remains.
- `codex_plugin_compat_t27_dcr_registers_public_loopback_client` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_dcr_rejects_secret_client` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_cimd_uses_only_configured_public_identity` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_cimd_requires_server_advertisement` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_scopes_fall_back_to_discovery` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_explicit_empty_scopes_never_expand` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_callback_rejects_wrong_issuer` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t27_flow_tests.rs`
- `codex_plugin_compat_t27_url_accepts_user_confirmation` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t27_url_declines_user_denial` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t27_url_cancels_rejected_question` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t27_url_cancels_closed_session` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t27_url_wire_preserves_request_identity` -> `crates/libs/plugin-mcp/src/host_migration_37_tests.rs`
- `codex_plugin_compat_t28_owner_keys_and_concurrent_refresh` -> `crates/libs/agent-plugins/src/host_migration_37_tests/t28_owner_tests.rs`
- `codex_plugin_compat_t28_public_oauth_refresh` -> Retained `crates/vw-agent/src/plugins/compat/t28_tests.rs`: Only ignored public OAuth -> real host grant/pool/logout acceptance remains.

## Future checks

```sh
python3 scripts/cargo_slot.py cargo test -p agent-plugins --lib host_migration_37_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p plugin-mcp --lib host_migration_37_tests -- --test-threads=1
```

Expected nonzero test counts: agent-plugins 37; plugin-mcp 6. No run/pass claims. Run retained host tests by exact original names above; use actual vw-agent --list to establish removed registrations, never zero-test success.

RustCodeGraph status inspected; index predates concurrent migration, so raw source/registrations reviewed. No measured compile or runtime savings.

## Task 1 external resources

- `crates/vw-agent/testdata/plugin_compat/t07/native.json`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t16/server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t18/server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t21/http_server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t21/result.json`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t21/server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t23/http_server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.
- `crates/vw-agent/testdata/plugin_compat/t23/server.py`: retained resource, original bytes unchanged; referenced by moved domain tests or remaining host transport tests.

Physical classification before: host production/forwarding 261; host tests/fixtures 3613; shared registration 242. Production content unchanged; these are not new domain algorithms.
- agent-plugins: domain production delta 0; new tests/fixtures 1349 lines.
- plugin-mcp: domain production delta 0; new tests/fixtures 95 lines.

Shared mod.rs also has concurrent task changes; physical totals above reflect the working snapshot, not exclusive ownership of every removed registration line. Exclusive own net deletion must be recomputed from the commit diff during closure.


Concurrent commit recovery temporarily removed domain registrations; final 87aa96fbc restores the task37 registrations/lock edges on the recovered HEAD. Receiving test files are tracked in final HEAD. Recompute exclusive net reduction from task changes at closure; transient intermediate HEAD counts are not acceptance evidence.
