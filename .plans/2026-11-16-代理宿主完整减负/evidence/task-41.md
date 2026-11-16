# 任务41：通道测试迁移证据

代码完成，编译/行为未验证。按用户要求，未运行任何 Cargo、构建、单元或实机测试。

## 全文件、全用例映射

| 原文件 | 原行数 | 目标 / 保留职责 | 完整测试名 |
|---|---:|---|---|
| approval_commands.rs | 35 | channel-runtime/src/host_migration_41_tests/approval_commands.rs | approval_command_parse_tests |
| command_parsing.rs | 184 | channel-runtime/src/host_migration_41_tests/command_parsing.rs | parse_runtime_command_allows_approval_commands_on_non_model_channels, parse_runtime_command_supports_natural_language_approval_intents |
| config_loading.rs | 32 | channel-runtime/src/host_migration_41_tests/config_loading.rs | config_loading_tests_placeholder |
| health_supervisor.rs | 128 | channel-runtime/src/host_migration_41_tests/health_supervisor.rs | classify_health_ok_true, classify_health_ok_false, classify_health_timeout |
| history_management.rs | 272 | channel-runtime/src/host_migration_41_tests/history_management.rs | context_window_overflow_error_detector_matches_known_messages, memory_context_skip_rules_exclude_history_blobs, normalize_cached_channel_turns_merges_consecutive_user_turns, normalize_cached_channel_turns_merges_consecutive_assistant_turns, normalize_preserves_failure_marker_after_orphan_user_turn, normalize_preserves_timeout_marker_after_orphan_user_turn, normalize_merges_consecutive_user_turns_simple, normalize_preserves_strict_alternation, normalize_merges_multiple_consecutive_user_turns, normalize_empty_input |
| identity.rs | 19 | channel-runtime/src/host_migration_41_tests/identity.rs | identity_tests_placeholder |
| message_dispatch.rs | 254 | 宿主原路径保留：真实平台解析、租户与项目隔离、真实 session/processor 流程，不能用领域替身替代 | channel_command_flow_feishu_named_session_roundtrip, channel_command_flow_qq_session_roundtrip, channel_command_flow_errors_preserve_project_session_and_history, channel_command_flow_concurrent_instances_rooms_threads_and_tenant_reject, channel_command_flow_qq_webhook_session_roundtrip, channel_command_flow_feishu_parser_preserves_actor_and_topic |
| mod.rs | 86 | channel-runtime/src/host_migration_41_tests.rs：workspace 夹具与自注册 |  |
| routing.rs | 13 | channel-runtime/src/host_migration_41_tests/routing.rs | routing_tests_placeholder |
| session_history.rs | 23 | channel-runtime/src/host_migration_41_tests/session_history.rs | to_session_history_preserves_tool_role |
| system_prompt.rs | 271 | channel-runtime/src/host_migration_41_tests/system_prompt.rs | prompt_contains_all_sections, channel_prompt_uses_configured_timezone, prompt_injects_tools, prompt_injects_safety, prompt_injects_workspace_files, prompt_missing_file_markers, prompt_ignores_legacy_bootstrap_file, prompt_no_daily_memory_injection, prompt_runtime_metadata, prompt_truncation, prompt_empty_files_skipped, prompt_contains_channel_capabilities, prompt_workspace_path |
| tool_execution.rs | 111 | channel-runtime/src/host_migration_41_tests/tool_execution.rs | build_runtime_tool_visibility_prompt_respects_excluded_snapshot |
| vision.rs | 13 | channel-runtime/src/host_migration_41_tests/vision.rs | vision_tests_placeholder |

## 调用与依赖边界

- command_parsing 直接使用 manager::runtime_command::parse_runtime_command；生产 manager 命令处理仍使用同一个入口。
- health_supervisor 使用 manager::health::classify_health_result；生产 doctor/health 亦使用同一分类器。
- history_management 使用 manager::errors::is_context_window_overflow_error 与 channel-support::prompt::should_skip_memory_context_entry。normalize_cached_channel_turns 原本仅 cfg(test)，作为原样测试夹具随测试迁入；宿主 history.rs 的另一组测试仍使用自己的原夹具，该文件属于其它任务，不修改。
- system_prompt 的薄测试适配器直调 channel-support::prompt::build_system_prompt_with_mode，保留 Full、非原生与时区参数；没有 AgentTool 的本组断言不涉及递归策略，端口传空字符串。
- tool_execution 使用 channel-support::prompt::build_runtime_tool_visibility_prompt 与真实 agent_loop::instructions::build_tool_instructions_from_specs；无模拟协议算法。
- session_history 直接使用 channel-support::session_events::to_session_history 与 session-port ChatRole。
- 5 个原 assert!(true) 占位用例原样保留，不记作行为覆盖。
- 接收库通过自身 lib.rs 的 cfg(test, 非 wasm32) 模块注册，子文件正常 mod；宿主 tests/mod.rs 与旧 mod tests 挂载删除；message_dispatch 的根 include 保留。未添加反向 vw-agent 依赖或 path/include 宿主源码。
- 仅新增已有锁定版本 chrono/chrono-tz 的测试依赖；其余依赖已存在。task-1 external-mounts.json 无本组额外跨目录挂载。

## 行数与保留说明

- 范围内宿主：生产 0→0 行，测试/夹具/文档 1441→254 行，净减 1187 行（82.4%）；领域新增生产 0 行，测试/夹具/文档 1183 行。生产下沉收益为 0。
- 唯一保留来源 message_dispatch.rs：254 行，6 条用例逐项列于上表；涉及真实权限拒绝、错误保持、命名实例与 webhook/websocket 接线。共享 command_flow_* 与 FlowSummaryServer 夹具都服务这些宿主用例。
- 原 history.rs 测试夹具不是本任务来源清单，不计入上述减量。

## 静态核对与未来验证

- 迁入 35 条测试，名称与断言保持；宿主保留 6 条（其中飞书解析用例受 feature 控制）。静态路径和直接依赖核对完成，运行注册数未知。
- 未来：`python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_41_tests -- --test-threads=1`，预期 35 条，必须非零且 failed=0。
- 未来宿主：以表中 6 个完整函数名分别过滤 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <name> -- --test-threads=1`；飞书用例加 `--features channel-lark`。每个预期 1 条。迁出项用真实宿主 --list 核对消失，不能以零测试声称通过。
- 无行为修改、无红绿证据、无编译收益结论；后续统一验证负责实际编译与运行。
