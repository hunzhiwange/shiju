# 任务34迁移证据

代码完成，编译/行为未验证。遵守本轮禁止 Cargo、构建、单元与实机测试的要求。

## 所有来源与全用例映射

现有生产下沉不计本轮收益。本轮删除无调用的 file_edit/utils 与 http_headers 转发；其算法一直由 tool-files/tool-web 持有。其余生产保留为 AgentContextHost 特化、真实租户/项目配置、OAuth、外部连接和生命周期端口。



### crates/vw-agent/src/tools/file_edit/tests.rs

物理行数 544 → 0。

- `edit_name` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_schema_has_required_fields` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_replaces_existing_text_after_read` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_blocks_without_prior_read` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_blocks_when_file_changed_since_read` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_requires_unique_old_string_unless_replace_all` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_replace_all_replaces_multiple_matches` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_preserves_quote_style_for_normalized_match` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_reports_no_op_when_replacement_is_identical` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_supports_notebook_paths` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_call_rejects_blank_file_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_execute_rejects_invalid_parameters` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_call_rejects_invalid_parameters` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_execute_returns_failure_for_missing_file` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_blocks_read_only_before_file_access` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_blocks_when_rate_limited_before_file_access` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_rejects_missing_file_metadata` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_rejects_directory_target` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

- `edit_rejects_symlink_target` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/file_edit/utils.rs

物理行数 10 → 0。

无直接定义测试。保留文件职责：删除无调用兼容转发。

### crates/vw-agent/src/tools/file_write/mod.rs

物理行数 5 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/file_write/tests.rs

物理行数 839 → 0。

- `file_write_name` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_schema_has_path_and_content` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_description_is_available` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_creates_file` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_overwrite_without_prior_read` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_creates_parent_dirs` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_overwrites_existing` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_reports_unchanged_existing_content` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_stale_overwrite` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_path_traversal` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_absolute_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_accepts_workspace_absolute_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_rejects_directory_target` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_missing_path_param` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_rejects_blank_path_value` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_missing_content_param` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_empty_content` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_symlink_escape` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_readonly_mode` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_when_rate_limited` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_symlink_target_file` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_blocks_null_byte_in_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_accepts_path_alias` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_accepts_file_path_alias` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_call_rejects_invalid_parameters` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

- `file_write_supports_notebook_paths` → `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/git_operations/mod.rs

物理行数 5 → 3。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/git_operations/tests.rs

物理行数 72 → 0。

- `task_pool_git_uses_active_worktree_directory` → `crates/libs/tool-assembly/src/host_migration_34_tests/git_operations.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/glob/mod.rs

物理行数 5 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/glob/tests.rs

物理行数 241 → 0。

- `glob_name_and_schema` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_finds_matches` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_uses_relative_path_argument` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_accepts_cwd_alias` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_returns_not_found_message` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_rejects_blank_pattern` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_rejects_missing_search_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_rejects_file_as_search_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_rejects_forbidden_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

- `glob_rate_limited` → `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/glob_search/mod.rs

物理行数 12 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/grep/mod.rs

物理行数 12 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/hooks/mod.rs

物理行数 19 → 19。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/hooks/tests.rs

物理行数 66 → 34。

- `pre_tool_hook_without_runner_preserves_tool_call` → `crates/libs/tool-assembly/src/host_migration_34_tests/hooks.rs`（完整函数，原名与断言保留）

- `pre_tool_hook_maps_cancellation_to_denied_error` → `crates/vw-agent/src/tools/hooks/tests.rs`（真实宿主边界，见下）

- `permission_hook_preserves_each_decision_variant` → `crates/libs/tool-assembly/src/host_migration_34_tests/hooks.rs`（完整函数，原名与断言保留）

- `post_tool_hooks_without_runner_accept_structured_and_legacy_results` → `crates/libs/tool-assembly/src/host_migration_34_tests/hooks.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/http_headers.rs

物理行数 3 → 0。

无直接定义测试。保留文件职责：删除无调用兼容转发。

### crates/vw-agent/src/tools/http_request/mod.rs

物理行数 3 → 3。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/image_asset_read/mod.rs

物理行数 12 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/ls/mod.rs

物理行数 12 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_catalog_refresh_tests.rs

物理行数 263 → 2。

- `dynamic_tools_mcp_refresh_notification_updates_next_request` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_failure_retains_old_catalog_and_retries` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_closed_connection_never_revives_tools` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_pagination_changes_never_publish_mixed_catalog` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_unchanged_catalog_keeps_version` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_applies_current_server_authorization` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_addition_requires_explicit_authorization` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

- `dynamic_tools_mcp_refresh_disabled_source_does_not_reconnect` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`（完整函数，原名与断言保留）

### crates/vw-agent/src/tools/mcp_common/mod.rs

物理行数 300 → 300。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_common/tests.rs

物理行数 522 → 313。

- `with_server_peer_remote_transports_report_header_build_errors_before_connecting` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `global_mcp_servers_append_without_overriding_acp_servers` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `global_mcp_servers_empty_input_preserves_existing_config` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `global_mcp_servers_invalid_entry_reports_source_path` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `mcp_server_overrides_update_runtime_management` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `mcp_server_overrides_skip_unknown_servers` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `mcp_server_overrides_update_existing_entry_and_preserve_omitted_fields` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `mcp_server_overrides_create_management_when_entry_is_missing` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `mcp_server_overrides_empty_input_preserves_management` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `with_server_peer_stdio_surfaces_spawn_errors` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `with_server_peer_remote_transports_surface_connection_errors_after_client_build` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `mcp_management_helpers_apply_defaults_and_timeout` → `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`（完整函数，原名与断言保留）

- `plugin_mcp_servers_are_visible_in_resolved_acp_config` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `resolved_acp_config_from_merges_global_servers_and_management_overrides` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `resolved_acp_config_uses_current_directory_entrypoint` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

- `resolved_acp_config_keeps_project_servers_when_runtime_config_cannot_load` → `crates/vw-agent/src/tools/mcp_common/tests.rs`（真实宿主边界，见下）

### crates/vw-agent/src/tools/mcp_common/wire_http.rs

物理行数 2 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_common/wire_stdio.rs

物理行数 2 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_inspect.rs

物理行数 12 → 12。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_resource.rs

物理行数 2 → 2。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

### crates/vw-agent/src/tools/mcp_status/mod.rs

物理行数 8 → 8。

无直接定义测试。保留文件职责：真实宿主类型特化或配置/连接端口；无生产领域算法迁移收益。

## 跨目录挂载

这些测试在旧宿主通过 include 重编生产源码；现在在真实实现模块下注册 `host_migration_34_tests`，不再重编或加载宿主源码。

### crates/libs/tool-files/src/glob_search/tests.rs（19 条）

- `glob_search_name_and_schema` → `glob_search::host_migration_34_tests`

- `glob_search_single_file` → `glob_search::host_migration_34_tests`

- `glob_search_multiple_files` → `glob_search::host_migration_34_tests`

- `glob_search_recursive` → `glob_search::host_migration_34_tests`

- `glob_search_uses_relative_path_argument` → `glob_search::host_migration_34_tests`

- `glob_search_no_matches` → `glob_search::host_migration_34_tests`

- `glob_search_missing_param` → `glob_search::host_migration_34_tests`

- `glob_search_rejects_absolute_path` → `glob_search::host_migration_34_tests`

- `glob_search_rejects_path_traversal` → `glob_search::host_migration_34_tests`

- `glob_search_rejects_disallowed_relative_search_path` → `glob_search::host_migration_34_tests`

- `glob_search_rejects_dotdot_only` → `glob_search::host_migration_34_tests`

- `glob_search_filters_symlink_escape` → `glob_search::host_migration_34_tests`

- `glob_search_readonly_mode` → `glob_search::host_migration_34_tests`

- `glob_search_rate_limited` → `glob_search::host_migration_34_tests`

- `glob_search_results_sorted` → `glob_search::host_migration_34_tests`

- `glob_search_excludes_directories` → `glob_search::host_migration_34_tests`

- `glob_search_invalid_pattern` → `glob_search::host_migration_34_tests`

- `glob_search_rejects_blank_pattern_before_filesystem_access` → `glob_search::host_migration_34_tests`

- `glob_search_reports_missing_and_non_directory_search_paths` → `glob_search::host_migration_34_tests`

### crates/libs/tool-files/src/grep/tests.rs（21 条）

- `grep_name_and_schema` → `grep::host_migration_34_tests`

- `grep_finds_matches_with_include` → `grep::host_migration_34_tests`

- `grep_rejects_invalid_regex` → `grep::host_migration_34_tests`

- `grep_rate_limited` → `grep::host_migration_34_tests`

- `args_deserialize_defaults_and_glob_alias` → `grep::host_migration_34_tests`

- `validate_capabilities_rejects_each_unsupported_option` → `grep::host_migration_34_tests`

- `format_output_mode_names_every_variant` → `grep::host_migration_34_tests`

- `unique_paths_collapses_adjacent_rows_in_sorted_order` → `grep::host_migration_34_tests`

- `counts_by_path_aggregates_non_adjacent_rows_and_preserves_first_seen_order` → `grep::host_migration_34_tests`

- `expand_braces_handles_multiple_groups_and_literal_patterns` → `grep::host_migration_34_tests`

- `is_binary_detects_extensions_and_file_contents` → `grep::host_migration_34_tests`

- `grep_files_mode_applies_head_limit_and_reports_truncation` → `grep::host_migration_34_tests`

- `grep_include_with_directory_separator_matches_relative_path` → `grep::host_migration_34_tests`

- `grep_count_mode_aggregates_matches_and_honors_offset` → `grep::host_migration_34_tests`

- `grep_content_mode_supports_case_insensitive_lines_and_truncates_display_only` → `grep::host_migration_34_tests`

- `grep_skips_binary_files_and_returns_empty_structured_result` → `grep::host_migration_34_tests`

- `grep_rejects_blank_pattern_invalid_glob_missing_path_and_parent_traversal` → `grep::host_migration_34_tests`

- `grep_call_success_populates_content_rendering_and_telemetry` → `grep::host_migration_34_tests`

- `grep_call_failure_returns_structured_error_and_failed_telemetry` → `grep::host_migration_34_tests`

- `grep_content_mode_without_line_numbers_separates_files` → `grep::host_migration_34_tests`

- `grep_count_mode_reports_truncation_across_multiple_files` → `grep::host_migration_34_tests`

### crates/libs/tool-files/src/ls/tests.rs（9 条）

- `ls_name_and_schema` → `ls::host_migration_34_tests`

- `ls_lists_files` → `ls::host_migration_34_tests`

- `ls_supports_ignore_patterns` → `ls::host_migration_34_tests`

- `ls_rejects_non_array_ignore_parameter` → `ls::host_migration_34_tests`

- `ls_reports_missing_search_directory_without_listing_files` → `ls::host_migration_34_tests`

- `ls_lists_relative_subdirectory_after_resolving_workspace_path` → `ls::host_migration_34_tests`

- `ls_rejects_relative_path_blocked_by_security_policy` → `ls::host_migration_34_tests`

- `ls_rejects_existing_file_search_path` → `ls::host_migration_34_tests`

- `ls_reports_invalid_custom_ignore_pattern` → `ls::host_migration_34_tests`

### crates/libs/tool-files/src/image_asset_read/tests.rs（5 条）

- `tool_rejects_unknown_or_cross_scope_image_id` → `image_asset_read::host_migration_34_tests`

- `tool_result_never_contains_image_payload` → `image_asset_read::host_migration_34_tests`

- `pending_reload_ids_are_deduplicated_and_bounded` → `image_asset_read::host_migration_34_tests`

- `requested_image_is_attached_to_next_step_once` → `image_asset_read::host_migration_34_tests`

- `successful_reload_returns_to_text_context` → `image_asset_read::host_migration_34_tests`

### 保留宿主挂载 crates/libs/tool-mcp/src/mcp_status/tests.rs

- `mcp_status_spec_declares_serial_read_only_alias_contract`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_status_mcp_server_management_disabled_servers_are_omitted`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_status_redacts_sensitive_fields`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_capability_status_reports_unsupported_capabilities`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_status_required_server_failure_is_reported`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_status_enabled_server_reports_connected_tool_count`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `mcp_status_trims_server_filter_and_reports_management_timeout`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

### 保留宿主挂载 crates/libs/tool-mcp/src/mcp_inspect_tests.rs

- `inspect_empty_config_returns_no_servers`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `inspect_disabled_servers_preserves_management_and_transport_metadata`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `inspect_workspace_rejects_invalid_project_json`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

- `codex_plugin_compat_t58_inspection_preflights_missing_executable`：真实 AgentContextHost 配置/发现/连接边界，保留原挂载。

## 保留职责逐符号说明

- `ScopedMcpConfigHost::{load,values,merge_plugins,managed_servers,native_selection}` 读取真实租户、项目、插件及请求选择；`PluginOptionsMcpConfigHost` 读取进程全局 Config 并发现插件，保留宿主。

- `load_resolved_acp_config*`、`load_resolved_runtime_mcp_config_from` 固定真实项目和 cwd；`with_server_peer*`、`with_server_client_timeout` 选当前会话连接池；`runtime_stdio_cwd`、`open_server_peer`、`open_connection_check_peer`、`AgentMcpHost` 执行真实插件 cwd、OAuth 与 elicitation 端口。

- mcp_common 留存的三个连接错误用例验证实际 AgentMcpHost 的启动/HTTP头构建/连接错误；四个配置用例验证进程当前目录、真实全局配置读取、插件发现与失败回退。没有用领域假端口冒充这些边界。

- hooks 留存 `pre_tool_hook_maps_cancellation_to_denied_error` 验证真实 new_hook_runner 接线；其余三条纯映射迁移。

- mcp_catalog_refresh_tests 仅重导出共享 Fixture，供 session-engine/tests/host_suites 的两个真实处理器场景使用。领域目录刷新八条断言自行注册，宿主不再挂载。

- file_edit/file_write/glob/git_operations、glob_search/grep/image_asset_read/ls 保留 AgentContextHost 类型别名，实际生产调用仍由工具工厂进入原领域实现。

- http_request 为公共兼容 re-export；mcp_resource 为真实宿主泛型特化；wire_http/wire_stdio 重导出清理与子进程类型供现有宿主连接检查使用；mcp_status/mcp_inspect 保留真实宿主类型与入口及其薄边界测试。

## 量化与依赖方向

24 个来源按 splitlines：2971 → 700，净减 2271 行（76.4%）。不计跨目录用例，直接迁移 76 条。测试、夹具与转发清理为主要收益，真实生产算法迁移 0 行。

宿主 → tool-assembly/tool-files/tool-git/tool-mcp/mcp-client；领域库不反向依赖 vw-agent，不 path/include 宿主源码。目录刷新 fixture 使用随库复制的 testdata/catalog_bridge.py，领域库不读取宿主源码或测试数据。

## 未来验证（本次均未运行）

`python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_34_tests -- --test-threads=1`，静态预期 76 条（平台 cfg 可能影响）。

`python3 scripts/cargo_slot.py cargo test -p tool-files --lib host_migration_34_tests -- --test-threads=1`，应非零；精确函数全集见上。

宿主保留测试按上述完整名称逐条过滤；迁出名称通过宿主 --list 证明消失，不能将零测试当通过。所有编译、行为、性能与实机检查未运行；静态差异及注册检查不代表通过测试。

## 分项行数与静态检查

- `crates/libs/tool-assembly/src/host_migration_34_tests/host.rs`：测试/夹具 55 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/file_edit.rs`：测试/夹具 544 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_catalog.rs`：测试/夹具 263 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_host.rs`：测试/夹具 40 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/hooks.rs`：测试/夹具 38 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/file_write.rs`：测试/夹具 839 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/mcp_values.rs`：测试/夹具 214 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/git_operations.rs`：测试/夹具 72 行，生产算法 0 行。
- `crates/libs/tool-assembly/src/host_migration_34_tests/glob.rs`：测试/夹具 241 行，生产算法 0 行。
- `crates/libs/tool-files/src/glob_search/tests.rs`：测试/夹具 293 行，生产算法 0 行。
- `crates/libs/tool-files/src/grep/tests.rs`：测试/夹具 572 行，生产算法 0 行。
- `crates/libs/tool-files/src/ls/tests.rs`：测试/夹具 258 行，生产算法 0 行。
- `crates/libs/tool-files/src/image_asset_read/tests.rs`：测试/夹具 150 行，生产算法 0 行。
- `crates/libs/tool-files/src/host_migration_34_tests.rs`：测试/夹具 55 行，生产算法 0 行。

领域 tool-files 未来过滤静态预期 54 条（包含平台条件用例）。宿主本组 2971→700，净减2271行（76.4%），超过70%约束。

宿主生产端口/转发与cfg注册主要保留于 mcp_common（300行）及各类型特化；本轮未迁移生产算法。宿主测试文件中剩余 mcp_common 约313行、hooks 34行及目录fixture重导出2行。逐文件精确行数见上表，禁止将删除转发计作算法迁移。

静态完成：新旧直接用例名称全部映射、四组include挂载删除、领域模块自注册、共享Fixture消费者保持接口；git diff --check无本任务空白错误。没有运行Cargo、测试、构建或实机。
