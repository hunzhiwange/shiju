# 任务16：静态交付证据

已完成本任务四组页面领域归属、入口接线和领域断言迁移。未编译、未测试；用户禁止的 Cargo、测试、绘制及回归均未执行。

## 真实生产调用链

- App 已持有领域 AgentsSettingsState/AcpSettingsState 和 security 领域 AutonomySettingsState；state/settings/agents.rs、infrastructure.rs、app_init/new.rs 已使用真实类型，本任务不复制状态或新增初始化算法。
- settings::agents/acp/autonomy/dialogue_flow 仍调用既有 agents_update/acp_update/autonomy_update/dialogue_flow_update；宿主只提供配置端点、生命周期及跨域效果。
- profile_modal::overlay → profile_modal::page_card：五页分派、native/ACP 边界和表单输入归领域；宿主只投影会话 capability、MCP、时钟和项目路径，承接 modal 生命周期。
- defaults::catalog_input 为生产 modal 和保存校验复用唯一目录投影，继续调用既有 validation_error/validate。
- Event 补齐 OpenUrlExternal/AppendText，使实际帮助菜单具名效果完整接入已有 URL/聊天追加入口。
- 图标原样移至 desktop-ui-controls::icons；desktop-chat-input 保留 re-export，宿主和领域用同一实现。移除新增的 desktop-chat-input 测试依赖，避免新增开发期回路。
- 静态解析发现前轮遗留的损坏 AllowedToolsApplyPreset 分支名，按真实枚举恢复；refresh_task 仅开放 crate 可见性供真实 HTTP 用例使用。

## 局部夹具与依赖

host_module_16_tests 独立注册24个子模块、86个源码用例；这不是运行通过数。ProfileFixture、AgentsControllerFixture、PolicyFixture、AcpPageFixture、DialogueFixture 只持有真实领域状态与局部输入，没有复制 App/AppState。

HTTP 夹具保留协议解析、异步响应和请求断言，使用显式 endpoint/ConfigAccess/GatewayClient；临时存储回读保持，不再改变进程环境。混合场景使用实际 MCP State/page/configuration_update、security view/update、quick_messages view/update；MCP 外部管理面板是未被测内容槽，不模拟配置算法。

新增开发依赖 appearance/mcp/image/urlencoding 和 tokio net/io-util；appearance/mcp 沿本地生产依赖传递检查没有返回 agents 的路径。既有 desktop-chat-view 开发依赖及 chat-tools→agents 路径继承前轮，未新增该循环。生产库不依赖宿主或测试源码。

## 前轮去重

- host_split_2 已持有 gap_11 权限、列表、通道的完整领域绘制与复制断言；同名宿主方法只保留 Local→宿主映射，本任务未再次复制三个领域用例。
- desktop_split_12 及子模块的局部构造/字段纯逻辑不重复迁移；本次保留真实 renderer/layout/click/draw/screenshot 和资料事务场景。
- desktop_split_17 的 ACP 空态/排序/构造与本次全状态动画、长表单像素断言不同；desktop_split_28 的纯转换与本次真实 HTTP、乱序目录响应、持久化事务断言不同。
- 本任务新增用例名与既有用例无重复挂载；原来源测试名逐项映射如下。

| 原来源 | 原用例 | 当前归属 |
|---|---|---|
| `system_settings_acp_tests.rs` | `acp_status_cards_and_editor_render_all_results_and_animate_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `system_settings_acp_tests_are_wired` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `system_settings_acp_page_keeps_core_actions` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `acp_catalog_displays_registered_agents_render_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `acp_editor_validates_and_saves_fields_render_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `acp_diagnostic_ignores_stale_results_render_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_tests.rs` | `acp_configuration_flow_regression_long_editor_keeps_actions_visible` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_view.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `ui_refresh_18_renders_catalogs_and_forms` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `ui_refresh_18_layout_and_interaction` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `ui_refresh_18_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `help_rollout_09_renders_contextual_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `help_rollout_09_mcp_server_and_policy_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `settings_help_gap_13_mcp_edit_heading_moves_preservation_rule` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `settings_help_gap_13_mcp_add_and_edit_transport_fields` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `system_settings_acp_ui_refresh_18_tests.rs` | `settings_help_gap_13_acp_diagnostic_draft_and_install_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/acp_ui.rs` |
| `basic_profile_ui_refresh_15_tests.rs` | `ui_refresh_15_layout_and_interaction` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_ui.rs` |
| `basic_profile_ui_refresh_15_tests.rs` | `agent_profile_modal_preserves_catalog_scroll` | `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile_ui_refresh_15_tests.rs` |
| `basic_profile_ui_refresh_15_tests.rs` | `ui_refresh_15_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `ui_refresh_16_layout_and_interaction` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `ui_refresh_16_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `help_rollout_08_renders_contextual_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `help_rollout_08_basic_help_uses_static_text` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `help_rollout_08_identity_and_tools_keep_controls_visible` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `settings_help_gap_05_native_modal_moves_explanations_into_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `settings_help_gap_05_acp_model_capability_controls_choices` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `basic_ui_refresh_16_tests.rs` | `settings_help_gap_05_fixed_role_and_tool_controls_remain_visible` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/basic_ui.rs` |
| `catalog_tests.rs` | `agent_catalog_view_failed_toggle_preserves_snapshot_and_main` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_transactions.rs` |
| `catalog_tests.rs` | `agent_catalog_view_copy_keeps_profile_with_new_custom_identity` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_transactions.rs` |
| `catalog_tests.rs` | `agent_catalog_view_renders_long_chinese_cards_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_transactions.rs` |
| `catalog_tests.rs` | `desktop_first_response_catalog_launch_preserves_profile_in_a_draft` | `crates/vw-desktop/src/app/components/system_settings_agents/catalog_tests.rs` |
| `catalog_tests.rs` | `agent_catalog_keeps_two_columns_and_scrollbar_gutter` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_transactions.rs` |
| `defaults_tests.rs` | `agent_defaults_form_switch_preserves_invalid_draft_and_prevents_save` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/defaults.rs` |
| `defaults_tests.rs` | `agent_defaults_form_forced_binding_rejects_conflicting_default` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/defaults.rs` |
| `defaults_tests.rs` | `agent_config_cleanup_single_model_group_prevents_conflict` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/defaults.rs` |
| `defaults_tests.rs` | `agent_config_cleanup_strict_skill_defaults_reject_save_outside_boundary` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/defaults.rs` |
| `defaults_tests.rs` | `agent_model_inheritance_renders_chinese_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/defaults.rs` |
| `defaults_tests.rs` | `agent_model_inheritance_verified_acp_models_survive_disconnect_and_reopen` | `crates/vw-desktop/src/app/components/system_settings_agents/defaults_tests.rs` |
| `mcp_tests.rs` | `agent_config_cleanup_mcp_tab_uses_stable_server_ids` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/mcp.rs` |
| `mcp_tests.rs` | `agent_config_cleanup_mcp_unavailable_selection_is_explicit` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/mcp.rs` |
| `mcp_tests.rs` | `agent_config_cleanup_mcp_renders_chinese_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/mcp.rs` |
| `mod_tests.rs` | `agents_view_builds_default_loading_and_error_states` | `crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs` |
| `mod_tests.rs` | `agent_config_tabs_render_each_existing_settings_page` | `crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs` |
| `mod_tests.rs` | `agents_error_banner_follows_agent_tabs` | `crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs` |
| `mod_tests.rs` | `agents_view_builds_empty_entries_fallback` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/page_states.rs` |
| `mod_tests.rs` | `agents_view_routes_each_detail_tab` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/page_states.rs` |
| `mod_tests.rs` | `agents_view_builds_main_enabled_disabled_and_model_metadata` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/page_states.rs` |
| `mod_tests.rs` | `settings_tabs_capabilities_appearance_agents` | `crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs` |
| `profile_modal_tests.rs` | `agent_profile_modal_cancel_preserves_saved_profile` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_profile_modal_failed_save_preserves_draft` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_profile_modal_builtin_rejects_engine_switch_and_delete` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_profile_modal_prompts_preserve_user_order` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_config_cleanup_five_tabs_render_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_render.rs` |
| `profile_modal_tests.rs` | `agent_config_cancel_discards_parameters_and_profile_together` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_config_create_commits_only_after_successful_save` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_config_cleanup_skill_directory_renders_chinese_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_render.rs` |
| `profile_modal_tests.rs` | `agent_config_cleanup_basic_combines_profile_and_runtime` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_config_cleanup_single_system_prompt_preserves_content` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_config_cleanup_multiline_description_roundtrips_and_cancels` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_runtime_creation_success_locks_runtime_but_keeps_profile_editable` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_runtime_creation_failed_save_allows_retry_and_cancel_leaves_no_entry` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_runtime_creation_reloaded_custom_and_builtin_reject_changes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_runtime_creation_empty_directory_refreshes_and_failure_preserves_native` | `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal_tests.rs` |
| `profile_modal_tests.rs` | `agent_runtime_creation_controls_render_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_render.rs` |
| `profile_modal_tests.rs` | `agent_runtime_visibility_switch_closes_and_rejects_tools_tab_without_clearing_values` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_update.rs` |
| `profile_modal_tests.rs` | `agent_runtime_visibility_actual_modal_renders_native_and_acp_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/profile_render.rs` |
| `profile_modal_tests.rs` | `settings_tabs_capabilities_appearance_profile` | `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal_tests.rs` |
| `skills_tests.rs` | `agent_config_cleanup_skill_modes_render_chinese_in_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/skills.rs` |
| `tests.rs` | `agents_view_loading_state_adds_status_row` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_view.rs` |
| `tests.rs` | `agents_view_builds_cards_for_main_enabled_disabled_and_model_variants` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_view.rs` |
| `tests.rs` | `agents_view_missing_selection_uses_first_entry_and_unknown_tab_falls_back` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_view.rs` |
| `tests.rs` | `session_entry_catalog_cards_show_icons_and_start_session` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_view.rs` |
| `tests.rs` | `session_entry_catalog_cards_render_both_themes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/catalog_view.rs` |
| `system_settings_autonomy_tests.rs` | `settings_tabs_core_appearance_autonomy` | `crates/vw-desktop/src/app/components/system_settings_autonomy_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `ui_refresh_22_layout_and_interaction` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `ui_refresh_22_sandbox_select_keeps_policy_values_from_keyboard` | `crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `ui_refresh_22_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids` | `crates/libs/desktop-settings-agents/src/host_split_3_tests.rs`<br>`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_11_permissions_explanations` | `crates/libs/desktop-settings-agents/src/host_split_2_tests.rs`<br>`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_11_list_editor_explanations` | `crates/libs/desktop-settings-agents/src/host_split_3_tests.rs`<br>`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_11_channel_formats` | `crates/libs/desktop-settings-agents/src/host_split_2_tests.rs`<br>`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `help_rollout_14_renders_contextual_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `help_rollout_14_security_renders_contextual_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_12_sandbox_modes` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_12_bash_inactive_checks` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_autonomy_ui_refresh_22_tests.rs` | `settings_help_gap_12_audit_otp_and_syscall_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/policy_ui.rs` |
| `system_settings_dialogue_flow_ui_refresh_29_tests.rs` | `ui_refresh_29_layout_and_interaction` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_ui.rs` |
| `system_settings_dialogue_flow_ui_refresh_29_tests.rs` | `ui_refresh_29_preserves_values_and_disabled_states` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_ui.rs` |
| `system_settings_dialogue_flow_ui_refresh_29_tests.rs` | `help_rollout_20_renders_contextual_help` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_ui.rs` |
| `acp_tests.rs` | `update_ignores_non_acp_settings_messages` | `crates/vw-desktop/src/app/message/settings/acp_tests.rs` |
| `agents_tests.rs` | `agent_unified_selection_catalog_launch_selects_profile_runtime` | `crates/vw-desktop/src/app/message/settings/agents_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_skill_directory_preserves_global_draft` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_global_mutations_do_not_use_skill_directory` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_stale_skill_catalog_is_ignored` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_profile_http_roundtrips_multiline_and_runtime` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_skill_selection_writes_only_one_default` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_config_cleanup_skill_selection_preserves_inherit_and_rejects_unavailable` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `session_entry_catalog_launch_preserves_profile_runtime` | `crates/vw-desktop/src/app/message/settings/agents_tests.rs` |
| `agents_tests.rs` | `agent_tool_catalog_async_survives_provider_failure` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `agent_tool_catalog_async_retries_on_profile_open` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `agents_tests.rs` | `saved_config_stays_clean_after_discard_refresh_in_either_response_order` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/agents_controller_tests.rs` |
| `dialogue_flow_tests.rs` | `dialogue_flow_loaded_messages_update_ui_state` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_update.rs` |
| `dialogue_flow_tests.rs` | `dialogue_flow_toggle_and_save_feedback_update_messages` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_update.rs` |
| `dialogue_flow_tests.rs` | `action_messages_schedule_follow_up_work` | `crates/vw-desktop/src/app/message/settings/dialogue_flow_tests.rs` |
| `dialogue_flow_tests.rs` | `loaded_success_preserves_existing_feedback_and_loaded_error_preserves_toggles` | `crates/libs/desktop-settings-agents/src/host_module_16_tests/dialogue_update.rs` |
| `dialogue_flow_tests.rs` | `unrelated_message_leaves_dialogue_flow_state_unchanged_and_stays_idle` | `crates/vw-desktop/src/app/message/settings/dialogue_flow_tests.rs` |
| `dialogue_flow_tests.rs` | `dialogue_flow_patch_failure_reaches_localized_feedback_and_queued_edits_save` | `crates/vw-desktop/src/app/message/settings/dialogue_flow_tests.rs` |

拆分补充：UI15首例的父级标签/错误横幅部分保留为 ui_refresh_15_host_tabs_and_errors；UI22首例的原生设置窗口/焦点部分保留为 ui_refresh_22_host_security_viewport；UI29第二例的 agents 父页滚动部分保留为 ui_refresh_29_host_quick_page_scrolls。Profile 空目录 runtime 原断言归 agent_runtime_empty_directory_keeps_native，宿主只保留 ACP 刷新/失败回调。

## 残留职责审计

| 来源 | 保留理由 |
|---|---|
| `crates/vw-desktop/src/app/components/system_settings_acp.rs` | view只借用外部输入并映射领域视图。 |
| `crates/vw-desktop/src/app/components/system_settings_acp_tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_acp_ui_refresh_18_tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/basic.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/basic_profile_ui_refresh_15_tests.rs` | 仅父级跨页导航、App消息和最外层Iced缓存/滚动恢复；renderer/Scene为真实窗口契约服务。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/basic_ui_refresh_16_tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/catalog_tests.rs` | 仅CatalogLaunch→真实会话草稿/runtime/navigation；wire fixture为该跨域契约所需。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/defaults.rs` | 只把跨域目录局部投影给已有校验，没有第二套校验。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/defaults_tests.rs` | Chat ACP响应→实际会话能力→defaults_input投影契约；使用真实Chat回调。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/domain.rs` | event跨域路由；icon共享资源；defaults/catalog/acp input只借用真实外部状态；help桥接宿主菜单效果。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/identity.rs` | 唯一函数把系统时钟投影为相对时间，再调用领域格式化。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/mcp.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/mcp_tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/mod.rs` | 描述符/父页view只装配其它设置域；active_tab_save_error选择不同领域错误；agents_view和agent_project_options只投影转发。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/mod_tests.rs` | 跨页装配、跨域错误横幅、General真实像素参照及App路由；纯agents空态/detail/元数据已迁库。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal.rs` | 投影真实会话capability、MCP、时钟与项目路径；overlay只承担宿主modal生命周期。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/profile_modal_tests.rs` | Agents→ACP刷新/失败回调，以及General真实像素参照下的消息投递/选中态跨域契约。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/shared.rs` | 仅父级跨域导航的统一tab适配。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/skills.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/skills_tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/tests.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_agents/tools.rs` | 已删除旧领域实现/测试包装，断言和注册迁入领域。 |
| `crates/vw-desktop/src/app/components/system_settings_autonomy.rs` | Local→宿主消息、实际工具/主题输入和Iced overlay生命周期。 |
| `crates/vw-desktop/src/app/components/system_settings_autonomy_tests.rs` | 与General标签真实像素样式的跨域参照。 |
| `crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs` | 仅原生设置窗口/焦点scope、窄窗口跨页装配及Local→宿主映射；领域布局已迁库。 |
| `crates/vw-desktop/src/app/components/system_settings_dialogue_flow.rs` | 仅实际字段借用与Local消息映射。 |
| `crates/vw-desktop/src/app/components/system_settings_dialogue_flow_ui_refresh_29_tests.rs` | 仅agents根页包裹quick子页的外层滚动契约；renderer/geometry服务该Iced生命周期。 |
| `crates/vw-desktop/src/app/message/settings/acp.rs` | 配置端点、Startup标记、延时生命周期与toast装配。 |
| `crates/vw-desktop/src/app/message/settings/acp_tests.rs` | 非ACP消息不能穿透adapter的路由契约。 |
| `crates/vw-desktop/src/app/message/settings/agents.rs` | 配置资源、Toast/Startup、ACP/MCP转发及真实session draft/runtime/navigation效果；唯一业务入口agents_update。 |
| `crates/vw-desktop/src/app/message/settings/agents_tests.rs` | 仅两项CatalogLaunch→实际session/runtime/navigation契约。 |
| `crates/vw-desktop/src/app/message/settings/autonomy.rs` | 局部消息映射与App持久化入口；转换全在autonomy_update。 |
| `crates/vw-desktop/src/app/message/settings/dialogue_flow.rs` | 局部消息映射和跨页系统设置patch队列；更新全在dialogue_flow_update。 |
| `crates/vw-desktop/src/app/message/settings/dialogue_flow_tests.rs` | 外部效果调度、非本域路由拒绝、真实统一patch队列失败/重试/本地化回调；纯状态已迁库。 |

状态组：agents/acp/autonomy继续用原领域类型；dialogue布尔/editor/反馈由既有Update局部借用；desktop_preferences_persistence是跨页共享队列，不复制到领域。没有新增App字段或巨型trait。

## 同口径统计与审查

- baseline.json来源8772行 → 当前2372行，宿主来源净减6400行；没有新建宿主替代文件。口径为pathlib splitlines，含空行注释。
- 文件名生产候选832行 → 713行，候选净减119行。候选只是文件名初筛，不把测试净减声称为生产净减；新page_card/catalog_input和共享图标均有真实生产调用者。
- 已核对106个原用例名归属、mod/cfg注册、完整view/update调用者及传递依赖；无宿主path/include、App/AppState复制。
- 使用独立rustfmt解析本任务修改源码；agents_update的既有语法损坏修正后可解析。git diff --check审查本任务路径及提交差异。

## 未验证与以后命令

- 未执行Cargo、测试、字体整形、draw/screenshot或权限验收；独立rustfmt不证明类型或运行正确。
- 原布局树下标、子节点数量、菜单动作和screenshot断言保留。本轮不根据静态猜测弱化断言。security/autonomy/dialogue原帮助回调对非复制动作的None处理在相应局部夹具中保持；未来菜单搜索断言可能揭示原有边界缺口，没有伪造消息绕过。
- 新共享icons真实渲染未执行；保留暗黑SVG着色、中文字体、4px滚动条与7px间隔的实现和绘制断言。
- 仅以后用户授权才运行：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-agents --lib host_module_16_tests -- --test-threads=1`；宿主检查为`python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`，统一阶段按版本合并；本轮不启动。
