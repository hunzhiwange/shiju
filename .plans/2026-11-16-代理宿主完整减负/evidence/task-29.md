# 任务29静态迁移证据

代码完成，编译/行为未验证。遵照用户禁止，本次未执行 Cargo（包括 list/fmt/check）、构建、单元或实机测试；静态 diff 检查不代表编译通过。

## 生产接线与归属

原宿主 catalog_tests 引用的 load_agent_configs 在最新宿主已无定义。实际实现位于 agent-loop::definitions；本次将其完整函数体迁入 agent-config::catalog::load_agent_configs，definitions 和宿主薄配置测试直接引用同一实现，无复制算法。生产 definitions::init_state 继续调用该入口，再调用既有 merged_agent_catalog，agent-config 依赖 config-agent/acp-agent-registry；依赖始终向领域库，不反向依赖 vw-agent。新测试仅使用两库既有依赖，无 Cargo.toml/Cargo.lock 修改。

旧 catalog_tests 22 项改在 agent-config::host_migration_29_tests 自注册。跨目录 memory_context 23 项由 agent-loop::host_migration_29_tests 自注册，真实 SqliteMemory 不变，测试函数体和断言未修改。无 path/include 加载宿主源码。旧宿主 mod catalog_tests、memory_context 挂载均已删除；其它 path 保留真实宿主契约（如下）。已存在领域实现和先前迁移不计本轮收益。

## 全来源文件映射（splitlines）

| 原文件 | 前→后行数 | 目标或保留职责 |
|---|---:|---|
| `agent/catalog.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/catalog_tests.rs` | 326→0 | 22 个完整函数迁至 agent-config/src/host_migration_29_tests.rs，自注册。 |
| `agent/classifier/mod.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/defaults.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/dispatcher/mod.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/approval.rs` | 5→5 | NonCliApprovalContext/Prompt、审批 task-local 及 await 决策的兼容导出，真实 core 调用者仍需要。 |
| `agent/loop_/context.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/cron.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/execution.rs` | 22→22 | schedule_tool_batches 将解析调用转为 dispatcher 输入；执行器直接导出。保留真实权限/MCP context 的宿主测试挂载。 |
| `agent/loop_/history.rs` | 18→18 | HistoryHost.restore/save/summarize/emit 对真实 checkpoint、共享摘要 runner、插件钩子的装配及 compact_history_with_internal_service 入口。 |
| `agent/loop_/history_tests.rs` | 126→126 | 4 项真实摘要 runner/失败/插件钩子契约；保留并逐项列在测试映射。 |
| `agent/loop_/instructions.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/progress.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/query_engine.rs` | 25→25 | QueryEngine 类型绑定 NativeCoreSkillHost；assemble_tools、team_runtime_instructions、restore_history、compact_history、agent_turn 注入真实 runtime/security/memory/tools 与租户状态；保留现有宿主测试绑定。 |
| `agent/loop_/runner.rs` | 7→7 | process_message 将领域 runner 绑定 NativeCoreSkillHost；2 项真实宿主测试。 |
| `agent/loop_/runner_tests.rs` | 112→112 | 真实工具/运行时装配、环境变量隔离与本地网关 503 契约；不能用领域替身替代。 |
| `agent/loop_.rs` | 201→201 | 宿主 core/query_engine/history/runner 装配及现有公共/测试兼容出口；无领域算法迁移收益。 |
| `agent/loop_/parsing/deepseek_dsml.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/parsing/json.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/parsing/minimax_xml.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/parsing/mod.rs` | 61→60 | 兼容调用者的 ParsedToolCall/parse_*、凭据与协议辅助导出；去掉五层子文件转发，无解析算法变动。 |
| `agent/loop_/parsing/tool_call_formats.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/parsing/tool_calls.rs` | 4→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/loop_/tests/mod.rs` | 65→63 | 仅保留 loop_tools（真实宿主审批/工具循环）与 tool_instructions（真实 default_tools 注册）挂载；memory_context 整组已改为领域自注册。 |
| `agent/memory_loader.rs` | 2→0 | 已删除；兼容公共路径由父模块直接导出实际领域模块，不再落盘转发。 |
| `agent/mod.rs` | 97→97 | DefinitionHost（配置、租户、技能目录、data_dir）及 get/default_agent/list/permission_rules/native_permission_ceiling 的具体泛型装配；catalog/classifier/defaults/dispatcher/memory_loader 已直接导出领域模块。 |
| `agent/prompt/mod.rs` | 6→6 | prompt 兼容导出及跨 channels/task_plan_run 的真实提示词接线测试。 |

本组宿主 1101→742 行，净减 359 行（32.6%），低于70%。不足原因逐文件见上表：主要保留 history_tests 126行、runner_tests 112行真实服务/网关边界，以及 loop_.rs 201行公共装配出口；不能以删断言或纯删注释满足门槛。继续可迁项为 execution::schedule_tool_batches 的类型归一化与 loop_ 兼容出口调用者收敛；须保留当前真实宿主集成契约，由收口验收减量缺口，不能宣称整轮达标。

## 全用例映射

以下每行代表原函数名保持不变（新模块前缀按目标变化）。

### agent/catalog_tests.rs → agent-config::host_migration_29_tests

- `agent_catalog_covers_thirty_unique_backends`
- `agent_catalog_preserves_native_role_limits`
- `agent_catalog_preserves_official_identity_and_editable_overrides`
- `agent_catalog_keeps_all_backends_without_installed_clis`
- `acp_onboarding_autohand_builtin_preset_metadata`
- `acp_onboarding_autohand_catalog_binding`
- `acp_onboarding_deepagents_builtin_preset_metadata`
- `acp_onboarding_deepagents_catalog_binding`
- `acp_onboarding_dimcode_builtin_preset_metadata`
- `acp_onboarding_dimcode_catalog_binding`
- `acp_onboarding_dirac_builtin_preset_metadata`
- `acp_onboarding_dirac_catalog_binding`
- `acp_onboarding_glm_acp_agent_builtin_preset_metadata`
- `acp_onboarding_glm_acp_agent_catalog_binding`
- `acp_onboarding_grok_builtin_preset_metadata`
- `acp_onboarding_grok_catalog_binding`
- `acp_onboarding_nova_builtin_preset_metadata`
- `acp_onboarding_nova_catalog_binding`
- `acp_onboarding_sigit_builtin_preset_metadata`
- `acp_onboarding_sigit_catalog_binding`
- `acp_onboarding_amp_acp_builtin_preset_metadata`
- `acp_onboarding_amp_acp_catalog_binding`

### agent/loop_/tests::memory_context → agent-loop::host_migration_29_tests::memory_context

- `test_scrub_credentials`
- `test_scrub_credentials_json`
- `scrub_credentials_empty_input`
- `scrub_credentials_no_sensitive_data`
- `scrub_credentials_short_values_not_redacted`
- `trim_history_preserves_system_prompt`
- `trim_history_noop_when_within_limit`
- `build_compaction_transcript_formats_roles`
- `apply_compaction_summary_replaces_old_segment`
- `autosave_memory_key_has_prefix_and_uniqueness`
- `autosave_memory_keys_preserve_multiple_turns`
- `build_context_ignores_legacy_assistant_autosave_entries`
- `trim_history_empty_history`
- `trim_history_system_only`
- `trim_history_exactly_at_limit`
- `trim_history_removes_oldest_non_system`
- `trim_history_with_no_system_prompt`
- `trim_history_preserves_role_ordering`
- `trim_history_with_only_system_prompt`
- `build_native_assistant_history_includes_reasoning_content`
- `build_native_assistant_history_omits_reasoning_content_when_none`
- `build_native_assistant_history_from_parsed_calls_includes_reasoning_content`
- `build_native_assistant_history_from_parsed_calls_omits_reasoning_content_when_none`

### agent/loop_/history_tests.rs → 保留 agent::loop_::history::history_tests

- `internal_compaction_returns_false_without_prior_history`

### agent/loop_/runner_tests.rs → 保留 agent::loop_::runner::runner_tests

- `process_message_returns_error_for_unknown_provider`
- `process_message_uses_default_provider_fallback_before_submit_error`

## 跨目录宿主测试挂载

下列文件仍由原宿主边界注册，不计已迁出；每项名称沿用原文件及模块树。无修改其函数体。definitions 的领域自注册由任务26建立，本任务仅修复宿主 load_agent_configs 导入。

- `tests/agent_facade_host.rs`: `info_serialization_flattens_and_round_trips_definition`, `primary_visible_excludes_hidden_and_subagents`, `choose_default_agent_validates_configured_default`, `choose_default_agent_uses_configured_then_builtin_priority`, `choose_default_agent_falls_back_alphabetically_or_errors`, `normalize_agent_mode_accepts_known_aliases`, `merge_json_value_deep_merges_objects_and_replaces_scalars`, `load_agent_configs_merges_configured_agents_with_builtins`, `load_agent_configs_discards_malformed_configured_map`, `load_agent_configs_uses_builtins_when_agents_key_is_missing`, `resolve_model_ref_handles_direct_split_and_missing_values`, `resolve_model_ref_trims_components_and_prefers_explicit_provider`, `build_resolved_agent_applies_runtime_defaults`, `public_delegate_agents_resolve_dedicated_role_prompts`, `build_resolved_agent_orders_permission_layers`, `build_resolved_agent_allows_selected_skills`, `build_resolved_agent_keeps_global_skill_deny_authoritative`, `build_resolved_agent_preserves_custom_builtin_prompts`, `build_resolved_agent_keeps_explicit_tool_output_deny`, `load_state_skips_disabled_agents_and_preserves_default_name`, `prompt_and_truncate_glob_are_non_empty`, `agent_native_defaults_skill_grants_preserve_user_denial_and_fixed_empty`

- `tests/agent_host_suite/mod.rs`:

- `execution_tests.rs`: `sequential_success_scrubs_output_and_enriches_result_dto`, `tool_result_failure_is_returned_as_failed_outcome_with_error_prefix`, `failed_tool_result_without_error_uses_model_output_as_reason`, `execution_error_and_permission_denial_become_failed_outcomes`, `unknown_tool_becomes_failed_outcome_with_dto`, `parallel_execution_collects_all_outcomes_without_short_circuiting_failures`, `empty_tool_calls_return_empty_outcomes`, `cancellation_token_stops_execution_before_tool_runs`

- `query_engine_test_binding.rs`:

- `query_engine_tests.rs`: `codex_discovery_core_query_engine_loads_and_resets_tools_between_user_turns`, `agent_and_query_engine_use_ephemeral_query_engine_context_each_turn`, `agent_and_query_engine_preserve_history_with_ephemeral_context_each_turn`, `query_engine_usage_totals_billable_tokens`, `query_engine_usage_total_saturates_on_overflow`, `recording_observer_accumulates_successful_llm_usage_and_forwards_calls`, `recording_observer_records_success_with_missing_usage_as_zero`, `recording_observer_saturates_oversized_token_counts`, `session_state_counts_non_system_messages_and_remaining_budget`, `snapshot_replace_and_restore_round_trip_history_turns_and_usage`, `submit_message_success_enriches_history_and_records_usage`, `submit_message_without_memory_context_uses_timestamp_prefix`, `first_turn_compaction_allows_threshold_but_rejects_overflow`, `submit_message_timestamp_uses_configured_timezone`, `new_rejects_invalid_programmatic_timezone`, `submit_message_success_preserves_history_after_turn`, `submit_message_error_rolls_back_history_and_turn_count`, `govern_history_does_not_apply_message_count_trimming`, `govern_history_preserves_history_after_a_response`, `govern_history_never_uses_local_summary_fallback`, `build_tool_descriptions_tracks_optional_tools`, `build_tool_descriptions_includes_browser_open_when_enabled`, `agcollab_channel_request_contains_only_enabled_team_runtime`

- `tests/prompt_host.rs`: `base_and_task_plan_prompts_share_recursive_orchestration_policy`

- `tests/loop_suite/tool_instructions.rs`: `build_tool_instructions_includes_all_tools`, `tools_to_openai_format_produces_valid_schema`

- `tests/loop_suite/loop_tools/multimodal.rs`: `run_tool_call_loop_returns_structured_error_for_non_vision_provider`, `run_tool_call_loop_rejects_oversized_image_payload`, `run_tool_call_loop_accepts_valid_multimodal_request_flow`

- `tests/loop_suite/loop_tools/parallel_execution.rs`: `scheduler_groups_read_only_concurrency_safe_calls_into_parallel_batch`, `scheduler_keeps_non_read_only_calls_serial`, `scheduler_splits_parallel_groups_around_serial_barriers`, `run_tool_call_loop_executes_multiple_tools_with_ordered_results`

- `tests/loop_suite/loop_tools/recovery.rs`: `run_tool_call_loop_executes_repaired_dsml_through_existing_permissions`, `run_tool_call_loop_rejects_malformed_dsml_without_final_text`, `run_tool_call_loop_deduplicates_repeated_tool_calls`, `run_tool_call_loop_retries_when_response_claims_completion_without_tool_call`, `run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call`, `looks_like_unverified_action_completion_without_tool_call_detects_claimed_side_effects`, `looks_like_unverified_action_completion_without_tool_call_ignores_non_side_effect_text`

- `tests/loop_suite/loop_tools/approvals.rs`: `run_tool_call_loop_denies_supervised_tools_on_non_cli_channels`, `run_tool_call_loop_waits_for_non_cli_approval_resolution`, `run_tool_call_loop_consumes_one_time_non_cli_allow_all_token`, `run_tool_call_loop_blocks_tools_excluded_for_channel`

- `tests/loop_suite/loop_tools/native_mode.rs`: `run_tool_call_loop_native_mode_preserves_fallback_tool_call_ids`, `native_tools_system_prompt_contains_zero_xml`

## 行数与未来验证

宿主测试物理行数：catalog 326→0，history 126→126，runner 112→112；跨目录 memory_context 的物理路径不变，编译归属由宿主转为领域，不能算宿主物理减量。其余来源为生产装配/导出及 cfg(test) 注册，逐项见表，不把这些全称为算法。领域生产 load_agent_configs 11行从 agent-loop 移至 agent-config（函数体相同）；领域测试新增 catalog 326行，memory_context 原断言原地保留，新增10行自注册适配及两处3行 lib 注册。

未来顺序运行（本次全部未运行）：

- `python3 scripts/cargo_slot.py cargo test -p agent-config --lib host_migration_29_tests -- --test-threads=1`：预期22项。
- `python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_29_tests -- --test-threads=1`：预期23项。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::loop_::history::history_tests -- --test-threads=1`：预期4项。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::loop_::runner::runner_tests -- --test-threads=1`：预期2项。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`：核对旧 catalog_tests、tests::memory_context 完整名称消失；零测试不作为通过证据。

静态检查：读取直接入口与注册、确认测试函数名/断言保留、无新增反向依赖；`git diff --check` 无输出。索引早于当前共享修改，RustCodeGraph status 可读，最新变更使用源码核对。未声称通过编译、行为或性能验收。
