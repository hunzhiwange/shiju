# 任务28静态迁移证据

当前为部分实现，任务未完成；未达到70%减量门槛，未删除任务卡。未运行任何 Cargo、构建、单元测试或实机检查。仅完成源码、依赖方向和差异静态检查；不声称编译或行为通过。

## 已闭合的迁移

agent-loop 自注册 host_migration_28_tests：单轮模型解析、OCR、能力拒绝、取消、观测与输入钩子；全部输出场景；工具历史ID规则。测试名称与断言保留。插件 emit_model 是外部端口空实现，真实插件事件测试继续使用宿主插件 fixture。领域依赖仅现有 agent-*、model-types、tool-types 与 plugin-hooks，无 vw-agent 反向依赖、include 或宿主 path 挂载。

生产规则此前已经位于 agent-loop，本轮生产迁出收益为0；删除的是宿主测试专用转发。真实生产调用仍为宿主 LoopServices → llm_round/tool_execution 适配 → agent_loop::core 领域入口。

## 文件与行数（splitlines）

|来源|原行数|现行数|职责/去向|
|---|---:|---:|---|
|collaboration.rs|5|5|integrate 为 tool_loop_tests 的真实子代理协调器端口，5行保留。|
|detection.rs|2|2|完成检测公共测试兼容 re-export，2行保留。|
|discovery_tests.rs|809|809|DiscoveryFileTool spec/name/description/parameters_schema/is_read_only/check_permissions/execute、discovery_tools、DiscoveryProvider、PendingCatalogTool 均为真实宿主 ToolDiscoverySession、BatchTool、实时目录及 Agent builder 请求夹具；尚需拆出领域断言，不能以该耦合理由整体豁免。|
|llm_round.rs|127|110|真实 SessionImageTextExtractor、plugin hook_model::emit/token_usage 与解析/历史端口组合；删除2个纯测试转发。|
|llm_round_tests.rs|624|192|仅保留 codex_plugin_compat_t41 真实插件 fixture 的6事件顺序、失败/取消/成功与敏感信息边界及其脚本供应商/观察者夹具。|
|output.rs|25|0|删除全部测试专用 wire 转发；领域测试直接绑定真实 ProgressWire。|
|output_tests.rs|132|0|整组迁 agent-loop 自注册 output。|
|tool_execution.rs|125|121|保留执行器/调度器、cron投递、native search序列化真实端口组合；删除 history_tool_call_id 测试转发。|
|tool_execution_tests.rs|725|715|MCP 目录构建/配置子进程应留真实边界；其余执行历史、进度及去重断言尚待进一步拆分。|

宿主来源合计 2574 → 1954，净减 620（24.1%）；父入口另减2行。领域新增生产0行、测试及夹具/适配 834 行。宿主生产适配原 252 行；测试/测试转发其余；现生产适配 231 行（按文件统计，含 cfg(test) import/辅助，非纯业务量）。

## 全用例映射

|原文件 / 用例|当前位置|
|---|---|
|discovery_tests.rs::codex_discovery_core_agent_discovers_and_calls_only_on_next_request|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_agent_discovers_and_calls_only_on_next_request|
|discovery_tests.rs::codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_text_protocol_exposes_only_current_specs_without_persisting_them|
|discovery_tests.rs::codex_discovery_core_same_round_search_does_not_allow_deferred_call|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_same_round_search_does_not_allow_deferred_call|
|discovery_tests.rs::codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_failed_and_repeated_searches_keep_visibility_consistent|
|discovery_tests.rs::codex_discovery_core_excluded_tool_cannot_be_searched_or_called|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_excluded_tool_cannot_be_searched_or_called|
|discovery_tests.rs::codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_denied_search_keeps_authorized_mcp_directly_visible|
|discovery_tests.rs::codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_batch_rejects_same_round_and_excluded_inner_calls|
|discovery_tests.rs::codex_discovery_core_empty_registry_provides_no_tool_protocol|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_empty_registry_provides_no_tool_protocol|
|discovery_tests.rs::codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::codex_discovery_core_loaded_batch_calls_preserve_inner_permission_checks|
|discovery_tests.rs::dynamic_tools_core_replacement_visible_next_request|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::dynamic_tools_core_replacement_visible_next_request|
|discovery_tests.rs::dynamic_tools_core_replacement_preserves_same_batch_rejection|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::dynamic_tools_core_replacement_preserves_same_batch_rejection|
|discovery_tests.rs::dynamic_tools_core_replacement_intersects_loaded_authorization|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::dynamic_tools_core_replacement_intersects_loaded_authorization|
|discovery_tests.rs::dynamic_tools_core_replacement_cannot_expand_child_authority|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::dynamic_tools_core_replacement_cannot_expand_child_authority|
|discovery_tests.rs::native_tool_search_core_loop_core_calls_discovered_tool_next_request|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_core_calls_discovered_tool_next_request|
|discovery_tests.rs::native_tool_search_core_loop_same_batch_remains_denied|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_same_batch_remains_denied|
|discovery_tests.rs::native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_empty_and_excluded_results_do_not_load_tools|
|discovery_tests.rs::native_tool_search_core_loop_rejects_unadvertised_and_invalid_search|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_rejects_unadvertised_and_invalid_search|
|discovery_tests.rs::native_tool_search_core_loop_ordinary_discovery_still_executes|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_ordinary_discovery_still_executes|
|discovery_tests.rs::native_tool_search_core_loop_replayed_search_is_not_completed_again|crates/vw-agent/src/agent/loop_/core/tool_loop/discovery_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_search_core_loop_replayed_search_is_not_completed_again|
|llm_round_tests.rs::fallback_tool_call_ids_are_stable_per_iteration|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::fallback_tool_call_ids_are_stable_per_iteration|
|llm_round_tests.rs::assistant_history_handles_plain_text_and_missing_native_ids|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::assistant_history_handles_plain_text_and_missing_native_ids|
|llm_round_tests.rs::run_llm_round_plain_success_records_usage_and_text|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_plain_success_records_usage_and_text|
|llm_round_tests.rs::run_llm_round_accepts_image_markers_for_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_accepts_image_markers_for_vision_provider|
|llm_round_tests.rs::run_llm_round_text_fallback_assigns_ids_and_native_history|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_text_fallback_assigns_ids_and_native_history|
|llm_round_tests.rs::run_llm_round_detects_malformed_tool_call_parse_issue|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_detects_malformed_tool_call_parse_issue|
|llm_round_tests.rs::run_llm_round_native_tool_calls_skip_text_fallback|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_native_tool_calls_skip_text_fallback|
|llm_round_tests.rs::run_llm_round_rejects_image_markers_for_non_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_rejects_image_markers_for_non_vision_provider|
|llm_round_tests.rs::run_llm_round_uses_ocr_before_non_vision_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_uses_ocr_before_non_vision_provider|
|llm_round_tests.rs::run_llm_round_fires_llm_input_hook|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_fires_llm_input_hook|
|llm_round_tests.rs::run_llm_round_cancellation_wins_over_slow_provider|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_cancellation_wins_over_slow_provider|
|llm_round_tests.rs::run_llm_round_provider_error_records_failed_response|crates/libs/agent-loop/src/host_migration_28_tests/llm_round/tests.rs::run_llm_round_provider_error_records_failed_response|
|llm_round_tests.rs::codex_plugin_compat_t41_real_provider_failure_and_success|crates/vw-agent/src/agent/loop_/core/tool_loop/llm_round_tests.rs（保留原宿主边界；未完成领域拆分）::codex_plugin_compat_t41_real_provider_failure_and_success|
|output_tests.rs::progress_helpers_emit_expected_messages|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_emit_expected_messages|
|output_tests.rs::thinking_progress_marks_later_iterations|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::thinking_progress_marks_later_iterations|
|output_tests.rs::progress_helpers_skip_when_delta_is_absent_or_empty|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_skip_when_delta_is_absent_or_empty|
|output_tests.rs::progress_helpers_ignore_closed_receivers|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::progress_helpers_ignore_closed_receivers|
|output_tests.rs::final_response_starts_by_clearing_draft|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_starts_by_clearing_draft|
|output_tests.rs::final_response_splits_on_whitespace_boundaries|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_splits_on_whitespace_boundaries|
|output_tests.rs::final_response_skips_when_delta_is_absent|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_skips_when_delta_is_absent|
|output_tests.rs::final_response_sends_only_clear_for_empty_text|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_sends_only_clear_for_empty_text|
|output_tests.rs::final_response_returns_cancelled_when_token_is_cancelled|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_returns_cancelled_when_token_is_cancelled|
|output_tests.rs::final_response_stops_cleanly_when_receiver_is_closed|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_stops_cleanly_when_receiver_is_closed|
|output_tests.rs::final_response_stops_when_receiver_closes_after_clear|crates/libs/agent-loop/src/host_migration_28_tests/output.rs::final_response_stops_when_receiver_closes_after_clear|
|tool_execution_tests.rs::immediate_failure_preserves_tool_identity_and_marks_failure|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::immediate_failure_preserves_tool_identity_and_marks_failure|
|tool_execution_tests.rs::history_tool_call_id_uses_native_fallback_only_when_needed|crates/libs/agent-loop/src/host_migration_28_tests/tool_identity.rs::history_tool_call_id_uses_native_fallback_only_when_needed|
|tool_execution_tests.rs::structured_progress_skips_when_result_dto_is_absent|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::structured_progress_skips_when_result_dto_is_absent|
|tool_execution_tests.rs::excluded_tool_writes_blocked_result_without_execution|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::excluded_tool_writes_blocked_result_without_execution|
|tool_execution_tests.rs::mcp_toolset_excluded_tool_is_blocked_by_channel_policy|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::mcp_toolset_excluded_tool_is_blocked_by_channel_policy|
|tool_execution_tests.rs::duplicate_text_tool_call_is_reported_without_second_execution|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::duplicate_text_tool_call_is_reported_without_second_execution|
|tool_execution_tests.rs::native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::native_tool_ids_keep_same_arguments_distinct_and_write_tool_messages|
|tool_execution_tests.rs::successful_tool_execution_emits_progress_and_structured_result|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::successful_tool_execution_emits_progress_and_structured_result|
|tool_execution_tests.rs::read_only_tools_execute_through_parallel_batch_and_keep_history_order|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::read_only_tools_execute_through_parallel_batch_and_keep_history_order|
|tool_execution_tests.rs::failed_tool_execution_emits_failure_progress_and_error_history|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::failed_tool_execution_emits_failure_progress_and_error_history|
|tool_execution_tests.rs::cron_add_delivery_is_injected_before_tool_execution|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::cron_add_delivery_is_injected_before_tool_execution|
|tool_execution_tests.rs::native_fallback_tool_ids_do_not_make_duplicate_calls_distinct|crates/vw-agent/src/agent/loop_/core/tool_loop/tool_execution_tests.rs（保留原宿主边界；未完成领域拆分）::native_fallback_tool_ids_do_not_make_duplicate_calls_distinct|

## 具体剩余工作与边界缺口

本组没有可直接复用的真实领域 ContextHost：宿主 AgentContextHost 的 ToolDiscoverySession/ToolCatalogSnapshot/ToolCatalog、目录授权、工具执行和插件快照生命周期仍在宿主组合。不能把它复制到库或以放行权限 mock 替代真实权限验收。后续需在这些真实端口支持下拆出 discovery 请求可见性/同批拒绝/native replay 与 tool_execution 的去重、顺序、进度断言；MCP tools_for_context 配置探测和插件t41事件验收留薄宿主测试。当前并未完成这些步骤，因此不将整族计为完成，不用已移动用例掩盖缺口。

继续可迁：tool_execution immediate_failure、无DTO进度、excluded、duplicate、native ID、success/failure进度、parallel顺序、cron注入与fallback去重；discovery 非真实目录/MCP资源生命周期的请求断言。需原任务继续完成，不能让收口仅凭任务卡删除判为成功。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_28_tests -- --test-threads=1`，静态预期 24 条，必须非零。

宿主逐条使用上表保留用例完整路径过滤；迁出场景需通过宿主真实 list 确认旧名消失，不以零测试结果记通过。宿主插件测试未来过滤 `codex_plugin_compat_t41_real_provider_failure_and_success` 预期1条；discovery / tool_execution 分别按原模块过滤并核对上表剩余数量。全部编译/行为检查本次未运行（遵守 plan.md 的用户覆盖）。
