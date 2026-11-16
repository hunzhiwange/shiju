# 任务27：代理执行 loop_/core

代码完成，编译/行为未验证。全部原用例已映射，领域断言完成下沉；保留真实宿主集成边界。未达到70%减量门槛，逐文件职责及后续可减项供任务59验收，不宣称达标。

## 生产调用与注册

- 新增 `agent-loop::core::turn::{agent_turn, run_tool_call_loop_with_reply_target, run_tool_call_loop_with_non_cli_approval_context}`：完整承接原轮次参数编排、回复目标及非CLI审批嵌套 scope。内部无技能入口仅供两个 scope 函数复用，不新增 trait/config/flag。
- `TOOL_LOOP_REPLY_TARGET` 唯一状态迁至 core::turn，原宿主路径再导出同一 LocalKey；审批 LocalKey 原本就在 agent-loop::approval，继续复用。宿主 services 两个回调仍读取相同状态。原三个宿主函数签名不变，只绑定真实 NativeCoreSkillHost 和 LoopServices；实际调用方向 vw-agent → agent-loop，无反向依赖。
- 历史、LLM轮次、工具循环、工具执行算法在本轮之前已经下沉，不算本轮生产收益。此次真实生产迁移为轮次三函数和回复目标作用域。
- core/mod.rs 注册生产 turn；领域 lib.rs 已注册 host_migration_27_tests，此模块自己注册 tool_loop/turn/collaboration 与专用 host/services 夹具。未 path/include 宿主源码。既有 permission-types dev依赖由任务26已提交，其余依赖均已存在，无本任务 Cargo 清单补丁。

## 领域夹具边界

测试复用真实 core::llm_round、parsing、core::tool_execution、execution、tool-context::executor 和 tool-services::scheduler；EchoTool/ContextCaptureTool 为原工具夹具，未复制调度、重试或历史算法。有限目录仅暴露注册规格；工具执行仍通过真实输入校验、权限流水线、call、结果映射与历史写回。测试 ContextHost 仅允许这两个无外部副作用工具跳过交互式审批，不模拟真实宿主安全结论；未使用的权限、插件、检查点及目录I/O端口明确 panic/bail。普通测试没有子任务和插件，外部事件端口返回空集合；协作负例改接 session-engine 真实协调器，并用 panic 保证预算/取消错误前不等待或消费事件。所有根会话使用原 UUID 唯一键。租户事件投递正例仍通过原生宿主鉴权适配运行。

## 全文件映射（splitlines）

|来源|修改前|修改后|归属/保留职责|
|---|---:|---:|---|
|constants.rs|2|2|DEFAULT_MAX_ITERATIONS/DEFAULT_MAX_HISTORY_MESSAGES/AUTOSAVE_MIN_MESSAGE_CHARS 的内部使用者兼容出口（全部2行）。|
|errors.rs|2|2|ToolLoopCancelled/is_tool_loop_cancelled/is_tool_iteration_limit_error 的错误识别兼容出口（全部2行）。|
|history.rs|12|6|autosave_memory_key/build_native_assistant_history/tools_to_openai_format 的真实调用出口；build_native_assistant_history_from_parsed_calls 为 llm_round::services 提供具体 CallView 函数指针绑定。|
|history_tests.rs|169|0|删除；整组10例和 Testcov0084Tool 的 name/description/parameters_schema/execute/spec 迁入领域。|
|mod.rs|66|51|模块注册和真实使用者导出；TOOL_LOOP_REPLY_TARGET 改为领域同一 LocalKey 的再导出，无第二份状态。|
|skills.rs|24|24|NativeCoreSkillHost 的 current_tenant_scope/load_skills_with_config/get_model/explicitly_referenced_skill_names/user_explicitly_requests_skill_use/user_invocable_skill_names/accessible_skills/description_for_skills/restore/save 全部为真实租户、模型、技能可见性和检查点I/O端口；CoreSkillSession 是具体别名。|
|timeouts.rs|2|2|effective_message_timeout_secs/message_timeout_budget_secs 的调用者兼容出口（全部2行）。|
|tool_loop.rs|140|132|run_tool_call_loop 和 run_tool_call_loop_with_skills 保持宿主公开签名；services 仅绑定真实目录刷新、OCR、LLM、工具执行、插件 stop 与全局协调器。移除不再使用的 collaboration 测试挂载和常量/错误导入。|
|tool_loop_tests.rs|663|134|只保留真实宿主集成/出口契约及其最小provider夹具，逐例见下。|
|turn.rs|353|112|agent_turn/run_tool_call_loop_with_reply_target/run_tool_call_loop_with_non_cli_approval_context 仅保持旧签名并注入 NativeCoreSkillHost 与 services；所有参数默认值和作用域编排均调用新领域入口。|
|turn_tests.rs|418|158|只保留真实宿主集成/出口契约及其最小provider夹具，逐例见下。|

宿主11文件共 1851 → 623，净减 1228（66.34%）。其中物理测试文件 1250 → 292；其余生产/装配/文档文件 601 → 331。领域新增测试/夹具 1384 行；新领域生产 turn.rs 434 行（包含搬迁文档、泛型约束及内部调用包装，不能全部当算法），core/mod.rs 注册2行。生产实际动作仍是原三函数、同一 task-local，无行为扩充。

## 全用例映射

|原测试文件::函数|目标（同名函数）/保留理由|
|---|---|
|history_tests.rs::autosave_memory_key_uses_prefix_and_unique_suffix|agent-loop/src/host_migration_27_tests.rs::autosave_memory_key_uses_prefix_and_unique_suffix|
|history_tests.rs::autosave_memory_key_allows_empty_prefix|agent-loop/src/host_migration_27_tests.rs::autosave_memory_key_allows_empty_prefix|
|history_tests.rs::tools_to_openai_format_maps_tool_specs|agent-loop/src/host_migration_27_tests.rs::tools_to_openai_format_maps_tool_specs|
|history_tests.rs::tools_to_openai_format_returns_empty_for_empty_registry|agent-loop/src/host_migration_27_tests.rs::tools_to_openai_format_returns_empty_for_empty_registry|
|history_tests.rs::native_assistant_history_trims_content_and_keeps_tool_arguments|agent-loop/src/host_migration_27_tests.rs::native_assistant_history_trims_content_and_keeps_tool_arguments|
|history_tests.rs::native_assistant_history_uses_null_for_blank_content_without_reasoning|agent-loop/src/host_migration_27_tests.rs::native_assistant_history_uses_null_for_blank_content_without_reasoning|
|history_tests.rs::parsed_tool_calls_build_native_assistant_history|agent-loop/src/host_migration_27_tests.rs::parsed_tool_calls_build_native_assistant_history|
|history_tests.rs::parsed_tool_calls_build_history_with_empty_calls|agent-loop/src/host_migration_27_tests.rs::parsed_tool_calls_build_history_with_empty_calls|
|history_tests.rs::parsed_tool_calls_serialize_arguments_and_blank_content|agent-loop/src/host_migration_27_tests.rs::parsed_tool_calls_serialize_arguments_and_blank_content|
|history_tests.rs::parsed_tool_calls_without_id_return_none|agent-loop/src/host_migration_27_tests.rs::parsed_tool_calls_without_id_return_none|
|tool_loop_tests.rs::completion_detection_is_reexported_for_tool_loop|原位：宿主 detection 兼容导出契约；算法断言由领域 detection_tests 持有。|
|tool_loop_tests.rs::run_tool_call_loop_returns_final_text_and_updates_history_when_max_iterations_is_zero|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_returns_final_text_and_updates_history_when_max_iterations_is_zero|
|tool_loop_tests.rs::run_tool_call_loop_exits_before_provider_call_when_cancelled|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_exits_before_provider_call_when_cancelled|
|tool_loop_tests.rs::run_tool_call_loop_reports_exhausted_after_last_tool_iteration|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_reports_exhausted_after_last_tool_iteration|
|tool_loop_tests.rs::run_tool_call_loop_retries_empty_final_response_until_text_arrives|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_retries_empty_final_response_until_text_arrives|
|tool_loop_tests.rs::run_tool_call_loop_reports_exhausted_when_all_final_responses_are_empty|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_reports_exhausted_when_all_final_responses_are_empty|
|tool_loop_tests.rs::run_tool_call_loop_omits_excluded_tools_from_provider_request|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_omits_excluded_tools_from_provider_request|
|tool_loop_tests.rs::run_tool_call_loop_retries_once_when_completion_claim_lacks_tool_call|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_retries_once_when_completion_claim_lacks_tool_call|
|tool_loop_tests.rs::run_tool_call_loop_retries_once_when_tool_payload_cannot_be_parsed|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_retries_once_when_tool_payload_cannot_be_parsed|
|tool_loop_tests.rs::run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call|agent-loop/src/host_migration_27_tests/tool_loop/tests.rs::run_tool_call_loop_errors_when_completion_claim_repeats_without_tool_call|
|tool_loop_tests.rs::agcollab_channel_integrates_child_results|原位：真实 NativeContext 租户检查 → session-engine 协调器事件投递 → channel最终输出/流式隐藏提前回答。|
|tool_loop_tests.rs::agcollab_channel_cancellation_and_budget_reject_unfinished_children|agent-loop/src/host_migration_27_tests/collaboration.rs::agcollab_channel_cancellation_and_budget_reject_unfinished_children|
|turn_tests.rs::reply_target_task_local_defaults_to_unset_outside_scope|agent-loop/src/host_migration_27_tests/turn.rs::reply_target_task_local_defaults_to_unset_outside_scope|
|turn_tests.rs::agent_turn_delegates_to_tool_loop_with_channel_defaults|agent-loop/src/host_migration_27_tests/turn.rs::agent_turn_delegates_to_tool_loop_with_channel_defaults|
|turn_tests.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_approval_and_reply_target|agent-loop/src/host_migration_27_tests/turn.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_approval_and_reply_target|
|turn_tests.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_none_when_context_is_absent|agent-loop/src/host_migration_27_tests/turn.rs::run_tool_call_loop_with_non_cli_approval_context_scopes_none_when_context_is_absent|
|turn_tests.rs::codex_plugin_compat_t44_actual_turn_continues_after_stop_block|原位：真实 plugins::t44_tests::runner 子进程和事件文件，验证Stop阻止后的继续、失败/Interrupt取消接线。|
|turn_tests.rs::codex_plugin_compat_t44_actual_turn_failure_and_cancellation|原位：真实 plugins::t44_tests::runner 子进程和事件文件，验证Stop阻止后的继续、失败/Interrupt取消接线。|

## 减量门槛与剩余职责

本组不足70%：constants/errors/timeouts 仅为现有调用者稳定出口，history.rs 仅具体类型绑定，skills.rs 仅真实租户/检查点/模型I/O，mod.rs 仅注册和再导出，tool_loop.rs 仅真实服务装配及稳定外部签名，turn.rs 仅相同端口的三个签名包装；全部测试保留项均为真实插件进程、租户投递或出口契约。不存在继续留在宿主的重试/预算/过滤/历史/作用域领域断言。后续可减项是收口时审查兼容出口是否能让所有消费者直接引用领域库、评估是否统一原生包装签名；这需跨其它任务调用者，当前不越界也不删除真实宿主覆盖凑数。逐文件精确行数如上。

scope.json 任务27仅拥有上述11个来源，没有追加外部挂载；task1 external-mounts 中 loop_history.rs 属另一文件族，不改其注册。旧 history_tests 文件及挂载已删除；tool_loop_tests/turn_tests 原领域函数已删除，只留映射表中的真实边界。原 collaboration 测试shim不再挂载（shim文件属于相邻任务来源，本任务只清理直接父挂载）。

## 静态核对与未来验证

源码集合核对：原 28 例 = 新领域 24 例 + 保留宿主 4 例，名称无缺失、无重复。所有原断言调用数量保留；只变更导入/端口绑定，未削弱预期。核对生产工厂、注册、依赖及局部 diff；没有执行测试代码。

未来命令：
- `python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_27_tests -- --test-threads=1`，静态预期 24 例，必须非零。
- 保留宿主每例用 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <下列完整模块路径> -- --test-threads=1`；每个过滤预期1例，先 `--list` 确认真实编译路径，不把零例当通过。
  - `agent::loop_::core::tool_loop::tool_loop_tests::completion_detection_is_reexported_for_tool_loop`
  - `agent::loop_::core::tool_loop::tool_loop_tests::agcollab_channel_integrates_child_results`
  - `agent::loop_::core::turn::turn_tests::codex_plugin_compat_t44_actual_turn_continues_after_stop_block`
  - `agent::loop_::core::turn::turn_tests::codex_plugin_compat_t44_actual_turn_failure_and_cancellation`
- 未来宿主 list 中以上24条迁出路径应消失，领域 list 应出现同名用例。
- 按用户明确约束，本轮未运行任何 Cargo（含metadata/list/fmt/check）、构建、单元或实机测试。所有编译和行为结果未知，无构建耗时收益结论。
