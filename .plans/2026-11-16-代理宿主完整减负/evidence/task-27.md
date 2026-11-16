# 任务27：代理执行 loop_/core

历史测试迁移及其静态接线完成；任务整体未完成，编译/行为未验证。减量不足70%，其余领域断言仍需拆分。

## 实际迁移

history_tests 全组10例、Testcov0084Tool 的 name/description/parameters_schema/execute/spec 完整迁至 agent-loop::host_migration_27_tests，保留名称与断言。仅替换类型导入并增加调用现有泛型算法的 CallView 绑定，未重写算法。领域 lib.rs 自注册；删除宿主旧文件及挂载、无用 Tool/ToolCall 导入与测试专用 parsed history 再导出。现有依赖已覆盖，无 Cargo.toml/Cargo.lock 补丁。

生产历史算法在本轮前已经下沉，本次新增生产迁移收益为0。真实生产方向保持 vw-agent → agent-loop；llm_round::services.history_parsed → 宿主类型绑定 → agent-loop::core::history。没有反向依赖、path/include 宿主源码或复制 App 状态。

## 全文件映射（splitlines）

|来源|修改前|修改后|职责/目标|
|---|---:|---:|---|
|constants.rs|2|2|兼容出口，DEFAULT_MAX_ITERATIONS/DEFAULT_MAX_HISTORY_MESSAGES/AUTOSAVE_MIN_MESSAGE_CHARS 仍供宿主调用；没有领域算法。|
|errors.rs|2|2|ToolLoopCancelled/is_tool_loop_cancelled/is_tool_iteration_limit_error 的实际错误识别出口。|
|history.rs|12|6|autosave_memory_key/build_native_assistant_history/tools_to_openai_format 的使用者出口；build_native_assistant_history_from_parsed_calls 为 llm_round::services 提供具体函数指针，保留 CallView 类型绑定。|
|history_tests.rs|169|0|整组领域测试迁至 agent-loop/src/host_migration_27_tests.rs|
|mod.rs|66|64|宿主入口注册、TOOL_LOOP_REPLY_TARGET 真实任务局部变量及内部调用者出口。|
|skills.rs|24|24|NativeCoreSkillHost 实现全部真实端口：当前租户、技能加载、模型目录、技能可见性、检查点读写；CoreSkillSession 是宿主具体类型别名。|
|timeouts.rs|2|2|effective_message_timeout_secs/message_timeout_budget_secs 的兼容出口。|
|tool_loop.rs|140|138|run_tool_call_loop/run_tool_call_loop_with_skills 是实际宿主调用入口；services 连接目录刷新、OCR、LLM、工具执行、插件 stop 和全局子代理协调器。|
|tool_loop_tests.rs|663|663|保留真实宿主集成测试，逐例见下表。|
|turn.rs|353|353|agent_turn 填充默认参数；run_tool_call_loop_with_reply_target 和 run_tool_call_loop_with_non_cli_approval_context 建立实际宿主任务局部作用域。|
|turn_tests.rs|418|418|保留真实宿主集成测试，逐例见下表。|

宿主本组 1851 → 1672，净减 179（9.67%）。测试文件 1250 → 1081；生产/装配/文档文件 601 → 591（并非真实生产算法行数）。领域新增测试/夹具 186 行、注册3行，生产算法0行。

## 全用例映射

|原测试文件::函数|实际归属及保留原因|
|---|---|
|history_tests.rs::autosave_memory_key_uses_prefix_and_unique_suffix|agent_loop::host_migration_27_tests::autosave_memory_key_uses_prefix_and_unique_suffix|
|history_tests.rs::autosave_memory_key_allows_empty_prefix|agent_loop::host_migration_27_tests::autosave_memory_key_allows_empty_prefix|
|history_tests.rs::tools_to_openai_format_maps_tool_specs|agent_loop::host_migration_27_tests::tools_to_openai_format_maps_tool_specs|
|history_tests.rs::tools_to_openai_format_returns_empty_for_empty_registry|agent_loop::host_migration_27_tests::tools_to_openai_format_returns_empty_for_empty_registry|
|history_tests.rs::native_assistant_history_trims_content_and_keeps_tool_arguments|agent_loop::host_migration_27_tests::native_assistant_history_trims_content_and_keeps_tool_arguments|
|history_tests.rs::native_assistant_history_uses_null_for_blank_content_without_reasoning|agent_loop::host_migration_27_tests::native_assistant_history_uses_null_for_blank_content_without_reasoning|
|history_tests.rs::parsed_tool_calls_build_native_assistant_history|agent_loop::host_migration_27_tests::parsed_tool_calls_build_native_assistant_history|
|history_tests.rs::parsed_tool_calls_build_history_with_empty_calls|agent_loop::host_migration_27_tests::parsed_tool_calls_build_history_with_empty_calls|
|history_tests.rs::parsed_tool_calls_serialize_arguments_and_blank_content|agent_loop::host_migration_27_tests::parsed_tool_calls_serialize_arguments_and_blank_content|
|history_tests.rs::parsed_tool_calls_without_id_return_none|agent_loop::host_migration_27_tests::parsed_tool_calls_without_id_return_none|
|tool_loop_tests.rs::completion_detection_is_reexported_for_tool_loop|原位保留：宿主 detection 再导出契约；算法测试已由 agent-loop::core::detection_tests 持有。|
|tool_loop_tests.rs::run_tool_call_loop_returns_final_text_and_updates_history_when_max_iterations_is_zero|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_exits_before_provider_call_when_cancelled|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_reports_exhausted_after_last_tool_iteration|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_retries_empty_final_response_until_text_arrives|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_reports_exhausted_when_all_final_responses_are_empty|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_omits_excluded_tools_from_provider_request|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_retries_once_when_completion_claim_lacks_tool_call|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_retries_once_when_tool_payload_cannot_be_parsed|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call|原位保留：经实际宿主 services 的 provider 请求/历史写入/工具执行接线；领域断言尚可继续拆分（见缺口）。|
|tool_loop_tests.rs::agcollab_channel_integrates_child_results|原位保留：全局真实子代理协调器与通道模型事件、流式可见性及取消/预算接线。|
|tool_loop_tests.rs::agcollab_channel_cancellation_and_budget_reject_unfinished_children|原位保留：全局真实子代理协调器与通道模型事件、流式可见性及取消/预算接线。|
|turn_tests.rs::reply_target_task_local_defaults_to_unset_outside_scope|原位保留：宿主真实 TOOL_LOOP_REPLY_TARGET/TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT 作用域与退出后清理。|
|turn_tests.rs::agent_turn_delegates_to_tool_loop_with_channel_defaults|原位保留：实际 agent_turn 参数默认值到原生 LoopServices 的接线。|
|turn_tests.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_approval_and_reply_target|原位保留：宿主真实 TOOL_LOOP_REPLY_TARGET/TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT 作用域与退出后清理。|
|turn_tests.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_none_when_context_is_absent|原位保留：宿主真实 TOOL_LOOP_REPLY_TARGET/TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT 作用域与退出后清理。|
|turn_tests.rs::codex_plugin_compat_t44_actual_turn_continues_after_stop_block|原位保留：plugins::t44_tests::runner 实际插件进程、文件事件、stop/Interrupt 宿主接线。|
|turn_tests.rs::codex_plugin_compat_t44_actual_turn_failure_and_cancellation|原位保留：plugins::t44_tests::runner 实际插件进程、文件事件、stop/Interrupt 宿主接线。|

## 减量缺口与继续可迁项

本组未达到70%净减；tool_loop_tests.rs 中取消、迭代耗尽、空响应重试、排除工具、无工具完成声明及解析重试仍混合领域断言。继续迁移需要领域 LoopServices 的最小 ContextHost/CoreSkillHost 与真实工具执行夹具，不能直接搬 NativeCoreSkillHost（其租户/插件/检查点归宿主），也不能以空执行器替代工具结果断言。本轮保留所有原断言，未以删除覆盖换行数。turn_tests 中三个审批/task-local 场景及两个真实插件场景为宿主边界；默认轮次场景可以进一步拆出参数编排领域断言。工具循环和轮次生产函数均是宿主装配，没有再次统计为生产迁移。以上为本任务未完成项，不能据本证据声称完整领域减负已达标。

scope.json 任务27只有上述11个来源，没有追加外部挂载；任务1 external-mounts 中 loop_history.rs 属其它文件族，本轮未改其注册。

## 未来验证（本次均未运行）

- `python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_27_tests -- --test-threads=1`，预期10例，必须非零。
- 宿主按上表保留函数逐个执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <函数名> -- --test-threads=1`；每个过滤预期至少1例，先由 list 核实完整路径，不能把零例作为通过。
- 对迁出的10个原宿主完整路径做未来 `--list` 核对，应全部消失；新领域10例应出现。
- 未运行 Cargo（包括metadata/list/fmt/check）、单元测试、构建及实机；仅做源码注册、导入、依赖方向和差异静态核对。无编译耗时或运行通过结论。
