# 任务26：代理执行 agent

代码完成，编译/行为未验证。本轮按公共规则未运行任何 Cargo、单元/实机测试、构建或格式化。仅源码、注册、依赖和差异静态核对；不声称编译耗时改善。

## 生产边界

`Agent`、`AgentBuilder`、单轮和运行编排在本轮之前已由 agent-loop 持有，不能计入本次生产下沉收益。宿主 core.rs 的 from_config 保留真实 observer/runtime/security/memory/tools/provider/skills 配置装配，models 保留真实模型目录读取，run_tool_call_loop_with_skills 保留真实宿主 LoopServices 调用。这些端口禁止用测试伪装成真实权限验收。

`run` 直接调用 agent_loop::run::run_with_agent_factory；删除宿主同名中间转发和空 turn 模块。builder.rs 为实际公共调用保留类型兼容出口。依赖方向 vw-agent → agent-loop，未增加反向依赖或 path/include 宿主代码。

## 测试归属

纯构建、历史、提示、显式模型分类共20项，在 host_migration_26_tests 自注册（8 builder + 12 core）。另自注册工厂错误与方法可用性2项，共22项。DomainHost 只满足类型约束，所有外部端口调用立即 panic；不执行真实循环、不模拟鉴权、不复制算法。

其余20项通过宿主薄入口绑定 NativeCoreSkillHost（3旧 tests、13 turn、4 run），断言与夹具归 agent_loop::agent::test_support；这20项仍编译单态化于宿主，不计为领域独立运行收益。保留真实工具循环入口，避免为测试复制 LoopServices 或另写算法。4项 from_config、2项 public run 和6项技能场景仍在宿主执行（含1项 run_single 真实循环，见逐项映射）。技能6项涉及真实租户权限桥、模型目录恢复、磁盘来源和技能工具，保持原断言；不能用空 host 代替。

## 全文件映射和行数

|来源文件|迁前|迁后|保留职责/目标|
|---|---:|---:|---|
|core.rs|153|153|Agent 类型及上述三个真实宿主端口|
|builder_tests.rs|33|0|8项全部转领域自注册|
|core_tests.rs|69|20|纯12项转领域，保留4项from_config和1项run_single真实循环|
|run_tests.rs|332|39|5项断言/夹具转领域，其中工厂错误自注册，4项真实循环绑定；2项公共run保留|
|turn.rs|1|0|删除空模块|
|turn_tests.rs|564|63|14项断言/夹具转领域，方法可用性自注册，13项真实循环绑定；导入技能场景所用夹具|
|run.rs|83|70|公共 run 配置工厂绑定|
|turn_skills_tests.rs|259|259|6项真实租户/模型/技能工具与文件来源桥接验收原样保留|
|mod.rs|39|35|生产及剩余真实宿主测试挂载|
|tests.rs|761|13|完整断言/夹具转领域，宿主3项真实循环绑定|
|builder.rs|2|2|公共 AgentBuilder 类型出口|

当前实际源快照 2296 → 654 行，净减 1642（71.5%）；与计划2296行基线一致。源生产/文档/挂载文件按上述 builder/core/run/mod/turn 分组，源测试其余六文件分组。

## 全用例映射

|原文件/用例|实际入口/目标|
|---|---|
|builder_tests.rs::build_reports_first_missing_required_dependency|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_reports_missing_provider_after_tools_are_present|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_reports_missing_memory_after_provider_is_present|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_reports_missing_observer_after_memory_is_present|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_fills_defaults_when_optional_fields_are_absent|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_collects_tool_specs_in_tool_order|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_uses_configured_optional_fields|agent_loop::host_migration_26_tests（库自注册）|
|builder_tests.rs::build_uses_configured_memory_loader|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::builder_returns_empty_builder_that_fails_without_required_parts|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::history_returns_empty_slice_for_new_agent|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::history_returns_messages_in_insertion_order|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::clear_history_removes_all_messages|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::clear_history_is_idempotent_for_empty_history|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::trim_history_keeps_system_messages_and_recent_non_system_messages|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::trim_history_keeps_all_non_system_messages_when_only_system_messages_exceed_limit|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::build_system_prompt_includes_tool_protocol_for_prompt_guided_provider|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::build_system_prompt_omits_tool_protocol_for_native_provider|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::classify_model_preserves_explicit_model_over_classification_rules|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::classify_model_preserves_explicit_model_when_no_rule_matches|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::legacy_and_processor_auto_selection_are_consistent|agent_loop::host_migration_26_tests（库自注册）|
|core_tests.rs::from_config_builds_agent_with_none_memory_and_supported_provider|原宿主路径（真实端口）|
|core_tests.rs::from_config_rejects_unknown_provider|原宿主路径（真实端口）|
|core_tests.rs::from_config_accepts_configured_custom_provider|原宿主路径（真实端口）|
|core_tests.rs::from_config_rejects_invalid_programmatic_timezone|原宿主路径（真实端口）|
|core_tests.rs::run_single_delegates_message_to_turn_and_returns_response|原宿主路径（真实端口）|
|run_tests.rs::run_rejects_missing_message_before_starting_agent|原宿主路径（真实端口）|
|run_tests.rs::run_propagates_default_agent_factory_error_for_unknown_provider|原宿主路径（真实端口）|
|run_tests.rs::run_applies_overrides_and_returns_agent_response|agent_loop::agent::test_support::run_tests::run_applies_overrides_and_returns_agent_response；原宿主模块薄绑定|
|run_tests.rs::run_uses_documented_provider_and_model_fallbacks|agent_loop::agent::test_support::run_tests::run_uses_documented_provider_and_model_fallbacks；原宿主模块薄绑定|
|run_tests.rs::run_propagates_agent_factory_error|agent_loop::host_migration_26_tests（库自注册）|
|run_tests.rs::run_propagates_provider_error_without_recording_end_event|agent_loop::agent::test_support::run_tests::run_propagates_provider_error_without_recording_end_event；原宿主模块薄绑定|
|run_tests.rs::run_accepts_empty_message_string|agent_loop::agent::test_support::run_tests::run_accepts_empty_message_string；原宿主模块薄绑定|
|turn_tests.rs::run_single_method_is_available_on_agent_type|agent_loop::host_migration_26_tests（库自注册）|
|turn_tests.rs::turn_uses_ephemeral_system_reminder_and_enriches_user_message|agent_loop::agent::test_support::turn_tests::turn_uses_ephemeral_system_reminder_and_enriches_user_message；原宿主模块薄绑定|
|turn_tests.rs::agent_and_query_engine_use_ephemeral_agent_context_each_turn|agent_loop::agent::test_support::turn_tests::agent_and_query_engine_use_ephemeral_agent_context_each_turn；原宿主模块薄绑定|
|turn_tests.rs::agent_and_query_engine_use_ephemeral_stream_context_each_turn|agent_loop::agent::test_support::turn_tests::agent_and_query_engine_use_ephemeral_stream_context_each_turn；原宿主模块薄绑定|
|turn_tests.rs::turn_timestamp_uses_configured_timezone|agent_loop::agent::test_support::turn_tests::turn_timestamp_uses_configured_timezone；原宿主模块薄绑定|
|turn_tests.rs::turn_skips_memory_context_when_loader_fails|agent_loop::agent::test_support::turn_tests::turn_skips_memory_context_when_loader_fails；原宿主模块薄绑定|
|turn_tests.rs::turn_auto_save_records_user_message_without_blocking_response|agent_loop::agent::test_support::turn_tests::turn_auto_save_records_user_message_without_blocking_response；原宿主模块薄绑定|
|turn_tests.rs::turn_executes_tool_calls_and_returns_final_response|agent_loop::agent::test_support::turn_tests::turn_executes_tool_calls_and_returns_final_response；原宿主模块薄绑定|
|turn_tests.rs::turn_routes_classified_message_to_hint_model|agent_loop::agent::test_support::turn_tests::turn_routes_classified_message_to_hint_model；原宿主模块薄绑定|
|turn_tests.rs::turn_propagates_provider_error|agent_loop::agent::test_support::turn_tests::turn_propagates_provider_error；原宿主模块薄绑定|
|turn_tests.rs::turn_trims_history_after_success|agent_loop::agent::test_support::turn_tests::turn_trims_history_after_success；原宿主模块薄绑定|
|turn_tests.rs::turn_with_stream_sends_progress_and_final_response|agent_loop::agent::test_support::turn_tests::turn_with_stream_sends_progress_and_final_response；原宿主模块薄绑定|
|turn_tests.rs::turn_with_stream_timestamp_uses_configured_timezone|agent_loop::agent::test_support::turn_tests::turn_with_stream_timestamp_uses_configured_timezone；原宿主模块薄绑定|
|turn_tests.rs::turn_with_stream_auto_saves_and_uses_memory_context|agent_loop::agent::test_support::turn_tests::turn_with_stream_auto_saves_and_uses_memory_context；原宿主模块薄绑定|
|turn_skills_tests.rs::codex_skills_core_agent_default_catalog_is_ephemeral_across_two_turns|原宿主路径（真实端口）|
|turn_skills_tests.rs::codex_skills_core_streaming_agent_preserves_skill_roles|原宿主路径（真实端口）|
|turn_skills_tests.rs::codex_skills_core_agent_successful_skill_replays_after_trim_and_clear_drops_it|原宿主路径（真实端口）|
|turn_skills_tests.rs::codex_skills_core_agent_unavailable_default_fails_before_provider|原宿主路径（真实端口）|
|turn_skills_tests.rs::codex_skills_core_agent_instruction_reference_preloads_from_real_source|原宿主路径（真实端口）|
|turn_skills_tests.rs::codex_skills_core_agent_tenant_deny_is_preserved_across_permission_bridge|原宿主路径（真实端口）|
|tests.rs::turn_without_tools_returns_text|agent_loop::agent::test_support::tests::tests::turn_without_tools_returns_text；原宿主模块薄绑定|
|tests.rs::turn_with_native_dispatcher_handles_tool_results_variant|agent_loop::agent::test_support::tests::tests::turn_with_native_dispatcher_handles_tool_results_variant；原宿主模块薄绑定|
|tests.rs::turn_routes_with_hint_when_query_classification_matches|agent_loop::agent::test_support::tests::tests::turn_routes_with_hint_when_query_classification_matches；原宿主模块薄绑定|

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p agent-loop --lib host_migration_26_tests -- --test-threads=1`：预期22项，必须非零。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::agent:: -- --list`：先核实剩余真实名称，纯领域22项应消失；随后按上表保留名逐项使用精确过滤运行，预期33项（20薄绑定+5core+2public run+6skills）。零测试不算通过。

外部挂载清单已检查，任务1未列本来源族额外 path 测试。新增 parking_lot（test-support夹具与依赖同生命周期）及 permission-types（仅dev；ContextHost签名），同步锁文件对应条目。

静态原测试属性数 55，迁后宿主 33、库自注册22，总数 55。所有原函数逐项映射，未删除断言。

接收库新增生产实现0行（只去转发）；测试/夹具行数见下表，已有test_support/core与builder实现不计新增收益。

|接收文件|新增测试/夹具行数|
|---|---:|
|crates/libs/agent-loop/src/host_migration_26_tests.rs|13|
|crates/libs/agent-loop/src/host_migration_26_tests/builder_tests.rs|32|
|crates/libs/agent-loop/src/host_migration_26_tests/host.rs|111|
|crates/libs/agent-loop/src/host_migration_26_tests/core_tests.rs|48|
|crates/libs/agent-loop/src/agent/test_support/run_tests.rs|309|
|crates/libs/agent-loop/src/agent/test_support/turn_tests.rs|548|
|crates/libs/agent-loop/src/agent/test_support/tests.rs|758|
