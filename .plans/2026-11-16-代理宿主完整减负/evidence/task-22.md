# 任务22证据

代码静态迁移已提交前核对；全部 Cargo、编译、行为、实机及测试列表检查均未运行。领域迁移及保留宿主端口接线已静态闭合；70%减量门槛未达，须由收口任务验收保留边界；不得作为编译或行为验收通过。

## 本轮实际变化

领域自注册 `session-engine/src/lib.rs -> host_migration_22_tests`，不依赖 vw-agent，不 path/include 宿主源码。既有生产领域实现不计入新增迁移收益。提示测试用显式 system 输入；预览端口只提供已有 tool_output 清理能力，未重写算法。合并重复 ArtifactHost/HelperHost 到现有 AgentRunnerHost，保留原 prompt/store/registry 调用。

## 来源全文件和符号映射

|来源文件|前→后行数|保留符号及职责|
|---|---:|---|
|`types.rs`|8→8|Request：具体 ApprovalManager/SessionSkillSources 的 Request 类型别名|
|`llm_messages.rs`|2→2|重导出/模块注册：既有 session-engine 领域类型/策略薄重导出，无本轮业务迁移收益|
|`prompting.rs`|8→5|build_prompt：宿主 system prompt 输入组合，历史拼装测试全迁出|
|`recovery.rs`|6→6|重导出/模块注册：既有 session-engine 领域类型/策略薄重导出，无本轮业务迁移收益|
|`todos.rs`|12→12|has_incomplete_todos, read_todos_or_empty, maybe_mark_todo_in_progress：真实 Todo 存储及写入工具接线；存储读取策略仍可拆出|
|`runner.rs`|293→166|DesktopSessionProtocol, Target, deref, deref_mut, new, prepare_native_skill_context, run, run_with_persisted_history, run_in_project, request_provider_retry_policy, with_request_model_context, resolve_request_model, should_handle_docs_request_locally, handle_docs_request, handle_structured_tool_calls, handle_assistant_text_branch, handle_empty_assistant_text, build_image_turn_context_for_request, compact_transient_history_for_request：真实宿主模型解析、技能、checkpoint、plugin hooks 和 catalog/子代理执行接线；纯协议断言仍可进一步拆分|
|`llm_runner.rs`|153→140|AgentLlmStreamHost, spawn, spawn_llm_stream_thread, spawn_real_llm_stream_thread, run_llm_step, run_llm_step_with_retry, run_llm_step_with_budget：线程工具/租户/project 上下文捕获、脚本流注入、shell 后台生命周期；脚本收集断言本轮已整组分离|
|`helpers.rs`|17→13|allowed_tool_ids, allowed_tool_ids_for_request, tool_call_preview, query_has_any_tool_calls：真实 registry specs 注册和必需工具筛选接线|
|`mod.rs`|71→71|重导出/模块注册：模块装配和公共兼容出口|
|`artifacts.rs`|48→28|artifact_request, persist_step_ai_call_payload, persist_llm_raw_step_payload, persist_final_ai_call_payload：请求投影、真实 UI 存储与 scoped project 写入；raw payload 纯构造场景本轮已整组迁出|
|`runner_host.rs`|375→375|AgentRunnerHost, CtxHost, ImageExtractor, image_extractor, Admission, Prompt, Connections, PlannedCall, Stream, Prefetch, config, apply_binding, native_defaults, providers, acp_model_source, is_acp_model, message_id, restore_context, admit_context, load_skills, accessible_skills, referenced_skills, explicit_skill_request, invocable_skills, run_video_command, run_bound_command, execution_tools, tool_specs, mcp_connections, refresh_catalog, request_catalog, new_discovery, rebase_discovery, exposed_specs, system_prompt, runtime_prompt, deliver_model_events, team_instructions, planned_call, parallel_limit, stop_hook, read_todos, with_request_tool_catalog, is_agent_tool, run_agent_tools_and_record, run_tool_and_record_with_protocol, run_tool_and_record, run_native_search_and_record, record_native_history_pair, list_docs, restore_skills, save_skills, save_context, compact_hook, run_llm_step, specs, sanitize_tool_input, build_prompt, persist_ai_call_payload：具体 Host 实现：配置、租户、工具、技能、hook、checkpoint 与模型线程入口|
|`runner_admission.rs`|53→53|native, admit_context：plugin turn 快照、hook 生命周期和请求 ToolUseContext 组合|
|`context_governance.rs`|2→2|重导出/模块注册：既有 session-engine 领域类型/策略薄重导出，无本轮业务迁移收益|
|`prefetch.rs`|32→32|AgentPrefetchHost, Context, root, State, default_agent, permission_rules, is_binary, resolve_full_path, run_read, prefetch_allowed, try_prefetch_read, prefetch_seed_context：默认代理权限、真实 file_read、二进制/路径判定；block_on 及空 root 场景本轮已迁出|
|`tools_exec.rs`|133→133|AgentToolPlannedCall, integrate_agent_tool_children, agent_tool_snapshot_status, agent_tool_snapshot_payload, finalize_agent_tool_children_with, prepare_native_search, take_native_output：真实工具权限、审批、catalog snapshot 和子代理 coordinator，不能以模拟端口替代验收|
|`planning.rs`|24→24|AgentPlanningHost, Context, State, run_tool_and_record, ingest_planning_answer_todowrite_only：真实 TodoWrite 执行和持久化；拒绝分支本轮已用不可执行端口迁出|
|`todo_updates.rs`|18→18|maybe_update_todos_after_work：真实 Todo 存储与 AgentRunnerHost 模型/执行接线|
|`turn_policy.rs`|2→2|重导出/模块注册：既有 session-engine 领域类型/策略薄重导出，无本轮业务迁移收益|
|`utils.rs`|33→33|重导出/模块注册：下属路径/工具/消息适配器装配|

来源19文件物理行：1290→1123，净减167（12.95%）。其中本轮新增领域生产算法0行；真实策略早已在领域库，收益是测试编译归属和冗余转发清理。新领域测试物理行共1061。

## 全测试映射（原完整名称→现归属）

### helpers_tests.rs
物理行 63→38；保留理由：真实 registry specs 注册和必需工具筛选接线。

- `allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set` → `原宿主挂载::helpers_tests::allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set（保留）`
- `tool_call_preview_sanitizes_large_or_sensitive_arguments` → `session_engine::host_migration_22_tests::helpers_tests::tool_call_preview_sanitizes_large_or_sensitive_arguments`
- `tool_call_preview_handles_empty_and_short_plain_arguments` → `session_engine::host_migration_22_tests::helpers_tests::tool_call_preview_handles_empty_and_short_plain_arguments`
- `tool_call_preview_truncates_sanitized_arguments_at_character_boundary` → `session_engine::host_migration_22_tests::helpers_tests::tool_call_preview_truncates_sanitized_arguments_at_character_boundary`
### todo_updates_tests.rs
物理行 129→129；保留理由：真实 Todo 存储与 AgentRunnerHost 模型/执行接线。

- `maybe_update_todos_returns_false_when_todowrite_is_not_allowed` → `原宿主挂载::todo_updates_tests::maybe_update_todos_returns_false_when_todowrite_is_not_allowed（保留）`
- `maybe_update_todos_returns_false_when_todo_list_is_empty` → `原宿主挂载::todo_updates_tests::maybe_update_todos_returns_false_when_todo_list_is_empty（保留）`
- `maybe_update_todos_returns_false_when_all_todos_are_completed` → `原宿主挂载::todo_updates_tests::maybe_update_todos_returns_false_when_all_todos_are_completed（保留）`
### llm_runner_tests.rs
物理行 460→353；保留理由：线程工具/租户/project 上下文捕获、脚本流注入、shell 后台生命周期；脚本收集断言本轮已整组分离。

- `llm_runner_tests_module_is_wired` → `原宿主挂载::llm_runner_tests::llm_runner_tests_module_is_wired（保留）`
- `acp_request_detection_accepts_test_flag_and_agent_name` → `session_engine::host_migration_22_tests::llm_runner_tests::acp_request_detection_accepts_test_flag_and_agent_name`
- `collect_stream_times_out_and_closes_open_reasoning` → `session_engine::host_migration_22_tests::llm_runner_tests::collect_stream_times_out_and_closes_open_reasoning`
- `collect_stream_allows_silent_acp_prompt_past_model_idle_timeout` → `session_engine::host_migration_22_tests::llm_runner_tests::collect_stream_allows_silent_acp_prompt_past_model_idle_timeout`
- `collect_stream_reports_empty_when_sender_disconnects_without_done` → `session_engine::host_migration_22_tests::llm_runner_tests::collect_stream_reports_empty_when_sender_disconnects_without_done`
- `llm_error_message_extracts_message_field_from_json` → `session_engine::host_migration_22_tests::llm_runner_tests::llm_error_message_extracts_message_field_from_json`
- `llm_error_message_falls_back_to_raw_string_when_not_json` → `session_engine::host_migration_22_tests::llm_runner_tests::llm_error_message_falls_back_to_raw_string_when_not_json`
- `llm_error_message_falls_back_to_raw_string_when_json_without_message` → `session_engine::host_migration_22_tests::llm_runner_tests::llm_error_message_falls_back_to_raw_string_when_json_without_message`
- `llm_error_message_falls_back_when_json_message_is_not_text` → `session_engine::host_migration_22_tests::llm_runner_tests::llm_error_message_falls_back_when_json_message_is_not_text`
- `acp_request_detection_rejects_unexpected_option_types` → `session_engine::host_migration_22_tests::llm_runner_tests::acp_request_detection_rejects_unexpected_option_types`
- `run_llm_step_uses_scripted_stream_and_filters_allowed_tools` → `session_engine::host_migration_22_tests::scripted_stream_tests::run_llm_step_uses_scripted_stream_and_filters_allowed_tools`
- `run_llm_step_reports_missing_scripted_stream` → `session_engine::host_migration_22_tests::scripted_stream_tests::run_llm_step_reports_missing_scripted_stream`
- `shell_outer_timeout_model_idle_does_not_cover_background_runtime` → `原宿主挂载::llm_runner_tests::shell_outer_timeout_model_idle_does_not_cover_background_runtime（保留）`
- `compaction_range_error_requires_oversized_nonempty_input_and_safe_replay` → `session_engine::host_migration_22_tests::scripted_stream_tests::compaction_range_error_requires_oversized_nonempty_input_and_safe_replay`
- `compaction_range_error_after_partial_output_never_replays` → `session_engine::host_migration_22_tests::scripted_stream_tests::compaction_range_error_after_partial_output_never_replays`
### todos_tests.rs
物理行 128→128；保留理由：真实 Todo 存储及写入工具接线；存储读取策略仍可拆出。

- `has_incomplete_todos_false_for_empty_or_all_completed_and_true_for_pending` → `原宿主挂载::todos_tests::has_incomplete_todos_false_for_empty_or_all_completed_and_true_for_pending（保留）`
- `has_incomplete_todos_treats_cancelled_as_incomplete` → `原宿主挂载::todos_tests::has_incomplete_todos_treats_cancelled_as_incomplete（保留）`
- `read_todos_or_empty_returns_current_items_or_empty_list` → `原宿主挂载::todos_tests::read_todos_or_empty_returns_current_items_or_empty_list（保留）`
- `maybe_mark_todo_in_progress_updates_first_pending_item_silently` → `原宿主挂载::todos_tests::maybe_mark_todo_in_progress_updates_first_pending_item_silently（保留）`
- `maybe_mark_todo_in_progress_noops_when_first_incomplete_is_already_in_progress` → `原宿主挂载::todos_tests::maybe_mark_todo_in_progress_noops_when_first_incomplete_is_already_in_progress（保留）`
- `maybe_mark_todo_in_progress_noops_for_empty_list_without_side_effects` → `原宿主挂载::todos_tests::maybe_mark_todo_in_progress_noops_for_empty_list_without_side_effects（保留）`
### prompting_tests.rs
物理行 164→0；保留理由：宿主 system prompt 输入组合，历史拼装测试全迁出。

- `build_prompt_with_empty_session_contains_only_system_prompt_separator` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_with_empty_session_contains_only_system_prompt_separator`
- `build_prompt_preserves_history_order_and_role_labels` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_preserves_history_order_and_role_labels`
- `build_prompt_appends_trimmed_extra_assistant_text` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_appends_trimmed_extra_assistant_text`
- `build_prompt_ignores_blank_extra_assistant_text` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_ignores_blank_extra_assistant_text`
- `build_prompt_truncates_tool_messages_on_utf8_boundary` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_truncates_tool_messages_on_utf8_boundary`
- `build_prompt_backs_up_when_tool_limit_splits_utf8_character` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_backs_up_when_tool_limit_splits_utf8_character`
- `build_prompt_keeps_tool_message_at_exact_byte_limit` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_keeps_tool_message_at_exact_byte_limit`
- `build_prompt_stops_before_oversized_older_history` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_stops_before_oversized_older_history`
- `build_prompt_stops_at_oversized_newest_message` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_stops_at_oversized_newest_message`
- `build_prompt_accepts_exact_total_budget_and_rejects_one_byte_more` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_accepts_exact_total_budget_and_rejects_one_byte_more`
- `build_prompt_adds_missing_newline_for_message_chunks` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_adds_missing_newline_for_message_chunks`
- `build_prompt_keeps_existing_message_newline_without_double_required_suffix` → `session_engine::host_migration_22_tests::prompting_tests::build_prompt_keeps_existing_message_newline_without_double_required_suffix`
### prefetch_tests.rs
物理行 224→224；保留理由：默认代理权限、真实 file_read、二进制/路径判定；block_on 及空 root 场景本轮已迁出。

- `block_on_drives_pending_future_without_existing_runtime` → `session_engine::host_migration_22_tests::prefetch_tests::block_on_drives_pending_future_without_existing_runtime`
- `block_on_runs_future_inside_existing_runtime` → `session_engine::host_migration_22_tests::prefetch_tests::block_on_runs_future_inside_existing_runtime`
- `app_session_scope_from_root_ignores_missing_or_blank_roots` → `session_engine::host_migration_22_tests::prefetch_tests::app_session_scope_from_root_ignores_missing_or_blank_roots`
- `app_session_scope_from_root_rejects_missing_directory` → `session_engine::host_migration_22_tests::prefetch_tests::app_session_scope_from_root_rejects_missing_directory`
- `try_prefetch_read_returns_false_for_missing_paths_and_directories` → `原宿主挂载::prefetch_tests::try_prefetch_read_returns_false_for_missing_paths_and_directories（保留）`
- `try_prefetch_read_records_text_file_only_when_permission_allows` → `原宿主挂载::prefetch_tests::try_prefetch_read_records_text_file_only_when_permission_allows（保留）`
- `try_prefetch_read_rejects_binary_files_without_recording_tools` → `原宿主挂载::prefetch_tests::try_prefetch_read_rejects_binary_files_without_recording_tools（保留）`
- `prefetch_seed_context_skips_when_query_contains_explicit_tool_call` → `原宿主挂载::prefetch_tests::prefetch_seed_context_skips_when_query_contains_explicit_tool_call（保留）`
- `prefetch_seed_context_deduplicates_repeated_file_candidates` → `原宿主挂载::prefetch_tests::prefetch_seed_context_deduplicates_repeated_file_candidates（保留）`
- `prefetch_seed_context_limits_prefetch_to_first_three_candidates` → `原宿主挂载::prefetch_tests::prefetch_seed_context_limits_prefetch_to_first_three_candidates（保留）`
- `prefetch_seed_context_leaves_session_unchanged_without_file_candidates` → `原宿主挂载::prefetch_tests::prefetch_seed_context_leaves_session_unchanged_without_file_candidates（保留）`
- `prefetch_seed_context_ignores_http_urls_and_absent_files` → `原宿主挂载::prefetch_tests::prefetch_seed_context_ignores_http_urls_and_absent_files（保留）`
### runner_tests.rs
物理行 3104→3104；保留理由：真实宿主模型解析、技能、checkpoint、plugin hooks 和 catalog/子代理执行接线；纯协议断言仍可进一步拆分。

- `runner_tests_module_is_wired` → `原宿主挂载::runner_tests::runner_tests_module_is_wired（保留）`
- `request_model_context_keeps_provider_and_model_together` → `原宿主挂载::runner_tests::request_model_context_keeps_provider_and_model_together（保留）`
- `request_model_context_retains_auto_selection_source` → `原宿主挂载::runner_tests::request_model_context_retains_auto_selection_source（保留）`
- `processor_auto_resolves_model_before_tool_context` → `原宿主挂载::runner_tests::processor_auto_resolves_model_before_tool_context（保留）`
- `processor_step_events_report_selected_model` → `原宿主挂载::runner_tests::processor_step_events_report_selected_model（保留）`
- `session_protocol_initial_step_contains_explicit_workspace_agents` → `原宿主挂载::runner_tests::session_protocol_initial_step_contains_explicit_workspace_agents（保留）`
- `workspace_skill_preload_is_native_only` → `原宿主挂载::runner_tests::workspace_skill_preload_is_native_only（保留）`
- `expert_package_protocol_replays_main_before_child` → `原宿主挂载::runner_tests::expert_package_protocol_replays_main_before_child（保留）`
- `request_reliability_values_feed_main_model_step_policy` → `原宿主挂载::runner_tests::request_reliability_values_feed_main_model_step_policy（保留）`
- `processor_image_assembly_does_not_emit_terminal_error_after_analysis_exhaustion` → `原宿主挂载::runner_tests::processor_image_assembly_does_not_emit_terminal_error_after_analysis_exhaustion（保留）`
- `handle_docs_request_lists_docs_files_and_finishes` → `原宿主挂载::runner_tests::handle_docs_request_lists_docs_files_and_finishes（保留）`
- `handle_docs_request_emits_error_when_docs_directory_is_missing` → `原宿主挂载::runner_tests::handle_docs_request_emits_error_when_docs_directory_is_missing（保留）`
- `handle_docs_request_stops_before_done_when_delta_is_rejected` → `原宿主挂载::runner_tests::handle_docs_request_stops_before_done_when_delta_is_rejected（保留）`
- `run_routes_docs_request_without_starting_an_llm_step` → `原宿主挂载::runner_tests::run_routes_docs_request_without_starting_an_llm_step（保留）`
- `raw_artifact_is_buffered_before_step_finish` → `原宿主挂载::runner_tests::raw_artifact_is_buffered_before_step_finish（保留）`
- `rejected_raw_artifact_stops_before_step_finish` → `原宿主挂载::runner_tests::rejected_raw_artifact_stops_before_step_finish（保留）`
- `acp_context_preflight_preserves_large_prompt_without_local_compaction` → `原宿主挂载::runner_tests::acp_context_preflight_preserves_large_prompt_without_local_compaction（保留）`
- `compaction_lifecycle_replaces_only_covered_history_after_completion` → `原宿主挂载::runner_tests::compaction_lifecycle_replaces_only_covered_history_after_completion（保留）`
- `compaction_lifecycle_keeps_history_and_omits_completion_when_summary_fails` → `原宿主挂载::runner_tests::compaction_lifecycle_keeps_history_and_omits_completion_when_summary_fails（保留）`
- `handle_structured_tool_calls_records_denied_result_for_unallowed_tool` → `原宿主挂载::runner_tests::handle_structured_tool_calls_records_denied_result_for_unallowed_tool（保留）`
- `handle_structured_tool_calls_denies_allowed_tool_on_last_step` → `原宿主挂载::runner_tests::handle_structured_tool_calls_denies_allowed_tool_on_last_step（保留）`
- `handle_assistant_text_branch_accepts_plain_final_text` → `原宿主挂载::runner_tests::handle_assistant_text_branch_accepts_plain_final_text（保留）`
- `legacy_require_agent_tool_option_does_not_reject_plain_final_text` → `原宿主挂载::runner_tests::legacy_require_agent_tool_option_does_not_reject_plain_final_text（保留）`
- `handle_assistant_text_branch_retries_empty_text_with_reasoning_message` → `原宿主挂载::runner_tests::handle_assistant_text_branch_retries_empty_text_with_reasoning_message（保留）`
- `todo_continuation_emits_status_when_incomplete_todos_remain` → `原宿主挂载::runner_tests::todo_continuation_emits_status_when_incomplete_todos_remain（保留）`
- `unfinished_todos_do_not_auto_complete_from_assistant_text` → `原宿主挂载::runner_tests::unfinished_todos_do_not_auto_complete_from_assistant_text（保留）`
- `handle_empty_assistant_text_emits_terminal_error_after_retry_limit` → `原宿主挂载::runner_tests::handle_empty_assistant_text_emits_terminal_error_after_retry_limit（保留）`
- `handle_empty_assistant_text_retries_at_limit_boundary` → `原宿主挂载::runner_tests::handle_empty_assistant_text_retries_at_limit_boundary（保留）`
- `compaction_lifecycle_failure_preserves_multibyte_history` → `原宿主挂载::runner_tests::compaction_lifecycle_failure_preserves_multibyte_history（保留）`
- `compaction_lifecycle_rejects_truncated_summary_without_replacing_history` → `原宿主挂载::runner_tests::compaction_lifecycle_rejects_truncated_summary_without_replacing_history（保留）`
- `compaction_lifecycle_can_summarize_successive_tool_rounds_in_one_turn` → `原宿主挂载::runner_tests::compaction_lifecycle_can_summarize_successive_tool_rounds_in_one_turn（保留）`
- `compaction_checkpoint_replays_after_restart_without_rewriting_chat` → `原宿主挂载::runner_tests::compaction_checkpoint_replays_after_restart_without_rewriting_chat（保留）`
- `compaction_checkpoint_failure_keeps_context_and_never_emits_completion` → `原宿主挂载::runner_tests::compaction_checkpoint_failure_keeps_context_and_never_emits_completion（保留）`
- `model_recovery_preflight_and_network_share_budget` → `原宿主挂载::runner_tests::model_recovery_preflight_and_network_share_budget（保留）`
- `model_recovery_preflight_exhaustion_and_permanent_resolution_stop` → `原宿主挂载::runner_tests::model_recovery_preflight_exhaustion_and_permanent_resolution_stop（保留）`
- `manual_main_model_save_changes_next_auto_request_in_same_session` → `原宿主挂载::runner_tests::manual_main_model_save_changes_next_auto_request_in_same_session（保留）`
- `codex_discovery_processor_search_exposes_only_hit_on_next_request_and_calls_it` → `原宿主挂载::runner_tests::codex_discovery_processor_search_exposes_only_hit_on_next_request_and_calls_it（保留）`
- `codex_skills_transport_processor_preserves_directory_and_body_roles_in_request` → `原宿主挂载::runner_tests::codex_skills_transport_processor_preserves_directory_and_body_roles_in_request（保留）`
- `codex_discovery_processor_same_round_search_cannot_authorize_direct_call` → `原宿主挂载::runner_tests::codex_discovery_processor_same_round_search_cannot_authorize_direct_call（保留）`
- `codex_discovery_processor_same_round_search_cannot_authorize_batch_inner_call` → `原宿主挂载::runner_tests::codex_discovery_processor_same_round_search_cannot_authorize_batch_inner_call（保留）`
- `codex_discovery_processor_failed_search_preserves_visibility_and_repeated_hits_are_unique` → `原宿主挂载::runner_tests::codex_discovery_processor_failed_search_preserves_visibility_and_repeated_hits_are_unique（保留）`
- `codex_discovery_processor_final_step_omits_tools_even_after_loading` → `原宿主挂载::runner_tests::codex_discovery_processor_final_step_omits_tools_even_after_loading（保留）`
- `codex_discovery_processor_retry_preserves_loaded_snapshot_without_duplicates` → `原宿主挂载::runner_tests::codex_discovery_processor_retry_preserves_loaded_snapshot_without_duplicates（保留）`
- `codex_discovery_processor_cancelled_turn_does_not_restore_loaded_tools_from_history` → `原宿主挂载::runner_tests::codex_discovery_processor_cancelled_turn_does_not_restore_loaded_tools_from_history（保留）`
- `codex_discovery_processor_disable_tools_omits_search_and_deferred_tools` → `原宿主挂载::runner_tests::codex_discovery_processor_disable_tools_omits_search_and_deferred_tools（保留）`
- `codex_discovery_processor_forbidden_tool_cannot_be_searched_or_called` → `原宿主挂载::runner_tests::codex_discovery_processor_forbidden_tool_cannot_be_searched_or_called（保留）`
- `codex_discovery_processor_search_disabled_exposes_authorized_mcp_directly` → `原宿主挂载::runner_tests::codex_discovery_processor_search_disabled_exposes_authorized_mcp_directly（保留）`
- `codex_skills_catalog_processor_sends_developer_metadata_and_keeps_it_transient` → `原宿主挂载::runner_tests::codex_skills_catalog_processor_sends_developer_metadata_and_keeps_it_transient（保留）`
- `codex_skills_catalog_processor_reloads_changed_and_removed_sources_between_requests` → `原宿主挂载::runner_tests::codex_skills_catalog_processor_reloads_changed_and_removed_sources_between_requests（保留）`
- `codex_skills_defaults_strict_selection_cannot_expand_catalog_or_invocation` → `原宿主挂载::runner_tests::codex_skills_defaults_strict_selection_cannot_expand_catalog_or_invocation（保留）`
- `codex_skills_defaults_denied_default_fails_before_model_request` → `原宿主挂载::runner_tests::codex_skills_defaults_denied_default_fails_before_model_request（保留）`
- `codex_skills_defaults_unavailable_defaults_fail_before_model_request` → `原宿主挂载::runner_tests::codex_skills_defaults_unavailable_defaults_fail_before_model_request（保留）`
- `codex_skills_defaults_selected_agent_deny_filters_catalog_and_call` → `原宿主挂载::runner_tests::codex_skills_defaults_selected_agent_deny_filters_catalog_and_call（保留）`
- `codex_skills_defaults_invalid_policy_fails_before_model_request` → `原宿主挂载::runner_tests::codex_skills_defaults_invalid_policy_fails_before_model_request（保留）`
- `codex_skills_replay_explicit_discovered_path_activates_body_before_model` → `原宿主挂载::runner_tests::codex_skills_replay_explicit_discovered_path_activates_body_before_model（保留）`
- `codex_skills_replay_successful_tool_survives_new_request_once_and_rejects_rewind` → `原宿主挂载::runner_tests::codex_skills_replay_successful_tool_survives_new_request_once_and_rejects_rewind（保留）`
- `codex_skills_replay_disabled_and_out_of_policy_sources_fail_before_next_model` → `原宿主挂载::runner_tests::codex_skills_replay_disabled_and_out_of_policy_sources_fail_before_next_model（保留）`
- `codex_skills_replay_user_selection_allows_user_only_skill_and_shell_read_does_not_activate` → `原宿主挂载::runner_tests::codex_skills_replay_user_selection_allows_user_only_skill_and_shell_read_does_not_activate（保留）`
- `codex_skills_replay_duplicate_calls_replace_arguments_and_compaction_rebuilds_catalog` → `原宿主挂载::runner_tests::codex_skills_replay_duplicate_calls_replace_arguments_and_compaction_rebuilds_catalog（保留）`
- `codex_plugin_compat_t10_compaction_restores_qualified_invocation_arguments` → `原宿主挂载::runner_tests::codex_plugin_compat_t10_compaction_restores_qualified_invocation_arguments（保留）`
- `codex_plugin_compat_t42_processor_emits_around_real_summary` → `原宿主挂载::runner_tests::codex_plugin_compat_t42_processor_emits_around_real_summary（保留）`
- `codex_plugin_compat_t44_session_final_text_requests_continuation` → `原宿主挂载::runner_tests::codex_plugin_compat_t44_session_final_text_requests_continuation（保留）`
- `agcollab_processor_integrates_before_done` → `原宿主挂载::runner_tests::agcollab_processor_integrates_before_done（保留）`
- `agcollab_processor_cancel_propagates_before_final` → `原宿主挂载::runner_tests::agcollab_processor_cancel_propagates_before_final（保留）`
- `agcollab_processor_gate_reports_budget_and_delivers_failure_once` → `原宿主挂载::runner_tests::agcollab_processor_gate_reports_budget_and_delivers_failure_once（保留）`
- `image_routing_acp_skips_catalog_and_separate_analysis` → `原宿主挂载::runner_tests::image_routing_acp_skips_catalog_and_separate_analysis（保留）`
- `image_routing_text_only_request_does_not_require_model_capabilities` → `原宿主挂载::runner_tests::image_routing_text_only_request_does_not_require_model_capabilities（保留）`
- `acp_model_source_routing_processor_rejects_missing_client_model` → `原宿主挂载::runner_tests::acp_model_source_routing_processor_rejects_missing_client_model（保留）`
- `acp_model_source_routing_processor_local_does_not_resolve_client_model` → `原宿主挂载::runner_tests::acp_model_source_routing_processor_local_does_not_resolve_client_model（保留）`
- `acp_model_source_routing_processor_freezes_real_model_for_step_events` → `原宿主挂载::runner_tests::acp_model_source_routing_processor_freezes_real_model_for_step_events（保留）`
- `agent_config_cleanup_delegate_default_empty_keeps_allowed_catalog` → `原宿主挂载::runner_tests::agent_config_cleanup_delegate_default_empty_keeps_allowed_catalog（保留）`
- `dynamic_tools_processor_replacement_visible_next_step` → `原宿主挂载::runner_tests::dynamic_tools_processor_replacement_visible_next_step（保留）`
- `dynamic_tools_processor_replacement_revokes_loaded_tool` → `原宿主挂载::runner_tests::dynamic_tools_processor_replacement_revokes_loaded_tool（保留）`
- `native_tool_search_desktop_desktop_returns_specialized_search_output` → `原宿主挂载::runner_tests::native_tool_search_desktop_desktop_returns_specialized_search_output（保留）`
- `native_tool_search_desktop_no_hits_and_invalid_query_do_not_load` → `原宿主挂载::runner_tests::native_tool_search_desktop_no_hits_and_invalid_query_do_not_load（保留）`
- `native_tool_search_desktop_same_batch_does_not_authorize_function` → `原宿主挂载::runner_tests::native_tool_search_desktop_same_batch_does_not_authorize_function（保留）`
- `native_tool_search_desktop_replacement_revokes_loaded_tool` → `原宿主挂载::runner_tests::native_tool_search_desktop_replacement_revokes_loaded_tool（保留）`
- `native_tool_search_history_restored_card_does_not_execute_search_again` → `原宿主挂载::runner_tests::native_tool_search_history_restored_card_does_not_execute_search_again（保留）`
### processor_tests.rs
物理行 387→193；保留理由：真实 registry 工具解析和 inline file_read 回填接线。

- `todowrite_completed_rewritten_to_pending_when_merge_false` → `session_engine::host_migration_22_tests::processor_tests::todowrite_completed_rewritten_to_pending_when_merge_false`
- `todowrite_completed_rewritten_to_in_progress_when_merge_true` → `session_engine::host_migration_22_tests::processor_tests::todowrite_completed_rewritten_to_in_progress_when_merge_true`
- `sanitize_tool_input_redacts_file_write_content_for_logs` → `session_engine::host_migration_22_tests::processor_tests::sanitize_tool_input_redacts_file_write_content_for_logs`
- `sanitize_tool_input_for_ui_keeps_full_file_write_content` → `session_engine::host_migration_22_tests::processor_tests::sanitize_tool_input_for_ui_keeps_full_file_write_content`
- `sanitize_tool_input_for_ui_keeps_raw_apply_patch_input` → `session_engine::host_migration_22_tests::processor_tests::sanitize_tool_input_for_ui_keeps_raw_apply_patch_input`
- `sanitize_tool_input_for_ui_uses_remaining_items_marker` → `session_engine::host_migration_22_tests::processor_tests::sanitize_tool_input_for_ui_uses_remaining_items_marker`
- `query_tool_call_detected` → `原宿主挂载::processor_tests::query_tool_call_detected（保留）`
- `parse_tool_at_ignores_incomplete_json_blocks` → `原宿主挂载::processor_tests::parse_tool_at_ignores_incomplete_json_blocks（保留）`
- `parse_tool_at_ignores_invalid_json_on_same_line` → `原宿主挂载::processor_tests::parse_tool_at_ignores_invalid_json_on_same_line（保留）`
- `parse_tool_at_accepts_valid_json_on_same_line` → `原宿主挂载::processor_tests::parse_tool_at_accepts_valid_json_on_same_line（保留）`
- `parse_tool_at_read_recovers_mention_after_malformed_block` → `原宿主挂载::processor_tests::parse_tool_at_read_recovers_mention_after_malformed_block（保留）`
- `inline_tool_execution_appends_tool_context_back_to_llm_messages` → `原宿主挂载::processor_tests::inline_tool_execution_appends_tool_context_back_to_llm_messages（保留）`
- `compact_tool_output_for_ui_keeps_file_link_and_removes_hidden_hint` → `session_engine::host_migration_22_tests::processor_tests::compact_tool_output_for_ui_keeps_file_link_and_removes_hidden_hint`
- `structured_tool_calls_are_not_executed_locally_for_acp_requests` → `session_engine::host_migration_22_tests::processor_tests::structured_tool_calls_are_not_executed_locally_for_acp_requests`
- `structured_tool_calls_still_execute_locally_for_non_acp_requests` → `session_engine::host_migration_22_tests::processor_tests::structured_tool_calls_still_execute_locally_for_non_acp_requests`
### planning_tests.rs
物理行 220→220；保留理由：真实 TodoWrite 执行和持久化；拒绝分支本轮已用不可执行端口迁出。

- `ingest_planning_answer_does_not_autogenerate_when_todowrite_is_not_allowed` → `session_engine::host_migration_22_tests::planning_tests::ingest_planning_answer_does_not_autogenerate_when_todowrite_is_not_allowed`
- `ingest_planning_answer_does_not_run_allowed_todowrite_without_extractable_items` → `session_engine::host_migration_22_tests::planning_tests::ingest_planning_answer_does_not_run_allowed_todowrite_without_extractable_items`
- `ingest_planning_answer_records_explicit_todowrite_without_input` → `原宿主挂载::planning_tests::ingest_planning_answer_records_explicit_todowrite_without_input（保留）`
- `ingest_planning_answer_autogenerates_todowrite_for_list_text` → `原宿主挂载::planning_tests::ingest_planning_answer_autogenerates_todowrite_for_list_text（保留）`
- `ingest_planning_answer_runs_explicit_todowrite_and_skips_implicit_fallback` → `原宿主挂载::planning_tests::ingest_planning_answer_runs_explicit_todowrite_and_skips_implicit_fallback（保留）`
- `ingest_planning_answer_consumes_multiline_todowrite_input_once` → `原宿主挂载::planning_tests::ingest_planning_answer_consumes_multiline_todowrite_input_once（保留）`
- `ingest_planning_answer_ignores_non_todowrite_tool_calls` → `session_engine::host_migration_22_tests::planning_tests::ingest_planning_answer_ignores_non_todowrite_tool_calls`
### tools_exec_tests.rs
物理行 1443→1443；保留理由：真实工具权限、审批、catalog snapshot 和子代理 coordinator，不能以模拟端口替代验收。

- `codex_discovery_processor_batch_inner_call_respects_request_snapshot` → `原宿主挂载::tools_exec_tests::codex_discovery_processor_batch_inner_call_respects_request_snapshot（保留）`
- `background_agent_launch_preserves_running_lifecycle_in_session_output` → `原宿主挂载::tools_exec_tests::background_agent_launch_preserves_running_lifecycle_in_session_output（保留）`
- `three_agent_tool_executions_start_in_parallel_and_keep_result_order` → `原宿主挂载::tools_exec_tests::three_agent_tool_executions_start_in_parallel_and_keep_result_order（保留）`
- `configured_agent_tool_limit_caps_parallel_execution` → `原宿主挂载::tools_exec_tests::configured_agent_tool_limit_caps_parallel_execution（保留）`
- `tool_classification_helpers_cover_todo_streaming_and_regular_tools` → `原宿主挂载::tools_exec_tests::tool_classification_helpers_cover_todo_streaming_and_regular_tools（保留）`
- `run_tool_and_record_reports_parse_errors_for_streaming_tools_and_deduplicates` → `原宿主挂载::tools_exec_tests::run_tool_and_record_reports_parse_errors_for_streaming_tools_and_deduplicates（保留）`
- `run_tool_and_record_delegates_batch_tool` → `原宿主挂载::tools_exec_tests::run_tool_and_record_delegates_batch_tool（保留）`
- `run_tool_and_record_rewrites_initial_todowrite_completed_status_until_non_todo_work_exists` → `原宿主挂载::tools_exec_tests::run_tool_and_record_rewrites_initial_todowrite_completed_status_until_non_todo_work_exists（保留）`
- `run_batch_tool_and_record_handles_invalid_json_recursive_and_empty_batches` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_handles_invalid_json_recursive_and_empty_batches（保留）`
- `run_batch_tool_and_record_records_failed_subtool_and_summary` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_records_failed_subtool_and_summary（保留）`
- `run_batch_tool_and_record_summarizes_single_and_parallel_successful_batches` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_summarizes_single_and_parallel_successful_batches（保留）`
- `run_batch_tool_and_record_passes_schema_args_and_legacy_parameters` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_passes_schema_args_and_legacy_parameters（保留）`
- `run_tool_and_record_records_successful_todoread_and_deduplicates_repeated_calls` → `原宿主挂载::tools_exec_tests::run_tool_and_record_records_successful_todoread_and_deduplicates_repeated_calls（保留）`
- `run_tool_and_record_evaluates_trace_fields_when_subscriber_is_enabled` → `原宿主挂载::tools_exec_tests::run_tool_and_record_evaluates_trace_fields_when_subscriber_is_enabled（保留）`
- `run_tool_and_record_saturates_non_todo_counter_without_emitting_ui` → `原宿主挂载::tools_exec_tests::run_tool_and_record_saturates_non_todo_counter_without_emitting_ui（保留）`
- `run_tool_and_record_records_permission_denial_with_request_context` → `原宿主挂载::tools_exec_tests::run_tool_and_record_records_permission_denial_with_request_context（保留）`
- `run_tool_and_record_preserves_editing_tool_success_in_session_history` → `原宿主挂载::tools_exec_tests::run_tool_and_record_preserves_editing_tool_success_in_session_history（保留）`
- `run_batch_tool_and_record_accepts_tool_calls_alias_and_summarizes_mixed_results` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_accepts_tool_calls_alias_and_summarizes_mixed_results（保留）`
- `run_batch_tool_and_record_records_denied_subtool_with_permission_request` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_records_denied_subtool_with_permission_request（保留）`
- `run_batch_tool_and_record_preserves_editing_subtool_and_parallel_ui_events` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_preserves_editing_subtool_and_parallel_ui_events（保留）`
- `run_batch_tool_and_record_emits_running_and_error_events_for_streaming_subtool` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_emits_running_and_error_events_for_streaming_subtool（保留）`
- `run_batch_tool_and_record_deduplicates_subtool_and_batch_messages` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_deduplicates_subtool_and_batch_messages（保留）`
- `run_batch_tool_and_record_rewrites_todowrite_only_before_non_todo_work` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_rewrites_todowrite_only_before_non_todo_work（保留）`
- `run_batch_tool_and_record_uses_existing_tokio_runtime_for_parallel_calls` → `原宿主挂载::tools_exec_tests::run_batch_tool_and_record_uses_existing_tokio_runtime_for_parallel_calls（保留）`
- `agent_tool_finalization_emits_terminal_payloads_before_done` → `原宿主挂载::tools_exec_tests::agent_tool_finalization_emits_terminal_payloads_before_done（保留）`
- `agent_tool_finalization_keeps_background_child_running_when_event_sink_closes` → `原宿主挂载::tools_exec_tests::agent_tool_finalization_keeps_background_child_running_when_event_sink_closes（保留）`
- `shell_session_batch_records_real_independent_handles` → `原宿主挂载::tools_exec_tests::shell_session_batch_records_real_independent_handles（保留）`
- `shell_session_failed_command_retains_stderr_and_exit_code` → `原宿主挂载::tools_exec_tests::shell_session_failed_command_retains_stderr_and_exit_code（保留）`
- `codex_plugin_compat_t31_stream_preserves_app_without_content_blocks` → `原宿主挂载::tools_exec_tests::codex_plugin_compat_t31_stream_preserves_app_without_content_blocks（保留）`
- `dynamic_tools_processor_catalog_preserves_approval_and_hook_denials` → `原宿主挂载::tools_exec_tests::dynamic_tools_processor_catalog_preserves_approval_and_hook_denials（保留）`
- `native_tool_search_desktop_executor_requires_real_success_and_preserves_hooks` → `原宿主挂载::tools_exec_tests::native_tool_search_desktop_executor_requires_real_success_and_preserves_hooks（保留）`
- `native_tool_search_desktop_ordinary_approval_and_hooks_still_deny` → `原宿主挂载::tools_exec_tests::native_tool_search_desktop_ordinary_approval_and_hooks_still_deny（保留）`
### artifacts_tests.rs
物理行 553→553；保留理由：请求投影、真实 UI 存储与 scoped project 写入；raw payload 纯构造场景本轮已整组迁出。

- `step_and_final_ai_call_payloads_persist_to_scoped_session` → `原宿主挂载::artifacts_tests::step_and_final_ai_call_payloads_persist_to_scoped_session（保留）`
- `ai_call_payloads_compact_repeated_context_before_persisting` → `原宿主挂载::artifacts_tests::ai_call_payloads_compact_repeated_context_before_persisting（保留）`
- `ai_call_payloads_respect_persistence_guards` → `原宿主挂载::artifacts_tests::ai_call_payloads_respect_persistence_guards（保留）`
- `final_ai_call_payload_uses_explicit_persist_decision` → `原宿主挂载::artifacts_tests::final_ai_call_payload_uses_explicit_persist_decision（保留）`
- `raw_payload_builds_system_messages_output_and_tool_calls` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_builds_system_messages_output_and_tool_calls`
- `raw_payload_builds_message_sources_without_provider_metadata` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_builds_message_sources_without_provider_metadata`
- `raw_payload_builds_request_snapshot_and_tool_catalog` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_builds_request_snapshot_and_tool_catalog`
- `raw_payload_options_are_explicitly_allowlisted` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_options_are_explicitly_allowlisted`
- `raw_payload_serializes_empty_optional_output` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_serializes_empty_optional_output`
- `raw_payload_respects_persistence_guards` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_respects_persistence_guards`
- `raw_payload_build_does_not_access_storage` → `session_engine::host_migration_22_tests::artifacts_tests::raw_payload_build_does_not_access_storage`

runner_tests 的 mcp_catalog_refresh_tests 子模块仍随原宿主挂载保留，未将其算作迁移；该子模块属于后续可拆分范围。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p session-engine --lib host_migration_22_tests -- --test-threads=1`：静态预期 51 个，必须非零。

宿主保留项使用上表名称加 `session::processor::` 实际模块前缀作精确过滤，未来先 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list` 获取真实完整名。迁出项应在宿主列表消失；零测试不能作为通过。未新增 llm-runtime/agent-acp 测试，不运行其空过滤。

## 未达减量门槛的后续项

已拆分 raw artifact 构造、prefetch 无权限依赖分支、planning 不执行分支；继续可迁候选是 runner 中与具体技能/配置重叠的协议断言，保留真实安全/执行端口验收。这些候选仍牵涉具体宿主配置/存储/执行验收，未用模拟宿主替代；保留文件及符号职责见上表，减量例外提交收口验收。

## 续执行静态核对

- 原始 raw payload 全部用例移至 artifacts_tests，真实 scoped 持久化4条继续由宿主执行；删除不再使用的 raw 转发和 ToolSpec fixture。
- 规划三条拒绝路径迁入 GuardHost：仅在不应调用执行时 panic，未模拟权限或存储；四条真实 TodoWrite 验收保留。
- prefetch 的运行时桥接/空路径/缺失目录四条迁出；权限、真实 file_read 与二进制访问验收保留。
- 剩余四条纯 LLM 请求筛选/溢出/不可重放策略迁入 scripted_stream_tests，脚本仅替代外部模型流。宿主保留租户/目录请求记录、取消流夹具及真实 shell 生命周期验收。
- runner 九个 cfg(test) 转发已删除，原宿主集成断言直接调用 session_engine 泛型入口；结构化调用保留 DesktopSessionProtocol 解包适配。
- 未增加依赖；没有把 App、AppState 或完整具体宿主拷入领域库；无领域反向依赖。
- git diff --check 针对本任务文件通过；未调用 Cargo/rustfmt/构建/测试可执行文件。源码测试数量和调用核对不等同编译或行为通过。

### 嵌套挂载补充

原 runner_tests::mcp_catalog_refresh_tests 的两条真实 MCP fixture 用例完整保留：
- dynamic_tools_mcp_refresh_processor_pins_current_request_and_updates_next_request
- dynamic_tools_mcp_refresh_processor_initializes_native_catalog

### 精确未来非零检查

宿主限定过滤：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::processor::artifacts::artifacts_tests -- --test-threads=1`，预期4条。
领域限定过滤使用上文 host_migration_22_tests 命令；保留宿主薄边界和迁出列表需再用实际 --list 校验。所有命令本轮未运行。
