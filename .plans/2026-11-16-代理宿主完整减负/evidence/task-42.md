# 任务42证据

代码完成，编译/行为未验证。遵守本轮禁止 Cargo、构建、测试和实机验证的授权约束。仅完成源码与差异静态核对，不报告通过数或编译收益。

来源全体 2632 → 1105 行，净减 1527 行（58.0%）。未达到70%，以下逐文件列明保留职责和继续可迁项，交收口验收。

## 职责与生产方向

生产算法在本轮前已经位于 channel-support / channel-runtime，本次不将既有下沉计为生产迁移收益。真实调用方向为 vw-agent::channels 的适配函数 → channel-runtime::manager::{command, dispatch, execution_binding, execution_control, message, message_execution, message_response, reload, start}，注入 runtime_host::HOST；runtime_host 实现 ChannelHost / CommandHost / ChannelStartupHost，接入真实租户、项目上下文、会话、Shell 回收、工具、配置监听与处理器。领域库没有反向依赖 vw-agent。

配置测试直接调用 channel-support::config；格式测试直接调用 channel-support::format；历史测试直接调用 channel-support::history 共享存储函数，删除无需的 provider/运行时上下文夹具。仅测试使用的 normalize_cached_channel_turns 连同2条原断言移入领域测试文件，未变成新生产API。Nostr构造的3条完整测试直接调用 channel-runtime::manager::channels_config::append_nostr_channel_if_available。

两个库均自注册 host_migration_42_tests，删除 config 旧挂载，history 仅挂载真实租户身份/中断键契约。format 保留进度哨兵及执行结果可见答案组合契约。

## 全文件映射与保留原因

| 来源 | 前→后行数 | 实际归属 / 保留职责及符号 |
|---|---:|---|
| `crates/vw-agent/src/channels/channel_project_scope_tests.rs` | 59→0 | 删除14个重复测试入口；完整原断言仍由宿主 session::session_tests 独立注册，不冒充领域迁移。 |
| `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs` | 178→120 | 删除18个重复测试入口；完整原断言仍由 runtime_command::session_commands_tests 的宿主 path 注册。保留首次响应真实处理器、数据库与本地HTTP模型调用验收：channel_first_response_hides_ordinary_and_explicit_drafts, channel_first_response_reuses_failed_ingress_then_publishes_success, channel_first_response_requests_title_once_after_commit, drop |
| `crates/vw-agent/src/channels/channels_config_host_tests.rs` | 55→10 | 3条Nostr→channel-runtime；QQ实际Webhook解析契约仍保留：channel_project_binding_qq_webhook_uses_default_instance |
| `crates/vw-agent/src/channels/clawdtalk/mod.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/cli/mod.rs` | 3→3 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/config.rs` | 237→234 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/config_tests.rs` | 615→0 | 整组→channel-support::host_migration_42_tests::config |
| `crates/vw-agent/src/channels/dingtalk/mod.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/discord/mod.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/email_channel/mod.rs` | 3→3 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/format.rs` | 83→83 | 进度哨兵与session执行可见答案组合端口：split_internal_progress_delta, sanitize_channel_response |
| `crates/vw-agent/src/channels/format_tests.rs` | 581→85 | 领域断言→channel-support；保留真实组合契约：name, description, parameters_schema, execute, spec, split_internal_progress_delta_detects_known_prefix, split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text, sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript, sanitize_channel_response_rejects_execution_transcript_without_final_answer |
| `crates/vw-agent/src/channels/history.rs` | 184→113 | 当前租户注入和共享上下文到历史存储端口适配；测试专用规范化函数已迁出：conversation_memory_key, conversation_history_key, interruption_scope_key, clear_sender_history, append_sender_turn, rollback_orphan_user_turn |
| `crates/vw-agent/src/channels/history_tests.rs` | 208→28 | 7条→channel-support；保留当前租户注入、中断与历史同键真实契约：channel_message, conversation_keys_include_thread_when_present |
| `crates/vw-agent/src/channels/http.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/imessage/mod.rs` | 3→3 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/irc/mod.rs` | 3→3 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/lark/mod.rs` | 3→3 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/linq/mod.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/channels_config.rs` | 4→4 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：collect_configured_channels |
| `crates/vw-agent/src/channels/manager/command.rs` | 7→7 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：handle_command |
| `crates/vw-agent/src/channels/manager/dispatch.rs` | 8→8 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：run_message_dispatch_loop |
| `crates/vw-agent/src/channels/manager/errors.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/execution_binding.rs` | 23→23 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：resolve_execution_route, execution_options |
| `crates/vw-agent/src/channels/manager/execution_control.rs` | 11→11 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：process_channel_message |
| `crates/vw-agent/src/channels/manager/health.rs` | 7→7 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：doctor_channels |
| `crates/vw-agent/src/channels/manager/listener.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/message.rs` | 24→24 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：process_channel_message_inner, prepare_streaming_ui_state, spawn_draft_updater |
| `crates/vw-agent/src/channels/manager/message_execution.rs` | 24→24 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：run_message_execution, stop_channel_execution, reap_channel_shell_processes |
| `crates/vw-agent/src/channels/manager/message_helpers.rs` | 6→6 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/message_response.rs` | 33→33 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：handle_llm_result, mark_retryable_reply, reconcile_channel_reply |
| `crates/vw-agent/src/channels/manager/message_result.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/mod.rs` | 48→48 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：command_host |
| `crates/vw-agent/src/channels/manager/provider.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/manager/reload.rs` | 23→23 | 真实配置监听器创建与ChannelConfigSource适配：start_channels_with_reload, run_channels_with_source, poll |
| `crates/vw-agent/src/channels/manager/runtime_host.rs` | 152→152 | 真实宿主端口实现，必须保留：now_ms, scrub_credentials, draft_sentinels, save_config, in_tenant_scope, in_scope, session_id, collect_response, reap_shell, load_binding, agent_catalog, prepare_binding, locked_model, persist_turn, take_reply, retry_reply, command_tenant, prepare_catalog, command_now_ms, session_activity, in_project_scope, in_session_scope, get_session, get_scoped_session, reserve_session, discard_session, set_session_title, set_session_archived, fork_session, remove_session, revert_session, cleanup_revert, compact_session, load_ui_session, load_session_preview, create_task, tenant, tools, skills, hooks, recursive_policy, tool_instructions, shell_policy |
| `crates/vw-agent/src/channels/manager/start.rs` | 16→16 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：start_channels, build_channel_hooks |
| `crates/vw-agent/src/channels/manager/telegram.rs` | 7→7 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。符号：bind_telegram_identity |
| `crates/vw-agent/src/channels/manager/typing.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/markdown_chunk.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |
| `crates/vw-agent/src/channels/mattermost/mod.rs` | 2→2 | 兼容导出或HOST注入薄适配；既有实际调用者与跨目录宿主测试依赖当前入口，不改公共路径。无业务函数；仅导出。 |

## 全用例映射

- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_rooms_have_independent_history` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_rooms_have_independent_history`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_instances_have_independent_history` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_instances_have_independent_history`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_threads_have_independent_history` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_threads_have_independent_history`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_senders_have_independent_history` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_senders_have_independent_history`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_identity_delimiters_do_not_collide` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_identity_delimiters_do_not_collide`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_binding_priority_and_immutable_snapshot` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_binding_priority_and_immutable_snapshot`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_invalid_binding_preserves_state` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_invalid_binding_preserves_state`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_project_history_and_routes_are_independent` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_project_history_and_routes_are_independent`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_unavailable_directory_does_not_cache_fake_session` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_unavailable_directory_does_not_cache_fake_session`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_tenant_cannot_resolve_other_tenant_binding` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_tenant_cannot_resolve_other_tenant_binding`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_processor_uses_captured_directory` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_processor_uses_captured_directory`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_memory_reads_only_matching_scope` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_memory_reads_only_matching_scope`
- `crates/vw-agent/src/channels/channel_project_scope_tests.rs::channel_project_scope_snapshot_rejects_different_sender` → `crates/vw-agent/src/channels/session_tests.rs（原断言，去重）::channel_project_scope_snapshot_rejects_different_sender`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_new_preserves_old` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_new_preserves_old`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_lists_are_parsed` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_lists_are_parsed`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_mutations_are_parsed` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_mutations_are_parsed`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_clear_is_separate` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_clear_is_separate`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_new_clears_cache` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_new_clears_cache`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_help_explains_clear` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_help_explains_clear`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_resume_reloads_persisted_state` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_resume_reloads_persisted_state`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_resume_isolates_all_chat_dimensions` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_resume_isolates_all_chat_dimensions`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_archive_rejects_stale_and_busy_state` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_archive_rejects_stale_and_busy_state`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_continue_and_rename_use_owned_sessions` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_continue_and_rename_use_owned_sessions`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_pipeline_persists_and_refreshes_model_options` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_pipeline_persists_and_refreshes_model_options`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_resume_isolates_projects_and_tenants` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_resume_isolates_projects_and_tenants`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_failed_creation_and_rename_preserve_binding` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_failed_creation_and_rename_preserve_binding`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_invalid_saved_model_preserves_current_state` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_invalid_saved_model_preserves_current_state`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_lists_are_stable_bounded_and_owned` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_lists_are_stable_bounded_and_owned`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_session_lifecycle_new_rejects_invalid_model_options` → `crates/libs/channel-runtime/src/runtime_command/session_commands_tests.rs（原断言，仍为宿主path测试，未宣称领域自注册）::channel_session_lifecycle_new_rejects_invalid_model_options`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_first_response_hides_ordinary_and_explicit_drafts` → `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs（宿主保留）::channel_first_response_hides_ordinary_and_explicit_drafts`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_first_response_reuses_failed_ingress_then_publishes_success` → `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs（宿主保留）::channel_first_response_reuses_failed_ingress_then_publishes_success`
- `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs::channel_first_response_requests_title_once_after_commit` → `crates/vw-agent/src/channels/channel_session_lifecycle_tests.rs（宿主保留）::channel_first_response_requests_title_once_after_commit`
- `crates/vw-agent/src/channels/channels_config_host_tests.rs::append_nostr_channel_if_available_is_noop_when_not_configured` → `crates/libs/channel-runtime/src/host_migration_42_tests.rs::append_nostr_channel_if_available_is_noop_when_not_configured`
- `crates/vw-agent/src/channels/channels_config_host_tests.rs::append_nostr_channel_if_available_adds_valid_configured_channel` → `crates/libs/channel-runtime/src/host_migration_42_tests.rs::append_nostr_channel_if_available_adds_valid_configured_channel`
- `crates/vw-agent/src/channels/channels_config_host_tests.rs::append_nostr_channel_if_available_reports_invalid_private_key` → `crates/libs/channel-runtime/src/host_migration_42_tests.rs::append_nostr_channel_if_available_reports_invalid_private_key`
- `crates/vw-agent/src/channels/channels_config_host_tests.rs::channel_project_binding_qq_webhook_uses_default_instance` → `crates/vw-agent/src/channels/channels_config_host_tests.rs（宿主保留）::channel_project_binding_qq_webhook_uses_default_instance`
- `crates/vw-agent/src/channels/config_tests.rs::timeout_helpers_apply_floor_scale_and_saturating_math` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::timeout_helpers_apply_floor_scale_and_saturating_math`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_preserve_configured_values` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_preserve_configured_values`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_prefer_complete_main_agent_model_binding` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_prefer_complete_main_agent_model_binding`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_ignore_incomplete_main_agent_model_binding` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_ignore_incomplete_main_agent_model_binding`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_use_builtin_provider_and_model_when_unset` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_use_builtin_provider_and_model_when_unset`
- `crates/vw-agent/src/channels/config_tests.rs::resolved_defaults_use_hardcoded_fallbacks_when_fields_are_none` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::resolved_defaults_use_hardcoded_fallbacks_when_fields_are_none`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_autonomy_policy_preserves_all_runtime_fields` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_autonomy_policy_preserves_all_runtime_fields`
- `crates/vw-agent/src/channels/config_tests.rs::timeout_budget_saturates_on_large_values` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::timeout_budget_saturates_on_large_values`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_config_path_uses_vibewindow_dir_when_present` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_config_path_uses_vibewindow_dir_when_present`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_snapshot_uses_context_without_cached_config` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_snapshot_uses_context_without_cached_config`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_snapshot_uses_context_when_config_path_is_not_cached` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_snapshot_uses_context_when_config_path_is_not_cached`
- `crates/vw-agent/src/channels/config_tests.rs::runtime_defaults_snapshot_prefers_cached_config_state` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::runtime_defaults_snapshot_prefers_cached_config_state`
- `crates/vw-agent/src/channels/config_tests.rs::snapshot_non_cli_excluded_tools_clones_current_list` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::snapshot_non_cli_excluded_tools_clones_current_list`
- `crates/vw-agent/src/channels/config_tests.rs::decrypt_optional_secret_for_runtime_reload_handles_all_value_shapes` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::decrypt_optional_secret_for_runtime_reload_handles_all_value_shapes`
- `crates/vw-agent/src/channels/config_tests.rs::config_file_stamp_reports_existing_files_only` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::config_file_stamp_reports_existing_files_only`
- `crates/vw-agent/src/channels/config_tests.rs::load_runtime_defaults_reports_read_and_parse_errors` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::load_runtime_defaults_reports_read_and_parse_errors`
- `crates/vw-agent/src/channels/config_tests.rs::load_runtime_defaults_reads_toml_config_and_policy` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::load_runtime_defaults_reads_toml_config_and_policy`
- `crates/vw-agent/src/channels/config_tests.rs::maybe_apply_runtime_config_update_skips_missing_paths_and_unchanged_stamp` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::maybe_apply_runtime_config_update_skips_missing_paths_and_unchanged_stamp`
- `crates/vw-agent/src/channels/config_tests.rs::maybe_apply_runtime_config_update_returns_load_errors_for_changed_files` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::maybe_apply_runtime_config_update_returns_load_errors_for_changed_files`
- `crates/vw-agent/src/channels/config_tests.rs::maybe_apply_runtime_config_update_applies_changed_config_to_runtime_state` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::maybe_apply_runtime_config_update_applies_changed_config_to_runtime_state`
- `crates/vw-agent/src/channels/config_tests.rs::maybe_apply_runtime_config_update_does_not_mutate_state_when_provider_creation_fails` → `crates/libs/channel-support/src/host_migration_42_tests/config.rs::maybe_apply_runtime_config_update_does_not_mutate_state_when_provider_creation_fails`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_removes_closed_tool_blocks` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_removes_closed_tool_blocks`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_handles_aliases_unclosed_json_and_sentinel` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_handles_aliases_unclosed_json_and_sentinel`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_preserves_unparseable_unclosed_blocks` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_preserves_unparseable_unclosed_blocks`
- `crates/vw-agent/src/channels/format_tests.rs::split_internal_progress_delta_detects_known_prefix` → `crates/vw-agent/src/channels/format_tests.rs（宿主保留）::split_internal_progress_delta_detects_known_prefix`
- `crates/vw-agent/src/channels/format_tests.rs::split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text` → `crates/vw-agent/src/channels/format_tests.rs（宿主保留）::split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text`
- `crates/vw-agent/src/channels/format_tests.rs::channel_delivery_instructions_are_known_for_chat_channels` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::channel_delivery_instructions_are_known_for_chat_channels`
- `crates/vw-agent/src/channels/format_tests.rs::channel_delivery_instructions_include_channel_specific_requirements` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::channel_delivery_instructions_include_channel_specific_requirements`
- `crates/vw-agent/src/channels/format_tests.rs::should_expose_internal_tool_details_respects_positive_and_negative_intents` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::should_expose_internal_tool_details_respects_positive_and_negative_intents`
- `crates/vw-agent/src/channels/format_tests.rs::extract_tool_context_summary_collects_unique_tool_names_after_start_index` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::extract_tool_context_summary_collects_unique_tool_names_after_start_index`
- `crates/vw-agent/src/channels/format_tests.rs::tool_json_payload_detection_requires_known_tool_and_arguments` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::tool_json_payload_detection_requires_known_tool_and_arguments`
- `crates/vw-agent/src/channels/format_tests.rs::tool_result_payload_requires_prior_tool_call_and_result_shape` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::tool_result_payload_requires_prior_tool_call_and_result_shape`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_tool_json_value_strips_calls_and_keeps_message_content` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_tool_json_value_strips_calls_and_keeps_message_content`
- `crates/vw-agent/src/channels/format_tests.rs::isolated_tool_json_artifacts_are_removed_without_touching_inline_json` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::isolated_tool_json_artifacts_are_removed_without_touching_inline_json`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_channel_response_removes_known_tool_json_and_tags` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_channel_response_removes_known_tool_json_and_tags`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript` → `crates/vw-agent/src/channels/format_tests.rs（宿主保留）::sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_channel_response_rejects_execution_transcript_without_final_answer` → `crates/vw-agent/src/channels/format_tests.rs（宿主保留）::sanitize_channel_response_rejects_execution_transcript_without_final_answer`
- `crates/vw-agent/src/channels/format_tests.rs::line_isolated_json_segment_detects_whitespace_boundaries` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::line_isolated_json_segment_detects_whitespace_boundaries`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_removes_all_closed_tag_variants` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_removes_all_closed_tag_variants`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_handles_block_boundaries_and_extra_blank_lines` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_handles_block_boundaries_and_extra_blank_lines`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_keeps_empty_line_after_removed_block` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_keeps_empty_line_after_removed_block`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_handles_unclosed_json_arrays_and_leading_close_tags` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_handles_unclosed_json_arrays_and_leading_close_tags`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_uses_earliest_tool_marker` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_uses_earliest_tool_marker`
- `crates/vw-agent/src/channels/format_tests.rs::strip_tool_call_tags_preserves_messages_without_tags` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_tool_call_tags_preserves_messages_without_tags`
- `crates/vw-agent/src/channels/format_tests.rs::should_expose_internal_tool_details_covers_verb_and_no_match_paths` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::should_expose_internal_tool_details_covers_verb_and_no_match_paths`
- `crates/vw-agent/src/channels/format_tests.rs::should_expose_internal_tool_details_handles_english_verb_and_negation_hints` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::should_expose_internal_tool_details_handles_english_verb_and_negation_hints`
- `crates/vw-agent/src/channels/format_tests.rs::extract_tool_context_summary_handles_tag_variants_and_malformed_results` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::extract_tool_context_summary_handles_tag_variants_and_malformed_results`
- `crates/vw-agent/src/channels/format_tests.rs::extract_tool_context_summary_ignores_native_json_without_calls` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::extract_tool_context_summary_ignores_native_json_without_calls`
- `crates/vw-agent/src/channels/format_tests.rs::extract_tool_context_summary_deduplicates_native_and_result_names` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::extract_tool_context_summary_deduplicates_native_and_result_names`
- `crates/vw-agent/src/channels/format_tests.rs::is_tool_call_payload_accepts_fallback_names_and_rejects_bad_shapes` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::is_tool_call_payload_accepts_fallback_names_and_rejects_bad_shapes`
- `crates/vw-agent/src/channels/format_tests.rs::is_tool_result_payload_rejects_objects_without_result` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::is_tool_result_payload_rejects_objects_without_result`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_tool_json_value_handles_arrays_and_non_objects` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_tool_json_value_handles_arrays_and_non_objects`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_tool_json_value_handles_tool_call_messages_without_content` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_tool_json_value_handles_tool_call_messages_without_content`
- `crates/vw-agent/src/channels/format_tests.rs::strip_isolated_tool_json_artifacts_replaces_message_objects_and_normalizes_crlf` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_isolated_tool_json_artifacts_replaces_message_objects_and_normalizes_crlf`
- `crates/vw-agent/src/channels/format_tests.rs::strip_isolated_tool_json_artifacts_preserves_invalid_and_unrelated_json` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_isolated_tool_json_artifacts_preserves_invalid_and_unrelated_json`
- `crates/vw-agent/src/channels/format_tests.rs::strip_isolated_tool_json_artifacts_removes_tool_arrays_and_following_results` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_isolated_tool_json_artifacts_removes_tool_arrays_and_following_results`
- `crates/vw-agent/src/channels/format_tests.rs::strip_isolated_tool_json_artifacts_keeps_early_results_until_a_tool_call_is_seen` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_isolated_tool_json_artifacts_keeps_early_results_until_a_tool_call_is_seen`
- `crates/vw-agent/src/channels/format_tests.rs::strip_isolated_tool_json_artifacts_preserves_plain_text_without_json` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::strip_isolated_tool_json_artifacts_preserves_plain_text_without_json`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_channel_response_preserves_clean_response_without_tools` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_channel_response_preserves_clean_response_without_tools`
- `crates/vw-agent/src/channels/format_tests.rs::sanitize_channel_response_matches_tool_names_case_insensitively` → `crates/libs/channel-support/src/host_migration_42_tests/format.rs::sanitize_channel_response_matches_tool_names_case_insensitively`
- `crates/vw-agent/src/channels/history_tests.rs::conversation_keys_include_thread_when_present` → `crates/vw-agent/src/channels/history_tests.rs（宿主保留）::conversation_keys_include_thread_when_present`
- `crates/vw-agent/src/channels/history_tests.rs::normalize_cached_turns_merges_unexpected_roles_and_skips_system` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::normalize_cached_turns_merges_unexpected_roles_and_skips_system`
- `crates/vw-agent/src/channels/history_tests.rs::normalize_cached_turns_drops_orphan_leading_roles_and_merges_without_separator_for_empty_last` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::normalize_cached_turns_drops_orphan_leading_roles_and_merges_without_separator_for_empty_last`
- `crates/vw-agent/src/channels/history_tests.rs::append_sender_turn_preserves_complete_history` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::append_sender_turn_preserves_complete_history`
- `crates/vw-agent/src/channels/history_tests.rs::clear_sender_history_removes_existing_key_and_ignores_missing_key` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::clear_sender_history_removes_existing_key_and_ignores_missing_key`
- `crates/vw-agent/src/channels/history_tests.rs::history_operations_recover_from_poisoned_history_lock` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::history_operations_recover_from_poisoned_history_lock`
- `crates/vw-agent/src/channels/history_tests.rs::rollback_orphan_user_turn_only_removes_matching_last_user_turn` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::rollback_orphan_user_turn_only_removes_matching_last_user_turn`
- `crates/vw-agent/src/channels/history_tests.rs::rollback_orphan_user_turn_removes_empty_history_entry` → `crates/libs/channel-support/src/host_migration_42_tests/history.rs::rollback_orphan_user_turn_removes_empty_history_entry`

## 后续可迁项与挂载边界

不足70%的主要原因：保留宿主首次响应测试120行、runtime_host 152行、config兼容入口234行及history当前租户/存储适配115行，其余见上表。config的大部分行是既有API文档，不能通过删文档凑减量。manager的跨目录测试仍引用 crate::app::agent 和真实宿主夹具；其源文件不是本任务唯一所有，保持现有挂载，不复制完整App或绕过真实权限。继续可迁：由对应测试文件所有者拆分 session_tests、session_commands_tests 和 manager 跨目录测试的领域断言，随后才能删除这些入口的测试兼容依赖。纯重导出薄文件仍供其他通道文件族和外部调用者使用。

## 未来验证（本次全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p channel-support --lib host_migration_42_tests -- --test-threads=1`：静态预期 63 条，须验证非零且failed=0。
- `python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_42_tests -- --test-threads=1`：静态预期 3 条，须验证非零且failed=0。
- 宿主保留用例按上列完整名称逐条过滤；32个重复包装入口应从宿主真实list中消失，原断言模块仍应存在。
- 不把list零测试当作通过；本轮未运行任何编译、行为、格式化或实机检查。

## 领域新增物理行数

- `crates/libs/channel-support/src/host_migration_42_tests/format.rs`：531行，全部为测试/夹具；新增生产逻辑0行。
- `crates/libs/channel-support/src/host_migration_42_tests/config.rs`：616行，全部为测试/夹具；新增生产逻辑0行。
- `crates/libs/channel-support/src/host_migration_42_tests/history.rs`：193行，全部为测试/夹具；新增生产逻辑0行。
- `crates/libs/channel-runtime/src/host_migration_42_tests.rs`：48行，全部为测试/夹具；新增生产逻辑0行。

## 深度迁移补充（覆盖上一轮保留结论）

读取 final.md 与 remaining-host.json 后继续执行。未运行任何 Cargo、构建、测试或实机命令。配置API与历史共享存储API的文档随真实实现归位，保持文档内容；该部分仅计兼容层清理，不计生产算法迁移。

原 host format.rs 的两个生产函数完整迁入 channel-runtime::format。运行库 manager 不再私有复制 sanitize_channel_response，直接使用同一入口；宿主 channels::format 直接别名此领域模块。路径调用者（宿主 runtime_command/session_commands_tests、manager 等）保持已有函数名。领域入口依赖已有 agent-loop::progress 和 session-engine::child_agent::history，后者与旧宿主使用的 run::visible_answer_from_execution_output 是同一重导出函数。没有新增依赖，也没有引入Host抽象或伪造租户。

原四条格式断言完整迁入 channel-runtime/src/host_migration_42_format_tests.rs，由 host_migration_42_tests::format 自注册。测试函数名及断言不变，去掉已经无调用的 NamedTool 夹具。宿主旧 format 测试文件和挂载删除。

配置旧 config.rs 完整消除，父模块直接别名 channel-support::config。原API文档迁到对应领域函数声明，非通过删文档凑减量。history 仍保留当前租户注入及 ChannelRuntimeContext 存储字段映射，长篇领域操作文档移至 channel-support::history 的对应真实函数。真实租户身份测试28行仍保留；首次响应120行仍验证真实宿主处理器与数据库、标题模型HTTP调用；runtime_host152行仍为实际宿主权限、项目上下文与会话资源端口，不能复制进领域库冒充迁移。

| 本轮来源 | 新归属 | 源前→后行数 |
|---|---|---:|
| `crates/vw-agent/src/channels/config.rs` | channel-support::config | 234→0 |
| `crates/vw-agent/src/channels/format.rs` | channel-runtime::format / host_migration_42_tests::format | 83→0 |
| `crates/vw-agent/src/channels/format_tests.rs` | channel-runtime::format / host_migration_42_tests::format | 85→0 |
| `crates/vw-agent/src/channels/history.rs` | channel-support::history + 宿主租户/存储薄端口 | 113→53 |

remaining-host.json 的42号全体来源 1105 → 643 行，追加净减 462 行。原基线2632行口径净减 1989 行（75.6%）。领域生产/测试与文档分别如上，不宣称编译收益。
- 原 `channels::format::format_tests::split_internal_progress_delta_detects_known_prefix` → `channel_runtime::host_migration_42_tests::format::split_internal_progress_delta_detects_known_prefix`。
- 原 `channels::format::format_tests::split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text` → `channel_runtime::host_migration_42_tests::format::split_internal_progress_delta_hides_ws_event_payloads_and_preserves_normal_text`。
- 原 `channels::format::format_tests::sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript` → `channel_runtime::host_migration_42_tests::format::sanitize_channel_response_keeps_only_visible_answer_from_execution_transcript`。
- 原 `channels::format::format_tests::sanitize_channel_response_rejects_execution_transcript_without_final_answer` → `channel_runtime::host_migration_42_tests::format::sanitize_channel_response_rejects_execution_transcript_without_final_answer`。

未来：`python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_42_tests -- --test-threads=1`，静态预期7条（原Nostr3条+格式4条）；channel-support原过滤与预期不变。真实编译归属和行为全部未验证。

静态注册审查：audit_registrations.py 返回 edges=1139、unresolved=0、removed_targets_referenced=0；本轮定向 git diff --check 无空白错误。此为词法静态证据，非运行测试。

静态注册审查：audit_registrations.py 返回 edges=1139、unresolved=0、removed_targets_referenced=0；本轮定向 git diff --check 无空白错误。此为词法静态证据，非运行测试。

## 专项复查：通道派发、取消与宿主装配

本轮仍禁止 Cargo、构建和运行测试，未宣称通过编译或行为验收。源码检索宿主与挂载的 manager 测试后，resolve_execution_route、process_channel_message_inner、spawn_draft_updater 三个宿主转发仅有定义，无调用者。删除这些失用包装；真正领域实现与领域调用均仍在 channel-runtime::manager，不将删除包装计为新增领域算法。

ChannelStartupHost 既有 recursive_policy、tool_instructions、shell_policy 方法改为直接调用 agent-loop 的领域默认实现，RuntimeHost 删除三项同义覆写；无新trait方法或开关。实际调用仍由运行库 startup 持有，不再依赖宿主提供通用提示词算法。保留可覆写能力，不改变其它端口实现。原 skills 方法经调用链核对会注入当前租户插件清单，明确保留；tools、hooks、tenant、session、Shell取消与处理器调用也仍是实际宿主边界。

channel_project_binding_qq_webhook_uses_default_instance 完整函数及两条断言原样迁到 channel-qq::host_migration_42_tests，自注册；只把兼容 QQChannel 路径改为本库真实类型。删除 channels_config_host_tests.rs 和宿主旧挂载，不复制宿主夹具、不删断言。

| 原文件 | 本轮前→后行数 | 保留 / 去向 |
|---|---:|---|
| `crates/vw-agent/src/channels/manager/execution_binding.rs` | 23→19 | 三个失用包装删除；实际端口/剩余测试挂载保留；QQ断言归领域库 |
| `crates/vw-agent/src/channels/manager/message.rs` | 24→12 | 三个失用包装删除；实际端口/剩余测试挂载保留；QQ断言归领域库 |
| `crates/vw-agent/src/channels/manager/runtime_host.rs` | 152→143 | 三个失用包装删除；实际端口/剩余测试挂载保留；QQ断言归领域库 |
| `crates/vw-agent/src/channels/channels_config_host_tests.rs` | 10→0 | 三个失用包装删除；实际端口/剩余测试挂载保留；QQ断言归领域库 |
| `crates/vw-agent/src/channels/mod.rs` | 334→331 | 三个失用包装删除；实际端口/剩余测试挂载保留；QQ断言归领域库 |

宿主本轮净减38行。其余原归属文件已逐项复查，未为达到整轮门槛伪迁真实边界。跨任务约4000行缺口不属于本任务完成声明。

未来精确验证：
- `python3 scripts/cargo_slot.py cargo test -p channel-qq --lib host_migration_42_tests -- --test-threads=1`，预期1条。
- channel-runtime 既有 host_migration_42_tests 预期7条不变；启动端口默认方法由后续该包相关启动契约验证。
- 宿主首次响应三条、租户身份一条及 manager 的真实会话/取消/配置监听挂载继续保留，未通过mock替代。

静态检查：audit_registrations.py 返回 edges=1105、unresolved=0、removed_targets_referenced=0；定向 git diff --check 无空白错误。上述为词法注册与差异核对，不是编译或运行测试。
