# 任务28静态迁移证据

状态：代码完成，编译/行为未验证。前次部分实现提交 2c92f73d8 的缺口已在本轮闭合，不再将实施困难列作外部阻塞。遵守用户覆盖，本次未运行任何 Cargo、构建、单元测试、测试可执行文件或实机测试。

## 架构与真实接线

生产规则此前已归 agent-loop；本轮生产算法迁移收益为0，删除的是测试专用转发和宿主场景正文。宿主 services() → agent_loop::core::llm_round / tool_execution 的真实生产链保持不变。

agent-loop 自注册 host_migration_28_tests（cfg(test) 或现有 test-support feature；宿主已有 dev-dependency 启用，无新增依赖）。单轮/输出/历史ID纯领域测试由库自身编译注册。discovery 与 tool_execution 为同一模块下领域场景函数，由宿主薄测试传入真实 NativeCoreSkillHost / ToolExecutionServices；领域泛型函数直接调用既有 Agent/AgentHost、ToolUseContext、ToolCatalog、BatchTool 与工具执行器，不复制 App/AppState，不引入放行权限mock，不改现有安全判断。

因此：物理场景正文归领域库，但30条泛型宿主场景仍在宿主 lib-test 中单态化并执行，不能声称它们已从宿主测试列表消失或获得编译时间收益。原有名称保留用于真实端口验收；24条纯领域测试的旧宿主挂载则已经删除。真实插件事件的1条保留宿主全过程。

依赖方向：vw-agent(dev test-support) → agent-loop → tool-context / tool-assembly / tool-services / model-types / agent-*；无 vw-agent 反向依赖，无 path/include 加载宿主源码，无领域文件被宿主 path 加载。唯一留在宿主的业务断言是明确的插件t41边界；其余宿主入口仅装配和调用领域场景。

任务1 external-mounts.json 静态核对：本组9个来源无额外宿主跨目录挂载目标。collaboration/detection 的调用者 tool_loop_tests 属其它任务，故保留其现有5/2行真实端口和兼容入口。共享 lib.rs 的本任务注册已经落盘并在并发提交99c7f786d中被一并保存；本任务不覆盖其它注册。

## 分项行数

|来源|基线行数|现行数|具体保留/迁出职责|
|---|---:|---:|---|
|collaboration.rs|5|5|integrate：tool_loop_tests 的真实子代理协调器服务端口，调用 agent_loop::core::tool_loop::integrate；无领域算法。|
|detection.rs|2|2|完成声明检测公开测试兼容 re-export；仍有直接调用者，规则在 core::detection。|
|discovery_tests.rs|809|98|19个 NativeCoreSkillHost 泛型实例化入口（实际数量以下表为准）；真实 AgentHost、当前租户/工作树租约、MCP发现/目录、BatchHost 与鉴权仍使用宿主实现；场景、文件工具与全部断言已迁出。|
|llm_round.rs|127|104|run_llm_round/with_extractor/services/LlmRoundResult：真实 SessionImageTextExtractor 和插件模型事件/解析/历史端口装配；删除 assign_fallback_tool_call_ids、build_assistant_history 测试转发。|
|llm_round_tests.rs|624|194|仅 codex_plugin_compat_t41_real_provider_failure_and_success 及 TestProvider/RecordingObserver/run_test_round 最小脚本夹具；断言真实插件6事件顺序、失败/取消/成功、敏感信息不泄露和hook上下文释放。|
|output.rs|25|0|删除测试专用 wire/update_thinking_progress/update_tool_call_progress/send_retry_progress/stream_final_response 转发；实际生产循环此前已调用领域 output。|
|output_tests.rs|132|0|全部迁入自注册 output；原文件和旧挂载已删除。|
|tool_execution.rs|125|110|execute_tool_calls_and_update_history/services/NativeHistory：真实执行、调度、cron、native search历史序列化端口；删除 immediate_failure/history_tool_call_id/send_structured_tool_result_progress 测试转发。|
|tool_execution_tests.rs|725|62|11个真实执行服务实例化入口；其中MCP边界构造真实 ToolRuntimeContext 并调用 tool_specs_for_context。脚本、目录夹具、工具实现、场景及断言全部在领域库。|

宿主9文件：2574 → 575，净减 1999 行（77.7%），超过70%门槛；直接父模块另删除2行 output 挂载（首个提交）。

生产端口文件（非纯业务行数）：252 → 214；其余测试/测试转发：2322 → 361。生产文件内删除测试专用导入与辅助转发，不计为算法迁出。领域本任务新增/迁入：生产0行，测试、泛型场景、夹具及服务绑定 2468 行；lib.rs 3行领域测试模块注册单列。宿主保留的TestProvider/RecordingObserver是外部端口脚本夹具，与领域脚本分别服务真实插件和纯轮次测试，没有复制宿主状态。

## 全文件/全用例逐项映射

|原文件及用例|领域场景/断言位置|运行注册|
|---|---|---|
|discovery_tests.rs::codex_discovery_core_agent_discovers_and_calls_only_on_next_request|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_agent_discovers_and_calls_only_on_next_request|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_same_round_search_does_not_allow_deferred_call|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_same_round_search_does_not_allow_deferred_call|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_excluded_tool_cannot_be_searched_or_called|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_excluded_tool_cannot_be_searched_or_called|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_1.rs::codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_empty_registry_provides_no_tool_protocol|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::codex_discovery_core_empty_registry_provides_no_tool_protocol|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::dynamic_tools_core_replacement_visible_next_request|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::dynamic_tools_core_replacement_visible_next_request|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::dynamic_tools_core_replacement_preserves_same_batch_rejection|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::dynamic_tools_core_replacement_preserves_same_batch_rejection|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::dynamic_tools_core_replacement_intersects_loaded_authorization|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_2.rs::dynamic_tools_core_replacement_intersects_loaded_authorization|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::dynamic_tools_core_replacement_cannot_expand_child_authority|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::dynamic_tools_core_replacement_cannot_expand_child_authority|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_core_calls_discovered_tool_next_request|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::native_tool_search_core_loop_core_calls_discovered_tool_next_request|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_same_batch_remains_denied|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::native_tool_search_core_loop_same_batch_remains_denied|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_rejects_unadvertised_and_invalid_search|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::native_tool_search_core_loop_rejects_unadvertised_and_invalid_search|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_ordinary_discovery_still_executes|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_3.rs::native_tool_search_core_loop_ordinary_discovery_still_executes|vw-agent 原完整测试路径（真实端口薄入口）|
|discovery_tests.rs::native_tool_search_core_loop_replayed_search_is_not_completed_again|crates/libs/agent-loop/src/host_migration_28_tests/discovery/scenarios_4.rs::native_tool_search_core_loop_replayed_search_is_not_completed_again|vw-agent 原完整测试路径（真实端口薄入口）|
|llm_round_tests.rs::fallback_tool_call_ids_are_stable_per_iteration|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::fallback_tool_call_ids_are_stable_per_iteration|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::assistant_history_handles_plain_text_and_missing_native_ids|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::assistant_history_handles_plain_text_and_missing_native_ids|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_plain_success_records_usage_and_text|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_plain_success_records_usage_and_text|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_accepts_image_markers_for_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_accepts_image_markers_for_vision_provider|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_text_fallback_assigns_ids_and_native_history|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_text_fallback_assigns_ids_and_native_history|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_detects_malformed_tool_call_parse_issue|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_detects_malformed_tool_call_parse_issue|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_native_tool_calls_skip_text_fallback|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_native_tool_calls_skip_text_fallback|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_rejects_image_markers_for_non_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_rejects_image_markers_for_non_vision_provider|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_uses_ocr_before_non_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_uses_ocr_before_non_vision_provider|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_fires_llm_input_hook|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_fires_llm_input_hook|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_cancellation_wins_over_slow_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_cancellation_wins_over_slow_provider|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::run_llm_round_provider_error_records_failed_response|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_provider_error_records_failed_response|agent-loop::host_migration_28_tests|
|llm_round_tests.rs::codex_plugin_compat_t41_real_provider_failure_and_success|crates/vw-agent/src/agent/loop_/core/tool_loop/llm_round_tests.rs::codex_plugin_compat_t41_real_provider_failure_and_success|vw-agent 原路径（真实插件边界）|
|output_tests.rs::progress_helpers_emit_expected_messages|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_emit_expected_messages|agent-loop::host_migration_28_tests|
|output_tests.rs::thinking_progress_marks_later_iterations|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::thinking_progress_marks_later_iterations|agent-loop::host_migration_28_tests|
|output_tests.rs::progress_helpers_skip_when_delta_is_absent_or_empty|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_skip_when_delta_is_absent_or_empty|agent-loop::host_migration_28_tests|
|output_tests.rs::progress_helpers_ignore_closed_receivers|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_ignore_closed_receivers|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_starts_by_clearing_draft|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_starts_by_clearing_draft|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_splits_on_whitespace_boundaries|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_splits_on_whitespace_boundaries|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_skips_when_delta_is_absent|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_skips_when_delta_is_absent|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_sends_only_clear_for_empty_text|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_sends_only_clear_for_empty_text|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_returns_cancelled_when_token_is_cancelled|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_returns_cancelled_when_token_is_cancelled|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_stops_cleanly_when_receiver_is_closed|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_stops_cleanly_when_receiver_is_closed|agent-loop::host_migration_28_tests|
|output_tests.rs::final_response_stops_when_receiver_closes_after_clear|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_stops_when_receiver_closes_after_clear|agent-loop::host_migration_28_tests|
|tool_execution_tests.rs::immediate_failure_preserves_tool_identity_and_marks_failure|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_1.rs::immediate_failure_preserves_tool_identity_and_marks_failure|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::history_tool_call_id_uses_native_fallback_only_when_needed|crates/libs/agent-loop/src/host_migration_28_tests/tool_identity.rs::history_tool_call_id_uses_native_fallback_only_when_needed|agent-loop::host_migration_28_tests|
|tool_execution_tests.rs::structured_progress_skips_when_result_dto_is_absent|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_1.rs::structured_progress_skips_when_result_dto_is_absent|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::excluded_tool_writes_blocked_result_without_execution|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_1.rs::excluded_tool_writes_blocked_result_without_execution|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::mcp_toolset_excluded_tool_is_blocked_by_channel_policy|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_1.rs::mcp_toolset_excluded_tool_is_blocked_by_channel_policy|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::duplicate_text_tool_call_is_reported_without_second_execution|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_2.rs::duplicate_text_tool_call_is_reported_without_second_execution|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_2.rs::native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::successful_tool_execution_emits_progress_and_structured_result|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_2.rs::successful_tool_execution_emits_progress_and_structured_result|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::read_only_tools_execute_through_parallel_batch_and_keep_history_order|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_2.rs::read_only_tools_execute_through_parallel_batch_and_keep_history_order|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::failed_tool_execution_emits_failure_progress_and_error_history|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_3.rs::failed_tool_execution_emits_failure_progress_and_error_history|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::cron_add_delivery_is_injected_before_tool_execution|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_3.rs::cron_add_delivery_is_injected_before_tool_execution|vw-agent 原完整测试路径（真实端口薄入口）|
|tool_execution_tests.rs::native_fallback_tool_ids_do_not_make_duplicate_calls_distinct|crates/libs/agent-loop/src/host_migration_28_tests/tool_execution/scenarios_3.rs::native_fallback_tool_ids_do_not_make_duplicate_calls_distinct|vw-agent 原完整测试路径（真实端口薄入口）|

静态清单：原 55 条 = 领域自注册 24 条 + discovery宿主薄入口 19 条 + execution宿主薄入口 11 条 + 真实插件边界1条。原全部名称唯一映射；逐函数 assert!/assert_eq!/assert_ne! 宏种类、顺序及完整宏正文在仅归一化命名空间/泛型绑定后逐项一致。未删除失败、安全、取消、同批拒绝、授权收缩、事件顺序和真实文件访问断言。

## 未来精确验证入口与预期

- `python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_28_tests -- --test-threads=1`：静态预期 24 条，必须非零。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::loop_::core::tool_loop::discovery_tests:: -- --test-threads=1`：预期 19 条，真实Host实例。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::loop_::core::tool_loop::tool_execution::tool_execution_tests:: -- --test-threads=1`：预期 11 条，包括真实MCP目录探测。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib codex_plugin_compat_t41_real_provider_failure_and_success -- --test-threads=1`：预期1条。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`：逐名核对上表24条纯领域旧路径消失，其余真实边界入口仍在；list命令仅未来执行。不能将0条结果当作通过。

以上命令均未执行，没有退出码/通过数/耗时证据。源码与差异静态核对不代替编译与行为验证；未来如有编译或行为失败须修复后重新获取证据。
