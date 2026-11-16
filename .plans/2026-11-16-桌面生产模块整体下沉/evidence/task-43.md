# 任务43：宿主入口与残留静态收口

本轮未运行 Cargo、测试或实机。仅源码接线、断言迁移及差异检查；不代表编译或行为通过。

## 实际迁移

- `app_view.rs` 的通知定位、恒定 toast 层与壁纸布局进入 `desktop-navigation::view_layout`，真实入口调用新函数。toast 避让仍读取会话/看板跨域状态；顺序、padding、Cover 与固定层结构保持。
- 四类外链预览卡片进入 `desktop-navigation::view_layout::link_preview`（macOS 条件与旧宿主一致），解析仍用 desktop-platform；宿主提供最小链接上下文和具体消息，交接、账号/主机范围、确认和 acknowledge 时机不变。
- 预览校验、文本截断提示及全部相邻测试进入 `desktop-editor::preview::file_validation`；宿主重导出，原测试挂载删除。
- 文件索引缓存测试整组进入 file-index，缓存根改为独立临时目录；三个原断言保持。宿主目录选择仍是系统资源边界。

## 测试与断言映射

迁移前以本任务开始时 HEAD 的测试源码为映射基准；此前领域迁移见各 task-N 永久证据。未改动的宿主测试原路径、名称、断言保持，包含真实字体、布局、事件和滚动测试。新增布局构造测试只补领域入口，不替代宿主集成覆盖。

- `crates/vw-desktop/src/app/files_tests.rs` → `crates/libs/file-index/src/host_module_43_tests.rs`：`refresh_file_index_saves_cache_and_load_file_index_reads_cached_files`；原 assert 宏 3 个，新位置 3 个。
- `crates/vw-desktop/src/app/preview_tests.rs` → `crates/libs/desktop-editor/src/preview/file_validation_tests.rs`：`preview_open_error_allows_utf8_text_files`, `preview_open_error_blocks_binary_files`, `safe_preview_text_cleans_wraps_and_returns_empty_for_missing_files`, `safe_preview_text_marks_files_over_the_byte_limit_as_truncated`, `preview_open_error_rejects_unsupported_extensions_before_reading`；原 assert 宏 8 个，新位置 8 个。

## 逐文件、函数与字段组归属

表中函数列表含测试辅助函数。无函数文件为注册/类型重导出；状态字段组仍由 `state.rs` 注册既有领域类型，App 保留领域状态与系统资源，不复制 App。初始化只组装各领域状态，消息根只注册路由和执行器适配。

| 基线文件（相对 src） | 当前行数 | 函数/测试入口 | 归属与保留边界 |
|---|---:|---|---|
| `app/assets/mod.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_assets |
| `app/components/agent_apps_high_availability/health.rs` | 20 | `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/agent_apps_high_availability/mod.rs` | 4 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/components/agent_apps_high_availability/policy_modal.rs` | 29 | `message`, `overlay` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/agent_apps_high_availability/queue.rs` | 29 | `message`, `overlay` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/aggregation_list.rs` | 5 | `overlay` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/animated_text.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/components/app_routing/config.rs` | 6 | `overlay`, `timeout_button` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/app_routing/lifecycle.rs` | 29 | `message`, `context`, `current`, `update`, `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/app_routing/mod.rs` | 5 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/components/app_routing/outbound.rs` | 6 | `overlay`, `entry` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/app_routing/requests.rs` | 6 | `overlay`, `entry` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/components/archived_chats_window/native.rs` | 156 | `send`, `update`, `view`, `run` | 真实 App 消息往返、窗口截图和验收进程资源；场景和夹具归 sessions |
| `app/components/archived_chats_window_tests.rs` | 11 | `archived_chats_native_acceptance_has_explicit_isolated_entry` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/components/cli_command_help.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_navigation |
| `app/components/code_review.rs` | 30 | `view`, `build_code_review_view_from_diff`, `git_diff_by_file` | session 网关变更记录转换成 Git UI 输入并映射全局消息 |
| `app/components/code_review_tests.rs` | 31 | `git_diff_by_file_without_project_returns_empty_collections`, `git_diff_by_file_with_unavailable_project_records_returns_empty_collections`, `view_without_project_constructs_empty_diff_state` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/components/editor.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_editor |
| `app/components/editor_toolbar.rs` | 12 | `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_editor |
| `app/components/input_mention_highlighter.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_editor |
| `app/components/markdown_editor.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_editor |
| `app/components/mcp_app_panel.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_web_runtime |
| `app/components/mod.rs` | 497 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/components/number_input.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/components/search_panel.rs` | 28 | `data`, `overlay`, `view` | workspace 文件缓存、session 标题与导航状态注入 search 视图 |
| `app/components/search_panel_tests.rs` | 40 | `session`, `search_view_content_len`, `view_groups_file_project_and_session_matches` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/components/startup_transition.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_navigation |
| `app/components/status_animation.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/components/svg_path.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/components/tab_bar.rs` | 9 | `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_navigation |
| `app/components/text_editor_context_menu.rs` | 14 | `text_context_message` | 跨域输入/消息/反馈映射；局部实现调用 desktop_ui_controls |
| `app/components/text_editor_scroll_panel.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/components/widgets.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_ui_controls |
| `app/deep_links.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_platform |
| `app/files.rs` | 17 | `load_file_index`, `refresh_file_index` | 应用数据目录所有权；索引扫描/缓存归 file-index |
| `app/files_tests.rs` | 0 | — | 测试随完整领域行为迁出，见上方映射 |
| `app/git.rs` | 71 | `backend_shell`, `git_stage_file`, `git_commit`, `git_commit_with_body`, `git_discard_hunk`, `git_stage_hunk`, `git_stage_line_insert`, `git_stage_line_delete` | 终端 Shell 到 Git 后端的跨域类型映射 |
| `app/impls/app_init.rs` | 36 | — | 模块注册或类型重导出；实现位于 desktop_apps_ui, desktop_settings_agents, desktop_workspace |
| `app/impls/app_update_tests.rs` | 718 | `test_app`, `startup_cli_service_result_controls_main_interface_gate`, `apply_project_chat_preferences_preserves_active_session_and_updates_draft`, `startup_agent_config_reload_rehydrates_gateway_backed_settings`, `bootstrap_app_config_updates_known_fields_and_clamps_file_manager_width`, `workspace_mode_bootstrap_preserves_current_mode_for_invalid_values`, `workspace_mode_bootstrap_accepts_work_mode`, `startup_app_config_loaded_uses_same_bootstrap_path`, `bootstrap_app_config_preserves_invalid_values_and_clamps_minimum_width`, `bootstrap_browser_config_normalizes_invalid_options`, `bootstrap_browser_config_accepts_supported_case_insensitive_options`, `bootstrap_browser_config_accepts_remaining_supported_modes`, `bootstrap_system_settings_updates_runtime_timezone_and_normalizes_active_gateway`, `bootstrap_system_settings_applies_lsp_enabled_state`, `app_bootstrap_preserves_custom_items_and_adds_missing_builtin_git_actions`, `failed_bootstrap_results_preserve_existing_state`, `update_loads_acp_agents_and_archived_sessions`, `update_handles_external_apps_and_session_preview_errors`, `session_previews_loaded_builds_only_non_empty_content`, `gateway_health_checked_retains_known_servers_and_ignores_unknown_results`, `selected_gateway_health_transition_triggers_only_when_becoming_healthy`, `copy_code_records_content_hash_without_executing_clipboard_task`, `copy_file_missing_path_preserves_clipboard_feedback`, `copy_feedback_expiration_only_clears_matching_hash_and_close_error_clears_message`, `batch_message_applies_each_nested_update_in_order`, `app_update_bootstrap_restores_source_without_overwriting_user_selection`, `app_update_late_bootstrap_preserves_checked_release`, `bootstrap_reasoning_effort_restores_open_history_and_preserves_local_default_selection`, `native_startup_rejects_an_old_snapshot_after_switching_tenants_and_back`, `startup_model_backfills_an_open_project_until_the_project_has_its_own_choice`, `startup_chat_defaults_preserve_an_unsubmitted_home_selection`, `input_model_provider_collapse_restores_and_preserves_local_edits` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/impls/app_view.rs` | 288 | `view_window`, `main_window_view_kind`, `view`, `show_tab_bar` | 窗口/页面选择、跨域浮层顺序和状态注入；具体布局在导航库 |
| `app/impls/app_view_status.rs` | 23 | `status_bar` | 跨域输入/消息/反馈映射；局部实现调用 desktop_editor, desktop_navigation |
| `app/impls/app_view_status_tests.rs` | 257 | `app_view_status_tests_module_is_wired`, `preview_tab`, `main`, `status_bar_builds_ready_when_no_preview_tab_is_active`, `status_bar_builds_ready_and_global_status_for_tab_without_server_key`, `status_bar_builds_ready_when_active_path_has_no_matching_tab`, `status_bar_builds_active_tab_lsp_variants`, `status_bar_builds_global_status_when_server_has_no_progress_map`, `status_bar_builds_server_key_when_progress_map_is_empty`, `status_bar_builds_running_progress_with_missing_optional_fields`, `status_bar_builds_overcomplete_progress_as_completion`, `status_bar_builds_notification_variant`, `status_bar_builds_multiple_notification_count`, `status_bar_uses_reserved_height`, `status_bar_keeps_notification_at_trailing_edge`, `status_bar_builds_account_entry_and_open_menu` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/impls/app_view_tests.rs` | 277 | `test_app`, `main_window_uses_startup_transition_until_service_is_ready`, `view_window_routes_task_pet_window_to_pet_view`, `view_builds_for_primary_screens_without_modal_state`, `view_builds_preview_screen`, `view_builds_workflow_tool_screen`, `view_builds_with_settings_notifications_error_and_toast_overlays`, `toast_show_replace_and_dismiss_preserve_acp_scroll_offsets_and_viewports`, `traverse`, `scrollable`, `view_builds_with_about_cli_search_and_rename_overlays`, `view_builds_with_plain_cli_install_modal`, `view_builds_with_project_edit_and_permission_modals_together`, `view_builds_shiju_install_guide`, `project_shell_keeps_tab_bar_in_code_and_work_modes`, `mini_companion_routes_none_avatar_without_changing_main_window` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/impls/mod.rs` | 36 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/impls/tests.rs` | 8 | `tests_module_is_wired` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/lsp/install.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_lsp |
| `app/lsp/mod.rs` | 18 | — | 模块注册或类型重导出；实现位于 desktop_lsp |
| `app/lsp/tests.rs` | 8 | `tests_module_is_wired` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/mcp_app_bridge.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_web_runtime |
| `app/mcp_app_download.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_web_runtime |
| `app/mcp_form.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_web_runtime |
| `app/mcp_link_preview.rs` | 41 | `handoff`, `overlay` | 仅 launch inbox 到目标设置域的确认交接；卡片在导航库 |
| `app/message/agent_apps_auth/binding.rs` | 6 | `entry` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/agent_apps_auth/flow.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_provider_apps |
| `app/message/agent_apps_diagnostics/endpoint_state.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_apps_ui |
| `app/message/agent_apps_diagnostics/endpoints.rs` | 34 | `message`, `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/agent_apps_diagnostics/mcp_results.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_apps_ui |
| `app/message/agent_apps_diagnostics/mcp_state.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_apps_ui |
| `app/message/agent_apps_diagnostics/model.rs` | 34 | `message`, `invalidate`, `can_show`, `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/agent_apps_diagnostics/model_state.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_apps_ui |
| `app/message/agent_apps_diagnostics.rs` | 36 | `message`, `context`, `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui, desktop_settings_mcp |
| `app/message/agent_apps_health.rs` | 49 | `message`, `current`, `visible`, `update`, `selected` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/agent_apps_high_availability.rs` | 39 | `message`, `current`, `can_show`, `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/agent_apps_high_availability_policy.rs` | 38 | `message`, `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_apps_ui |
| `app/message/editor/metrics.rs` | 56 | `applied`, `displayed`, `commit`, `apply_snapshot`, `apply_to_open_editors` | 编辑器策略归 desktop-editor；此处同步预览与 Git 编辑器并持久化 |
| `app/message/editor/persistence.rs` | 23 | `enqueue`, `save_complete` | 启动编辑标记、网关 endpoint 注入和 toast 跨域装配 |
| `app/message/gateway_health.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_settings_network |
| `app/message/knowledge_tool.rs` | 36 | `gateway`, `map_event`, `apply_feedback`, `update`, `ensure_loaded` | 跨域输入/消息/反馈映射；局部实现调用 desktop_expert_hub |
| `app/message/mod.rs` | 271 | `after`, `spawn_blocking_opt`, `sleep` | 消息注册与 Iced/native/wasm 执行器适配 |
| `app/message/notification.rs` | 23 | `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_navigation |
| `app/message/search/mod.rs` | 28 | `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_search |
| `app/message/search/tests.rs` | 181 | `chat_message`, `app`, `session_info`, `collect_task_messages`, `task_contains_widget_action`, `input_changed_refreshes_cache_without_closing_overlay_for_blank_query`, `open_clears_previous_query_shows_overlay_and_focuses_input`, `select_file_sets_file_input_and_hides_overlay`, `select_session_with_known_directory_routes_to_project_session`, `child_session_navigation_search_without_directory_defers_to_project_route`, `select_project_hides_overlay_before_returning_open_task` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/message/task_board.rs` | 41 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/message/task_board_tests.rs` | 11 | `test_module_is_wired` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/message/tests.rs` | 64 | `collect_task_messages`, `after_emits_the_original_message_after_sleeping`, `spawn_blocking_opt_returns_closure_value`, `spawn_blocking_opt_preserves_explicit_none`, `spawn_blocking_opt_maps_panicking_task_to_none` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/mod.rs` | 204 | `project_dirs` | 注册/重导出及应用配置目录命名空间（系统资源） |
| `app/preview.rs` | 41 | — | 模块注册或类型重导出；实现位于 desktop_editor |
| `app/preview_tests.rs` | 0 | — | 测试随完整领域行为迁出，见上方映射 |
| `app/prompt_link_preview.rs` | 55 | `handoff`, `overlay` | 仅 launch inbox 到目标设置域的确认交接；卡片在导航库 |
| `app/provider.rs` | 30 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/provider_link_preview.rs` | 16 | `overlay` | 仅 launch inbox 到目标设置域的确认交接；卡片在导航库 |
| `app/provider_tests.rs` | 9 | `test_module_is_wired` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/single_instance.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_platform |
| `app/skill_hub/cache_sync.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/client.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/connectors.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/installer.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/knowledge.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/mod.rs` | 57 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/skill_hub/models.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/native.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/package_version_cache.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_skill_hub |
| `app/skill_hub/shiju_knowledge_state_tests.rs` | 59 | `shiju_knowledge_state_messages_reject_active_credential_change_and_redact`, `chunk` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/skill_hub/state.rs` | 9 | `clear_expert_hub_account` | 跨域输入/消息/反馈映射；局部实现调用 desktop_skill_hub |
| `app/skill_hub/state_tests.rs` | 11 | `expert_hub_account_clear_resets_shiju_apps` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/skill_link_preview.rs` | 52 | `handoff`, `overlay` | 仅 launch inbox 到目标设置域的确认交接；卡片在导航库 |
| `app/state.rs` | 89 | — | 模块注册或类型重导出；实现位于 desktop_provider_apps, runtime_tools |
| `app/tests.rs` | 20 | `test_module_is_wired`, `project_dirs_points_at_current_application_namespace` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `app/theme_catalog/mod.rs` | 14 | `sync_active_theme_skin`, `clear_active_theme_skin` | 跨域输入/消息/反馈映射；局部实现调用 desktop_theme |
| `app/views/home/common.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_navigation |
| `app/views/home/mod.rs` | 32 | `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_navigation |
| `app/views/knowledge.rs` | 19 | `view` | 跨域输入/消息/反馈映射；局部实现调用 desktop_expert_hub, desktop_ui_controls |
| `app/views/mod.rs` | 61 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `app/views/workflow_manager/mod.rs` | 54 | `client`, `map_message`, `load_lifecycle_workflows_task`, `view`, `update_app` | 跨域输入/消息/反馈映射；局部实现调用 desktop_ui_controls, desktop_workflow_ui |
| `app/web_install/mod.rs` | 23 | `update` | 跨域输入/消息/反馈映射；局部实现调用 desktop_web_install |
| `app/webview_runtime.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_web_runtime |
| `apps/mod.rs` | 3 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `desktop_update_helper.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_update, desktop_update_helper |
| `fonts.rs` | 2 | — | 模块注册或类型重导出；实现位于 desktop_assets |
| `legacy_task_board_switches_exact_tests.rs` | 24 | `legacy_task_board_switches_are_ignored_after_capability_cutover` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `lib.rs` | 53 | — | 模块注册或类型重导出；实现位于 已注册领域子模块 |
| `main.rs` | 275 | `main_window_platform_settings`, `main_window_settings`, `configure_native_graphics_backend`, `init_native_tracing`, `native_tracing_filter`, `should_suppress_native_stderr_line`, `install_macos_native_stderr_filter`, `default_font`, `iced_settings`, `boot_app`, `main` | 进程启动、系统 stderr 文件描述符/环境、字体和窗口生命周期装配 |
| `main_tests.rs` | 211 | `set`, `remove`, `drop`, `main_window_settings_use_expected_desktop_defaults`, `main_window_settings_target_expected_dom_element_on_wasm`, `main_window_platform_settings_enable_macos_chrome_integration`, `main_window_platform_settings_keep_non_macos_defaults`, `native_tracing_filter_uses_safe_defaults_when_rust_log_is_absent`, `native_tracing_filter_honors_valid_rust_log_directives`, `native_tracing_filter_uses_safe_defaults_for_invalid_rust_log`, `native_stderr_filter_suppresses_macos_imk_run_loop_noise`, `native_stderr_filter_keeps_unrelated_stderr_lines`, `default_font_uses_platform_family_name`, `iced_settings_include_default_font_and_embedded_fonts`, `channel_qr_flow_stops_on_close_and_fills_confirmed_credentials`, `windows_graphics_backend_forces_dx12_over_inherited_settings` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |
| `native_crash.rs` | 1 | — | 模块注册或类型重导出；实现位于 desktop_platform |
| `tests.rs` | 7 | `module_test_anchor` | 保留宿主调用/跨域状态契约及原断言；局部实现已调用领域入口 |

## 静态证据与限制

- `git diff --check`：通过。修改文件独立 rustfmt 解析，未调用 Cargo。
- 新领域源码不引用 `crate::app`，无反向 path/include、无 App 副本，无新增依赖或总控库。
- 未更改任何既有字体、滚动、事件与绘制测试；新增测试本轮未运行。
- 本任务不代表任务44整体审计或计划整体完成；其它工作区改动未纳入提交。
- 以后获准验证时：`python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_43_tests`；`python3 scripts/cargo_slot.py cargo test -p desktop-editor --lib file_validation`；`python3 scripts/cargo_slot.py cargo test -p file-index --lib host_module_43_tests`。这些命令未执行。
