# 任务23静态迁移证据
代码完成，编译/行为未验证。本轮按计划未执行 Cargo、测试、构建或实机验证。

## 生产边界
现有实现已在 session-engine；本轮真实生产算法迁移为 0。未把既有下沉计为收益。依赖保持 vw-agent → session-engine，无反向依赖，无 path/include 宿主源码。领域测试由本库 lib.rs 及 compaction.rs 自注册，直接调用原领域函数；存储用例沿用 host_split_8 的子进程临时根目录隔离。无新增依赖。
agent_binding 保留 store（宿主数据库）、scope（当前 home/tenant）、load_scoped/save_scoped、SessionSettings 的 load/save/remove/Deref、confirmed_full_access/resolve_full_access_scoped（注入作用域）、resolve_full_access/load（当前项目）、apply_to_config/prepare 及 AdmissionHost（真实配置/技能/MCP/权限端口）。capability_policy 保留重导出与 AdmissionHost 入口；compaction 保留 CompactionHost、process/process_session 及真实快照/消息投影/模型/摘要端口；compaction_checkpoint 保留 scope_for_directory/scope 与 save/restore/copy/copy_fork/remove_strict/remove 的真实租户目录注入；compaction_summary 保留 SummaryHost、summarize、summarize_async 的真实 llm runner、取消、spawn_blocking 和租户/项目传播；context_injection 保留兼容重导出。所有调用方向沿用现有生产入口。

## 全文件及全用例映射

### agent_binding_tests.rs：测试 174 → 125 行
- `agent_session_identity_storage_round_trip_is_scoped_and_secret_free` → 宿主保留；真实当前 home/租户存储或 AdmissionHost 分叉准入
- `agent_session_identity_rejects_engine_and_agent_changes` → crates/libs/session-engine/src/host_migration_23_tests/binding.rs；原函数名/断言保留，入口改为领域依赖
- `agent_session_identity_keeps_live_credentials_and_security_rules` → crates/libs/session-engine/src/host_migration_23_tests/binding.rs；原函数名/断言保留，入口改为领域依赖
- `agent_config_cleanup_full_access_session_state_preserves_explicit_false` → 宿主保留；真实当前 home/租户存储或 AdmissionHost 分叉准入
- `agent_config_cleanup_full_access_retains_permission_denials` → crates/libs/session-engine/src/host_migration_23_tests/binding.rs；原函数名/断言保留，入口改为领域依赖
- `agent_binding_inherited_history_cannot_override_fork_selection` → 宿主保留；真实当前 home/租户存储或 AdmissionHost 分叉准入

### capability_policy_tests.rs：测试 284 → 240 行
- `agent_native_defaults_mcp_selection_reaches_registered_tools_with_session_isolation` → 宿主保留；真实技能发现、MCP 注册/执行、项目覆盖与跨租户拒绝
- `agent_native_defaults_do_not_inject_disabled_skills` → 宿主保留；真实技能发现、MCP 注册/执行、项目覆盖与跨租户拒绝
- `agent_native_defaults_mcp_respects_tenant_and_project_safety_scope` → 宿主保留；真实技能发现、MCP 注册/执行、项目覆盖与跨租户拒绝
- `agent_config_cleanup_strict_skill_boundary_survives_fixed_selection` → crates/libs/session-engine/src/host_migration_23_tests/policy.rs；原函数名/断言保留，入口改为领域依赖
- `agent_config_cleanup_native_skill_inheritance_uses_allowed_source` → 宿主保留；真实技能发现、MCP 注册/执行、项目覆盖与跨租户拒绝

### claude_protocol_parity_tests.rs：测试 109 → 109 行
- `delegate_capability_policy_is_consistent_across_agenttool_gateway_and_legacy_adapter` → 宿主保留；真实旧 delegated_tools 适配器与现代策略一致性

### codex_alignment_tests.rs：测试 361 → 361 行
- `codex_alignment_skill_search_mcp_compaction_and_new_turn_preserve_boundaries` → 宿主保留；真实 processor、工具/MCP、技能重放与新轮次接线
- `codex_alignment_explicit_tools_search_disabled_and_disable_tools_keep_authorization` → 宿主保留；真实 processor、工具/MCP、技能重放与新轮次接线
- `codex_alignment_strict_skills_and_last_round_do_not_expand_capabilities` → 宿主保留；真实 processor、工具/MCP、技能重放与新轮次接线
- `codex_alignment_same_name_replacement_and_disabled_sources_cannot_replay_old_body` → 宿主保留；真实 processor、工具/MCP、技能重放与新轮次接线
- `codex_alignment_new_session_does_not_inherit_skill_or_tool_exposure` → 宿主保留；真实 processor、工具/MCP、技能重放与新轮次接线

### compaction_tests.rs：测试 831 → 524 行
- `overflow_reserves_summary_output_space` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `instance_directory_opt_returns_directory_inside_project_context` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `compaction_text_formats_roles_trims_and_skips_empty_text` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `compaction_text_keeps_all_messages_and_multibyte_tail` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `compaction_restore_reloads_agents_before_replaying_current_invoked_skill_body` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `process_returns_stop_when_parent_user_message_is_missing` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `internal_compaction_replaces_covered_history_with_a_model_summary` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `internal_compaction_keeps_history_when_summary_is_blank_or_fails` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `process_does_not_compact_when_the_boundary_has_no_prior_history` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `process_does_not_synthesize_a_summary_for_blank_history` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `create_writes_user_message_with_compaction_part` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `create_preserves_manual_compaction_flag` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `prune_marks_only_old_unprotected_outputs_after_thresholds` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `prune_skips_unfinished_tool_states` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖
- `internal_compaction_single_summary_preserves_all_persisted_history` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `manual_compaction_checkpoint_matches_expanded_tool_history` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff` → 宿主保留；真实 CompactionHost 快照/模型端口或完整会话 fork/remove 生命周期
- `native_tool_search_history_compaction_keeps_call_output_pair` → crates/libs/session-engine/src/host_migration_23_tests/compaction.rs；原函数名/断言保留，入口改为领域依赖

### compaction_checkpoint_tests.rs：测试 150 → 75 行
- `checkpoint_restores_model_context_without_mutating_source` → crates/libs/session-engine/src/host_migration_23_tests/checkpoint.rs；原函数名/断言保留，入口改为领域依赖
- `checkpoint_failed_save_keeps_previous_checkpoint` → crates/libs/session-engine/src/host_migration_23_tests/checkpoint.rs；原函数名/断言保留，入口改为领域依赖
- `checkpoint_fork_uses_matching_prefix_and_removal_keeps_source` → crates/libs/session-engine/src/host_migration_23_tests/checkpoint.rs；原函数名/断言保留，入口改为领域依赖
- `checkpoint_rejects_changed_tool_arguments` → crates/libs/session-engine/src/host_migration_23_tests/checkpoint.rs；原函数名/断言保留，入口改为领域依赖
- `checkpoint_isolated_by_tenant` → 宿主保留；真实租户 task-local 注入或项目跨目录与技能检查点联动
- `fork_history_copies_only_matching_checkpoints_between_directories` → 宿主保留；真实租户 task-local 注入或项目跨目录与技能检查点联动

### compaction_summary_tests.rs：测试 402 → 374 行
- `compaction_single_request_preserves_structured_history` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_overflow_trims_request_copy_and_keeps_tool_pairs` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_reuses_cancellation_and_retry` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_dropped_async_request_cancels_the_model_worker` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_preflight_bounds_large_tool_output_without_mutating_source` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_range_overflow_recovers_single_large_message` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_overflow_retries_are_bounded_and_preserve_source` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_impossible_budget_preserves_system_and_skips_model` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_failed_attempt_leaves_previous_checkpoint_restorable` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_budget_reserves_output_and_accepts_unset_input_limit` → crates/libs/session-engine/src/host_migration_23_tests/summary.rs；原函数名/断言保留，入口改为领域依赖
- `compaction_equal_context_and_output_uses_bounded_request_allocation` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_unknown_window_recovers_from_provider_overflow_without_changing_history` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_empty_output_is_normalized_without_replaying_partial_streams` → 宿主保留；真实 LLM 流、重试、取消、请求记录或异步工作线程
- `compaction_empty_error_translation_preserves_metadata_and_other_failures` → crates/libs/session-engine/src/host_migration_23_tests/summary.rs；原函数名/断言保留，入口改为领域依赖
- `agent_binding.rs`：宿主生产/端口（含文档、测试注册）120 → 120 行；职责见生产边界。
- `capability_policy.rs`：宿主生产/端口（含文档、测试注册）10 → 10 行；职责见生产边界。
- `compaction.rs`：宿主生产/端口（含文档、测试注册）67 → 67 行；职责见生产边界。
- `compaction_checkpoint.rs`：宿主生产/端口（含文档、测试注册）32 → 32 行；职责见生产边界。
- `compaction_summary.rs`：宿主生产/端口（含文档、测试注册）79 → 79 行；职责见生产边界。
- `context_injection.rs`：宿主生产/端口（含文档、测试注册）2 → 2 行；职责见生产边界。

宿主来源合计 2621 → 2118，净减 503 行（19.2%），未达 70%，须由收口验收。领域新增测试/夹具 747 行，另增加两处注册；领域生产逻辑新增 0 行。
继续可迁项：宿主 LLM 流测试需领域 runner 自有脚本流夹具；完整 CompactionHost 受 HookHost/McpToolHost 多层宿主约束，不能复制整套宿主伪装领域测试；capability 与 codex 对真实 MCP/工具装配的安全断言应保留真实宿主验收。上述每个保留文件及每条用例已逐项列出，不删除断言凑比例。

## 未来验证
`python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_23_tests -- --test-threads=1`：静态预期 20 条，必须确认 ran > 0 且 failed=0。llm-runtime/agent-acp 未新建模块，不运行零测试过滤。
宿主未来按上述各保留名称使用 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <完整名称> -- --exact --test-threads=1`（名称需加原模块路径），每条预期 1；迁出项未来用宿主 --list 核对消失。全部编译、行为、运行耗时检查均未运行。
静态核对：20 个迁出函数不再挂在原宿主模块；原文件只保留真实集成用例及其夹具，没有通过 path 反向挂载领域测试。

## 恢复深迁（以本节覆盖上轮保留结论）
已读取 final.md 及 remaining-host.json 的 owner=23 残留。新增接收3个测试模块，由现有 host_migration_23_tests 自注册。
- `compaction_empty_output_is_normalized_without_replaying_partial_streams` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_unknown_window_recovers_from_provider_overflow_without_changing_history` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_equal_context_and_output_uses_bounded_request_allocation` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_impossible_budget_preserves_system_and_skips_model` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_overflow_retries_are_bounded_and_preserve_source` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_range_overflow_recovers_single_large_message` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_preflight_bounds_large_tool_output_without_mutating_source` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_reuses_cancellation_and_retry` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_overflow_trims_request_copy_and_keeps_tool_pairs` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `compaction_single_request_preserves_structured_history` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_summary.rs`；保留原断言及名称。
- `agent_config_cleanup_full_access_session_state_preserves_explicit_false` → `crates/libs/session-engine/src/host_migration_23_tests/agent_binding.rs`；保留原断言及名称。
- `agent_session_identity_storage_round_trip_is_scoped_and_secret_free` → `crates/libs/session-engine/src/host_migration_23_tests/agent_binding.rs`；保留原断言及名称。
- `fork_history_copies_only_matching_checkpoints_between_directories` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_checkpoint.rs`；保留原断言及名称。
- `checkpoint_isolated_by_tenant` → `crates/libs/session-engine/src/host_migration_23_tests/compaction_checkpoint.rs`；保留原断言及名称。
- `internal_compaction_single_summary_preserves_all_persisted_history` → `crates/libs/session-engine/src/host_migration_23_tests/compaction.rs`；保留原断言及名称。
- `internal_compaction_keeps_history_when_summary_is_blank_or_fails` → `crates/libs/session-engine/src/host_migration_23_tests/compaction.rs`；保留原断言及名称。
- `internal_compaction_replaces_covered_history_with_a_model_summary` → `crates/libs/session-engine/src/host_migration_23_tests/compaction.rs`；保留原断言及名称。
摘要测试注入的只有 LlmStreamHost 外部响应，真实调用 compaction_summary → recovery → llm_runner 流收集器，保留重试、空输出 replay_safe 元数据、预算、完整消息及请求数量断言。宿主只留异步工作线程取消及失败检查点的两条实际装配断言。
绑定存储直接使用独立临时 home 与真实 GatewaySettingsStore，无恢复模型配置夹具；仍经 session_store 真实租户 task-local。检查点跨项目/租户断言改由 agent_project/session_store 及真实两个 checkpoint 模块持有，MountRevision 外部输入为无挂载版本。
生产端口拆分：CheckpointHost 只提供 checkpoint_tenant 和 emit_compaction。compact_with_summary 与 process_with_summarizer 使用此端口；所有现有 HookHost 通过 blanket impl 调用原 hook_compact::emit，保持生产事件顺序。process/process_session 的完整 HookHost 上下文不变。无新增依赖、无反向挂载。
- 恢复后宿主 `agent_binding_tests.rs`：63 行。
- 恢复后宿主 `capability_policy_tests.rs`：239 行。
- 恢复后宿主 `claude_protocol_parity_tests.rs`：109 行。
- 恢复后宿主 `codex_alignment_tests.rs`：361 行。
- 恢复后宿主 `compaction_tests.rs`：390 行。
- 恢复后宿主 `compaction_checkpoint_tests.rs`：0 行。
- 恢复后宿主 `compaction_summary_tests.rs`：75 行。
- 恢复后宿主 `agent_binding.rs`：116 行。
- 恢复后宿主 `capability_policy.rs`：10 行。
- 恢复后宿主 `compaction.rs`：67 行。
- 恢复后宿主 `compaction_checkpoint.rs`：27 行。
- 恢复后宿主 `compaction_summary.rs`：76 行。
- 恢复后宿主 `context_injection.rs`：2 行。
恢复后来源宿主共 1535 行，对原2621行净减 1086 行（41.4%）。领域测试静态预期共 37 条，未来仍用 session-engine --lib host_migration_23_tests 过滤，当前未运行任何编译、构建或测试。领域新增生产端口约20行，不宣称既有压缩算法为新迁出。
尚存 codex_alignment 全套真实工具/MCP/技能端到端装配、capability_policy 的真实能力发现和范围拒绝、agent_binding 的真实准入分叉边界，以及 compaction 的快照投影和真实会话分叉生命周期。未达到70%门槛，明确保留给收口核对；不以物理移动宿主装配冒充领域迁移。

## 第三轮残留拆分
再次读取 final.md 的 owner23 建议及 remaining-host.json 的13项原路径。
- `manual_compaction_checkpoint_matches_expanded_tool_history` → session-engine/src/host_migration_23_tests/compaction.rs。全部断言保留；真实 llm_messages 投影与 compaction_checkpoint 存储，无宿主快照装配依赖。
- `codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff` → 同上。全部断言保留；真实 session_lifecycle/session_fork、message、skill_checkpoint 及动态工具复制仍执行。LifecycleFixture 只提供 home/version 以及空外部 Hook/MCP 清理（该夹具无外部资源），MountRevision 输入为无挂载版本；没有模拟会话或检查点算法。测试通过既有子进程隔离模式设置临时目录，当前未运行。
- 生产接线：snapshot_checkpoint_source 改为接收投影函数；checkpoint_source 原真实生产调用传 H::project_message。删除宿主专供迁出测试使用的 snapshot_checkpoint_source 与 process_with_summarizer 转发，消除过量测试挂载依赖。
- 剩余 compaction_tests.rs 三条实际 process 入口边界（缺失父用户、无此前历史、空历史），不再保留领域持久化矩阵。原模型/消息夹具仍用于这些真实入口；capability、codex、binding 准入等剩余用例归属见前轮逐项映射。
- 当前 `agent_binding.rs`：116 行。
- 当前 `agent_binding_tests.rs`：63 行。
- 当前 `capability_policy.rs`：10 行。
- 当前 `capability_policy_tests.rs`：239 行。
- 当前 `claude_protocol_parity_tests.rs`：109 行。
- 当前 `codex_alignment_tests.rs`：361 行。
- 当前 `compaction.rs`：59 行。
- 当前 `compaction_tests.rs`：244 行。
- 当前 `compaction_checkpoint.rs`：27 行。
- 当前 `compaction_checkpoint_tests.rs`：0 行。
- 当前 `compaction_summary.rs`：76 行。
- 当前 `compaction_summary_tests.rs`：75 行。
- 当前 `context_injection.rs`：2 行。
当前来源共 1381 行，相对2621行净减 1240 行（47.3%）。本轮额外减 154 行。累计领域测试静态预期39条；未来命令保持 `python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_23_tests -- --test-threads=1`。
静态全仓引用搜索确认 snapshot_checkpoint_source 只有已更新的真实生产入口和迁入用例。diff --check 已执行。未执行 Cargo、构建、测试或实机验证。仍未达70%，不能据此宣称整轮门槛关闭。
