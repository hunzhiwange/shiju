# 任务40：runtime_command 静态迁移证据

代码完成，编译/行为未验证。按公共计划禁止 Cargo、构建、单元/实机测试；仅源码、注册、依赖及差异静态核对。未宣称编译收益。

## 真实入口与依赖

生产实现此前已在 channel-runtime::runtime_command，宿主 handler 调用 manager::command_host() 注入真实 CommandHost。此次生产迁移收益为 0；原薄端口和兼容导出保留。channel-runtime → channel-support/session-engine/agent-*，无反向 vw-agent 依赖，无 path/include 加载宿主。新增模块直接调用领域 handler，CommandPort 仅提供 tenant 和 scope 执行；会话/任务端口一旦触达即 panic，不能充当真实权限或存储验收。使用既有 ChannelRuntimeContext 类型、真实 ApprovalManager，不复制 App/AppState，不新增依赖。

## 全文件映射和保留职责

|来源|之前行数|之后行数|职责/去向|
|---|---:|---:|---|
|`crates/vw-agent/src/channels/runtime_command.rs`|44|40|宿主模块注册与公共命令导出；移除根哨兵旧挂载，保留真实宿主测试挂载及先前外部 path 测试。|
|`crates/vw-agent/src/channels/runtime_command_tests.rs`|8|0|哨兵完整迁至领域自注册模块，删除旧挂载。|
|`crates/vw-agent/src/channels/runtime_command/approval_commands.rs`|2|2|领域实现的兼容导出；已有调用者及外部挂载测试仍引用，不计生产迁移收益。|
|`crates/vw-agent/src/channels/runtime_command/approval_config.rs`|2|2|领域实现的兼容导出；已有调用者及外部挂载测试仍引用，不计生产迁移收益。|
|`crates/vw-agent/src/channels/runtime_command/catalog.rs`|2|2|领域实现的兼容导出；已有调用者及外部挂载测试仍引用，不计生产迁移收益。|
|`crates/vw-agent/src/channels/runtime_command/command.rs`|4|4|领域实现的兼容导出；已有调用者及外部挂载测试仍引用，不计生产迁移收益。|
|`crates/vw-agent/src/channels/runtime_command/command_tests.rs`|149|149|5条真实宿主目录集成；CatalogChannel/name/send/listen 为出站端口，catalog_context 通过真实租户项目配置注册模型，catalog_dispatch 验证宿主 handler 接线；继续可提取纯目录断言，但不以假租户/模型端口替代。|
|`crates/vw-agent/src/channels/runtime_command/handler.rs`|7|7|真实 CommandHost 注入的宿主调用边界。|
|`crates/vw-agent/src/channels/runtime_command/handler_tests.rs`|621|300|17条独立测试迁往 host_migration_40_tests（含根哨兵），保留2条真实宿主集成及其 provider/tool/channel/context 最小原夹具；继续可拆 mixed slash 测试的审批部分，但本轮保留完整函数与断言。|
|`crates/vw-agent/src/channels/runtime_command/history_commands.rs`|7|7|真实 CommandHost 历史操作端口及缓存函数导出。|
|`crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs`|352|352|11条真实会话/消息存储、租户/项目所有权及压缩 runner 集成；seed/fixture/model_server 必须随真实生命周期夹具；继续可提取确认缓存策略断言，现保持整函数与端到端断言。|
|`crates/vw-agent/src/channels/runtime_command/project_commands.rs`|3|3|领域实现的兼容导出；已有调用者及外部挂载测试仍引用，不计生产迁移收益。|
|`crates/vw-agent/src/channels/runtime_command/session_commands.rs`|45|45|模型目录初始化及真实 CommandHost 注入的兼容调用边界。|
|`crates/vw-agent/src/channels/runtime_command/session_lifecycle.rs`|9|9|真实 CommandHost 生命周期端口与路由持久化导出。|
|`crates/vw-agent/src/channels/runtime_command/task_mode.rs`|9|9|真实任务投递宿主端口与开关导出。|

## 全用例映射

- `crates/vw-agent/src/channels/runtime_command_tests.rs::test_module_is_linked` → `channel_runtime::host_migration_40_tests::test_module_is_linked`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_real_provider_model_and_effort_success` → `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_real_provider_model_and_effort_success`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_invalid_inputs_preserve_route_and_history` → `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_invalid_inputs_preserve_route_and_history`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_model_selection_isolated_across_domains` → `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_model_selection_isolated_across_domains`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_status_is_read_only_and_scoped` → `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_status_is_read_only_and_scoped`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_provider_catalog_isolated_by_tenant` → `crates/vw-agent/src/channels/runtime_command/command_tests.rs::channel_command_catalog_provider_catalog_isolated_by_tenant`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::runtime_command_parser_keeps_model_switch_payload` → `channel_runtime::host_migration_40_tests::runtime_command_parser_keeps_model_switch_payload`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::non_command_returns_false_when_task_mode_is_off` → `channel_runtime::host_migration_40_tests::non_command_returns_false_when_task_mode_is_off`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::parsed_command_is_consumed_without_target_channel` → `channel_runtime::host_migration_40_tests::parsed_command_is_consumed_without_target_channel`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::approval_management_command_denies_sender_not_in_approvers` → `channel_runtime::host_migration_40_tests::approval_management_command_denies_sender_not_in_approvers`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::natural_language_approval_denies_unauthorized_sender_before_direct_promotion` → `channel_runtime::host_migration_40_tests::natural_language_approval_denies_unauthorized_sender_before_direct_promotion`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::natural_language_approval_commands_follow_runtime_policy_modes` → `channel_runtime::host_migration_40_tests::natural_language_approval_commands_follow_runtime_policy_modes`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_specific_natural_language_policy_override_is_used` → `channel_runtime::host_migration_40_tests::channel_specific_natural_language_policy_override_is_used`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::direct_mode_keeps_natural_language_unapprove_as_revoke` → `channel_runtime::host_migration_40_tests::direct_mode_keeps_natural_language_unapprove_as_revoke`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::slash_command_dispatch_covers_session_model_and_approval_variants` → `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::slash_command_dispatch_covers_session_model_and_approval_variants`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::task_mode_plain_message_is_consumed_without_reply_channel` → `channel_runtime::host_migration_40_tests::task_mode_plain_message_is_consumed_without_reply_channel`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::send_failure_still_consumes_runtime_command` → `channel_runtime::host_migration_40_tests::send_failure_still_consumes_runtime_command`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_lark_complete_outbound` → `channel_runtime::host_migration_40_tests::channel_command_help_lark_complete_outbound`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_detail_and_unknown` → `channel_runtime::host_migration_40_tests::channel_command_help_detail_and_unknown`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_permission_does_not_expose_approvers` → `channel_runtime::host_migration_40_tests::channel_command_help_permission_does_not_expose_approvers`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_named_instance_preserves_addressing` → `channel_runtime::host_migration_40_tests::channel_command_help_named_instance_preserves_addressing`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_long_output_has_complete_entries` → `channel_runtime::host_migration_40_tests::channel_command_help_long_output_has_complete_entries`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_pipeline_routes_named_instance_without_project_or_model` → `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_pipeline_routes_named_instance_without_project_or_model`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/handler_tests.rs::channel_command_help_text_channels_share_catalog` → `channel_runtime::host_migration_40_tests::channel_command_help_text_channels_share_catalog`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_lists_real_anchors_without_mutation` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_lists_real_anchors_without_mutation`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_fork_switches_and_preserves_source` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_fork_switches_and_preserves_source`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_foreign_anchors_preserve_cache_and_binding` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_foreign_anchors_preserve_cache_and_binding`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_reset_requires_confirmation_and_reloads_cache` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_reset_requires_confirmation_and_reloads_cache`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_stale_confirmation_is_rejected` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_stale_confirmation_is_rejected`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_cross_scope_and_busy_are_rejected` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_cross_scope_and_busy_are_rejected`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_rejects_forged_binding_across_projects_and_tenants` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_rejects_forged_binding_across_projects_and_tenants`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_switching_sessions_invalidates_confirmation` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_switching_sessions_invalidates_confirmation`（原函数断言保留）。
- `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_part_changes_invalidate_confirmation` → `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs::channel_session_history_part_changes_invalidate_confirmation`（原函数断言保留）。

## 外部挂载与数量

runtime_command/session_commands_tests.rs 和 task_mode_tests.rs 在任务1归属清单中的旧跨目录挂载仍依赖 crate::app 等宿主端口；本次未伪装为领域自注册。根解析测试已由 channel-runtime::manager::runtime_command 自注册，未重复迁移。保留文件逐符号可由表中路径的 fn 定义核对；各宿主边界及夹具未删断言。

本任务15个来源：1264 → 931 行，净减 333（26.3%），低于70%。缺口由上表逐文件原因和继续可迁项说明，交收口验收，不宣称达标。领域新增测试 563 行，lib.rs 注册新增3行，生产新增0行。宿主原测试文件按上表计，其他来源全为薄边界/导出；没有新迁出生产逻辑。

## 未来验证（全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_40_tests -- --test-threads=1`：预期17条非零。
- 宿主剩余：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channels::runtime_command::handler_tests -- --test-threads=1` 预期2条；`channels::channel_command_catalog_tests`预期5条；`channels::channel_session_history_tests`预期11条。实际命令按同一入口分别过滤，不能把零测试当成功。
- 迁出原名需在未来宿主 `--list` 核实消失。channel-support 无新增模块，不运行空过滤。
- 未运行 Cargo metadata/list/check/fmt/test、构建、可执行文件或实机；静态差异检查不等于编译/行为通过。

- 补充映射：`channel_session_history_compact_persists_summary_and_next_chat_reads_it` → 原 `channels::channel_session_history_tests::channel_session_history_compact_persists_summary_and_next_chat_reads_it`，保留真实摘要 runner、存储和下一轮读取断言。

- 补充映射：`channel_session_history_blank_compaction_preserves_persisted_history_and_cache` → 原 `channels::channel_session_history_tests::channel_session_history_blank_compaction_preserves_persisted_history_and_cache`，保留真实摘要 runner、存储和下一轮读取断言。

### 原外部挂载 session_commands_tests.rs
- `chat_with_system`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `route`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `context`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `message`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_show_model_includes_current_model`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_model_rejects_blank_model_without_mutating_route`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_model_trims_backticks_persists_route_and_clears_history`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_model_updates_only_target_sender_and_preserves_task_mode`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_provider_reports_unknown_provider_without_mutating_state`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_provider_uses_default_provider_without_initializing_new_provider`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_provider_leaves_history_when_provider_is_already_selected`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_set_provider_reports_sanitized_init_error_without_mutating_state`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_clear_session_clears_session_history_and_disables_task_mode`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `handle_task_mode_clears_session_history_and_enables_task_mode`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `set_catalog_model`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `lifecycle_fixture`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_new_preserves_old`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_lists_are_parsed`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_mutations_are_parsed`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_clear_is_separate`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_new_clears_cache`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_help_explains_clear`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `lifecycle_call`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `lifecycle_id`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `lifecycle_save_history`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_resume_reloads_persisted_state`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_resume_isolates_all_chat_dimensions`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_archive_rejects_stale_and_busy_state`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_continue_and_rename_use_owned_sessions`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `name`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `send`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `listen`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `lifecycle_dispatch`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_pipeline_persists_and_refreshes_model_options`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_resume_isolates_projects_and_tenants`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_failed_creation_and_rename_preserve_binding`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_invalid_saved_model_preserves_current_state`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_lists_are_stable_bounded_and_owned`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `channel_session_lifecycle_new_rejects_invalid_model_options`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。

### 原外部挂载 task_mode_tests.rs
- `chat_with_system`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `failing`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `default`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `name`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `send`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `listen`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `route`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `context`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `message`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `set_sender_task_mode_toggles_only_task_mode_flag`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_message_returns_false_when_mode_is_disabled`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_consumes_message_without_channel_or_content`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_creates_task_and_sends_threaded_response`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_keeps_auto_model_when_route_uses_auto`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_reports_task_creation_error`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
- `task_mode_still_consumes_message_when_reply_send_fails`：原位置/原宿主挂载保留，依赖真实宿主会话、任务或共享上下文；不计本轮迁移。
