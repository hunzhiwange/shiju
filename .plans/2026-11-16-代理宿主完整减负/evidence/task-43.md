# 任务43迁移证据

代码及接线完成；所有 Cargo、构建、编译、行为、单元和实机检查均未运行（用户明确禁止）。静态核对不等同测试通过。

## 生产与依赖方向

宿主 channels::prompt 重导出 channel_runtime::prompt；网关/通道的既有 build_entrypoint_protocol_messages、build_system_prompt[_with_mode]、build_runtime_tool_visibility_prompt 调用因此进入运行库。工作空间指令解析 build_workspace_protocol_messages 保留宿主 session::prompt 端口。运行库使用已有 session-engine、agent-loop、channel-support 依赖，不反向依赖 vw-agent。

实际迁出生产函数：build_entrypoint_protocol_messages、build_runtime_tool_visibility_prompt、build_system_prompt、build_system_prompt_with_mode；block_on_future 迁入 channel-support::blocking，session processor 与同步路由目录调用通过宿主唯一重导出使用。其余原有 channel-support 函数仅迁测试，不计新生产收益。

channel-runtime 新增生产 chrono-tz 声明（原 dev 已有，锁文件依赖条目已包含）；channel-support 原生 tokio 增加 rt-multi-thread/time/net，以保持 block_in_place 与 enable_all 行为。没有引入新包或改变权限。

## 全来源映射与保留职责

同路径 splitlines：2763 → 1467 行，净减 1296 行（46.9%）。未达70%，不得视为减量门槛通过。

- `crates/vw-agent/src/channels/mod.rs`：369 → 334 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/mod_tests.rs`：53 → 35 行。保留 root_module_reexports_core_channel_types 与 native_channel_reexports_are_constructible，证明宿主公开入口确实连到通道实现；阻塞桥两用例迁入 support。

- `crates/vw-agent/src/channels/nextcloud_talk/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/nostr/mod.rs`：3 → 3 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/outbound_delivery.rs`：2 → 2 行。稳定公开兼容 re-export（平台/特性门控由父模块负责），无新增生产算法；保留现有调用路径，后续仅在调用者统一改用领域 crate 后可删。

- `crates/vw-agent/src/channels/outbound_format.rs`：2 → 2 行。稳定公开兼容 re-export（平台/特性门控由父模块负责），无新增生产算法；保留现有调用路径，后续仅在调用者统一改用领域 crate 后可删。

- `crates/vw-agent/src/channels/prompt.rs`：349 → 20 行。仅保留 build_workspace_protocol_messages：宿主读取实际工作空间指令；其余组装及原文档归运行库。

- `crates/vw-agent/src/channels/prompt_tests.rs`：580 → 0 行。整组迁入 channel-runtime::host_migration_43_tests::prompt。

- `crates/vw-agent/src/channels/qq/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/routing.rs`：284 → 281 行。保留同步目录边界、上下文 defaults/classification/override 参数投影和公共路径；算法已在 support，不能重复计迁移。继续可迁：调用方异步化后消除同步桥和冗余投影。

- `crates/vw-agent/src/channels/routing_tests.rs`：268 → 0 行。整组迁入 channel-support::host_migration_43_tests::routing；目录查询改为 await 真实领域入口，断言保留。

- `crates/vw-agent/src/channels/scope.rs`：27 → 27 行。保留具体会话/租户/项目 task-local、事务存储、标题任务及运行入口适配；策略已归 support。继续可迁：session processor事件转换需 processor Request/StreamEvent 端口所有者统一抽取。

- `crates/vw-agent/src/channels/scope_delivery_tests.rs`：44 → 44 行。真实 process_channel_message 宿主端口验收：命令目录、实例投递及路由归属，依赖宿主 catalog_context；不能用模拟执行器取代。

- `crates/vw-agent/src/channels/session.rs`：213 → 213 行。保留具体会话/租户/项目 task-local、事务存储、标题任务及运行入口适配；策略已归 support。继续可迁：session processor事件转换需 processor Request/StreamEvent 端口所有者统一抽取。

- `crates/vw-agent/src/channels/session_tests.rs`：473 → 410 行。角色转换及五个身份隔离测试归 support；保留真实 processor、租户 task-local、项目数据库注册、会话 reserve/discard 和隔离子进程验收，不能以模拟端口替代。registered_context/channel_message 被宿主命令测试直接调用。继续可迁：分离绑定优先级/无效绑定及项目路由隔离断言，需将真实数据库上下文夹具与会话宿主验收分别注册。

- `crates/vw-agent/src/channels/session_turn.rs`：61 → 61 行。保留具体会话/租户/项目 task-local、事务存储、标题任务及运行入口适配；策略已归 support。继续可迁：session processor事件转换需 processor Request/StreamEvent 端口所有者统一抽取。

- `crates/vw-agent/src/channels/signal/mod.rs`：3 → 3 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/slack/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/telegram/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/traits/mod.rs`：3 → 3 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/wati/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/wechat/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/wecom/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/whatsapp/mod.rs`：10 → 10 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/whatsapp_storage/mod.rs`：3 → 3 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

- `crates/vw-agent/src/channels/whatsapp_web/mod.rs`：2 → 2 行。保留实际宿主装配、功能/平台门控、公开兼容入口和其他任务拥有的模块挂载；仅迁本任务 blocking 定义。后续可由收口检查无调用兼容模块/常量，不能删除其他任务挂载。

## 全函数与测试映射（原名保留）

以下含非 #[test] 的夹具及异步行为函数；保留函数继续由原宿主注册/调用，不冒充新领域测试。

- `crates/vw-agent/src/channels/mod.rs::block_on_future` → `crates/libs/channel-support/src/blocking.rs::block_on_future`

- `crates/vw-agent/src/channels/mod_tests.rs::root_module_reexports_core_channel_types` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/mod_tests.rs::assert_channel` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/mod_tests.rs::native_channel_reexports_are_constructible` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/mod_tests.rs::block_on_future_runs_outside_runtime` → `crates/libs/channel-support/src/host_migration_43_tests/blocking.rs::block_on_future_runs_outside_runtime`

- `crates/vw-agent/src/channels/mod_tests.rs::block_on_future_runs_inside_runtime` → `crates/libs/channel-support/src/host_migration_43_tests/blocking.rs::block_on_future_runs_inside_runtime`

- `crates/vw-agent/src/channels/prompt.rs::build_entrypoint_protocol_messages` → `crates/libs/channel-runtime/src/prompt.rs::build_entrypoint_protocol_messages`

- `crates/vw-agent/src/channels/prompt.rs::build_workspace_protocol_messages` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/prompt.rs::build_runtime_tool_visibility_prompt` → `crates/libs/channel-runtime/src/prompt.rs::build_runtime_tool_visibility_prompt`

- `crates/vw-agent/src/channels/prompt.rs::build_system_prompt` → `crates/libs/channel-runtime/src/prompt.rs::build_system_prompt`

- `crates/vw-agent/src/channels/prompt.rs::build_system_prompt_with_mode` → `crates/libs/channel-runtime/src/prompt.rs::build_system_prompt_with_mode`

- `crates/vw-agent/src/channels/prompt_tests.rs::name` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::name`

- `crates/vw-agent/src/channels/prompt_tests.rs::description` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::description`

- `crates/vw-agent/src/channels/prompt_tests.rs::parameters_schema` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::parameters_schema`

- `crates/vw-agent/src/channels/prompt_tests.rs::execute` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::execute`

- `crates/vw-agent/src/channels/prompt_tests.rs::name` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::name`

- `crates/vw-agent/src/channels/prompt_tests.rs::store` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::store`

- `crates/vw-agent/src/channels/prompt_tests.rs::recall` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::recall`

- `crates/vw-agent/src/channels/prompt_tests.rs::get` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::get`

- `crates/vw-agent/src/channels/prompt_tests.rs::list` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::list`

- `crates/vw-agent/src/channels/prompt_tests.rs::forget` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::forget`

- `crates/vw-agent/src/channels/prompt_tests.rs::count` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::count`

- `crates/vw-agent/src/channels/prompt_tests.rs::health_check` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::health_check`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_entry` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_entry`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_system_prompt_keeps_cli_plain_when_no_context` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_system_prompt_keeps_cli_plain_when_no_context`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_system_prompt_adds_delivery_visibility_and_delivery_context` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_system_prompt_adds_delivery_visibility_and_delivery_context`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_system_prompt_can_allow_explicit_execution_details` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_system_prompt_can_allow_explicit_execution_details`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_system_prompt_uses_visibility_as_root_for_unknown_non_cli_channel` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_system_prompt_uses_visibility_as_root_for_unknown_non_cli_channel`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_system_prompt_uses_delivery_instructions_when_base_is_empty` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_system_prompt_uses_delivery_instructions_when_base_is_empty`

- `crates/vw-agent/src/channels/prompt_tests.rs::entrypoint_context_all_external_paths_share_agents_skill_contract` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::entrypoint_context_all_external_paths_share_agents_skill_contract`

- `crates/vw-agent/src/channels/prompt_tests.rs::selection_trace_redacts_prompt` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::selection_trace_redacts_prompt`

- `crates/vw-agent/src/channels/prompt_tests.rs::channel_auto_uses_selected_model` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::channel_auto_uses_selected_model`

- `crates/vw-agent/src/channels/prompt_tests.rs::runtime_tool_visibility_sorts_filters_and_uses_native_protocol_text` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::runtime_tool_visibility_sorts_filters_and_uses_native_protocol_text`

- `crates/vw-agent/src/channels/prompt_tests.rs::runtime_tool_visibility_reports_no_tools_and_xml_protocol` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::runtime_tool_visibility_reports_no_tools_and_xml_protocol`

- `crates/vw-agent/src/channels/prompt_tests.rs::runtime_tool_visibility_xml_protocol_lists_sorted_tools_and_exclusions` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::runtime_tool_visibility_xml_protocol_lists_sorted_tools_and_exclusions`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_context_skip_predicate_handles_special_keys_and_size` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_context_skip_predicate_handles_special_keys_and_size`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_context_filters_scores_special_keys_and_truncates_long_entries` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_context_filters_scores_special_keys_and_truncates_long_entries`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_context_is_empty_when_recall_fails_or_filters_everything` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_context_is_empty_when_recall_fails_or_filters_everything`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_context_limits_included_entries_to_maximum` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_context_limits_included_entries_to_maximum`

- `crates/vw-agent/src/channels/prompt_tests.rs::memory_context_stops_when_total_character_budget_would_be_exceeded` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::memory_context_stops_when_total_character_budget_would_be_exceeded`

- `crates/vw-agent/src/channels/prompt_tests.rs::inject_workspace_file_handles_present_missing_empty_and_truncated_files` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::inject_workspace_file_handles_present_missing_empty_and_truncated_files`

- `crates/vw-agent/src/channels/prompt_tests.rs::workspace_identity_context_loads_openclaw_bootstrap_files` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::workspace_identity_context_loads_openclaw_bootstrap_files`

- `crates/vw-agent/src/channels/prompt_tests.rs::load_openclaw_bootstrap_files_does_not_include_bootstrap_file` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::load_openclaw_bootstrap_files_does_not_include_bootstrap_file`

- `crates/vw-agent/src/channels/prompt_tests.rs::workspace_identity_context_accepts_identity_config_and_default_limit` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::workspace_identity_context_accepts_identity_config_and_default_limit`

- `crates/vw-agent/src/channels/prompt_tests.rs::system_prompt_includes_tools_workspace_runtime_and_execution_style` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::system_prompt_includes_tools_workspace_runtime_and_execution_style`

- `crates/vw-agent/src/channels/prompt_tests.rs::system_prompt_adds_default_multi_agent_policy_only_when_agent_tool_available` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::system_prompt_adds_default_multi_agent_policy_only_when_agent_tool_available`

- `crates/vw-agent/src/channels/prompt_tests.rs::system_prompt_teaches_event_driven_subagent_wait_and_synthesis` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::system_prompt_teaches_event_driven_subagent_wait_and_synthesis`

- `crates/vw-agent/src/channels/prompt_tests.rs::system_prompt_default_wrapper_uses_xml_task_text` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::system_prompt_default_wrapper_uses_xml_task_text`

- `crates/vw-agent/src/channels/prompt_tests.rs::system_prompt_with_compact_skill_mode_excludes_static_skill_content` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::system_prompt_with_compact_skill_mode_excludes_static_skill_content`

- `crates/vw-agent/src/channels/routing.rs::available_provider_ids` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::available_provider_ids` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::resolve_provider_alias` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::default_route_selection` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::classify_message_route` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::set_route_selection` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing.rs::resolve_provider_alias` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/routing_tests.rs::chat_with_system` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::chat_with_system`

- `crates/vw-agent/src/channels/routing_tests.rs::route` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::route`

- `crates/vw-agent/src/channels/routing_tests.rs::context` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::context`

- `crates/vw-agent/src/channels/routing_tests.rs::resolve_provider_alias_normalizes_known_provider_id_case` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::resolve_provider_alias_normalizes_known_provider_id_case`

- `crates/vw-agent/src/channels/routing_tests.rs::resolve_provider_alias_rejects_blank_input` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::resolve_provider_alias_rejects_blank_input`

- `crates/vw-agent/src/channels/routing_tests.rs::available_provider_ids_is_sorted` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::available_provider_ids_is_sorted`

- `crates/vw-agent/src/channels/routing_tests.rs::route_selection_defaults_to_runtime_defaults_and_removes_default_override` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::route_selection_defaults_to_runtime_defaults_and_removes_default_override`

- `crates/vw-agent/src/channels/routing_tests.rs::route_selection_recovers_from_poisoned_override_lock` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::route_selection_recovers_from_poisoned_override_lock`

- `crates/vw-agent/src/channels/routing_tests.rs::classify_message_route_returns_matching_model_route` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::classify_message_route_returns_matching_model_route`

- `crates/vw-agent/src/channels/routing_tests.rs::classify_message_route_returns_none_when_hint_has_no_model_route` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::classify_message_route_returns_none_when_hint_has_no_model_route`

- `crates/vw-agent/src/channels/routing_tests.rs::classify_message_route_returns_none_when_classification_is_disabled` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::classify_message_route_returns_none_when_classification_is_disabled`

- `crates/vw-agent/src/channels/routing_tests.rs::load_cached_model_preview_handles_missing_invalid_and_limited_cache` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::load_cached_model_preview_handles_missing_invalid_and_limited_cache`

- `crates/vw-agent/src/channels/routing_tests.rs::build_models_help_response_includes_cached_models_or_refresh_hint` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::build_models_help_response_includes_cached_models_or_refresh_hint`

- `crates/vw-agent/src/channels/scope.rs::resolve_channel_message_snapshot` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/scope.rs::scoped_history_key` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/scope.rs::channel_chat_gate` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/scope_delivery_tests.rs::name` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::name`

- `crates/vw-agent/src/channels/scope_delivery_tests.rs::send` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/scope_delivery_tests.rs::listen` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/scope_delivery_tests.rs::channel_project_scope_instance_reply_and_route_stay_with_sender` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::clear_sender_session_id` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::resolve_or_create_sender_session_id` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::owned_channel_sessions` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::create_channel_session` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::run_session_processor_for_channel` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::collect_channel_response` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::get` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::get`

- `crates/vw-agent/src/channels/session.rs::reserve` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::discard` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session.rs::channel_prompt_event` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::chat_with_system` → `crates/libs/channel-support/src/host_migration_43_tests/routing.rs::chat_with_system`

- `crates/vw-agent/src/channels/session_tests.rs::channel_message` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_message`

- `crates/vw-agent/src/channels/session_tests.rs::test_context` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::processor_request` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::run_session_processor_for_channel_accumulates_and_forwards_docs_output` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::run_session_processor_for_channel_propagates_docs_error` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::to_session_history_maps_known_roles_and_skips_unknown_roles` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::to_session_history_maps_known_roles_and_skips_unknown_roles`

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_rooms_have_independent_history` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_project_scope_rooms_have_independent_history`

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_instances_have_independent_history` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_project_scope_instances_have_independent_history`

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_threads_have_independent_history` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_project_scope_threads_have_independent_history`

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_senders_have_independent_history` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_project_scope_senders_have_independent_history`

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_identity_delimiters_do_not_collide` → `crates/libs/channel-support/src/host_migration_43_tests/session.rs::channel_project_scope_identity_delimiters_do_not_collide`

- `crates/vw-agent/src/channels/session_tests.rs::registered_context` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_binding_priority_and_immutable_snapshot` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_invalid_binding_preserves_state` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_project_history_and_routes_are_independent` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_unavailable_directory_does_not_cache_fake_session` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_tenant_cannot_resolve_other_tenant_binding` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_processor_uses_captured_directory` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_memory_reads_only_matching_scope` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::channel_project_scope_snapshot_rejects_different_sender` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_tests.rs::isolated_processor_fixture` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::take_committed_channel_reply` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::persist_channel_turn` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::lock_history` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::get` → `crates/libs/channel-runtime/src/host_migration_43_tests/prompt.rs::get`

- `crates/vw-agent/src/channels/session_turn.rs::now_ms` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::first_response_source` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::load` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::commit_first` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::commit_snapshot` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::refresh` → 原宿主文件保留；职责见上表

- `crates/vw-agent/src/channels/session_turn.rs::spawn_title` → 原宿主文件保留；职责见上表

## 注册及静态核对

运行库与支撑库 lib.rs 均以原生 cfg(test) 自注册 host_migration_43_tests 子模块；没有 path/include 宿主源码或反向依赖。旧 prompt_tests/routing_tests 文件及挂载删除；mod_tests 仅保留公开入口契约；session_tests 仅移除六个纯领域用例及其旧定义。任务1额外归属 dispatch_tests.rs 和 dispatch_cancel_tests.rs 均为真实宿主调度/模型取消流验收，已物理归还宿主同名文件并删除跨目录 path。测试和夹具原样保留，原模块名不变，其他任务的 execution_control_tests 对 channels::dispatch_tests 的引用继续有效。其余跨目录挂载不属本任务。

测试函数体和断言沿用；必要变化仅类型路径、纯领域 API 参数、异步目录调用；session 身份键使用固定默认租户，与原夹具默认租户相同。宿主真实租户/数据库和进程隔离用例保留。

## 行数分项

- 新增领域生产：365 行（含文档/导入/注册，不等同算法行数）。

- 新增领域测试：957 行（含文档/导入/注册，不等同算法行数）。

- 宿主原测试：1418 → 489 行。

- 宿主原生产/转发/文档：1345 → 978 行。

## 未来限定验证（本次未运行）

- `python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_43_tests -- --test-threads=1`：静态预期 25 个，必须实际非零且 failed=0 才算通过。

- `python3 scripts/cargo_slot.py cargo test -p channel-support --lib host_migration_43_tests -- --test-threads=1`：静态预期 18 个，必须实际非零且 failed=0 才算通过。

- 宿主未来按上表保留的原完整测试名运行精确过滤；迁走的函数需用宿主 list 证明消失，不以零测试声称通过。处理器、租户及投递集成测试仍需原生验证。

- 未取得编译耗时或行为结果，不能宣称编译提速或行为验收通过。

附：上述两个外部测试文件物理归还宿主，分别包含并发许可/中断/反馈投递及真实 processor 取消与清理；原名、断言、子进程隔离不变。它们不计领域迁移收益；前述同路径行数口径未含新增归还宿主文件，最终全局宿主行数须包含二者。

依赖补充：运行库测试直接使用 model-selection 公开决策类型，新增 dev 依赖并同步锁文件；未使用不存在的 session_engine::llm 路径。

外部挂载 `dispatch_tests.rs`：356 行，生产0行，全部真实宿主验收/夹具，保留函数：chat_with_system, new, blocking, feedback_blocking, sent_contents, release, wait_for_active_sends, max_active_sends, name, send, stop_typing, listen, context, message, registered_context, compute_max_in_flight_messages_clamps_to_configured_bounds, log_worker_join_result_ignores_success_and_records_join_errors, run_message_dispatch_loop_processes_runtime_commands_until_input_closes, run_message_dispatch_loop_limits_in_flight_workers, run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel, run_message_dispatch_loop_keeps_distinct_telegram_senders_independent, run_message_dispatch_loop_stops_when_semaphore_closed_before_receive, run_message_dispatch_loop_does_not_interrupt_model_queries。

外部挂载 `dispatch_cancel_tests.rs`：358 行，生产0行，全部真实宿主验收/夹具，保留函数：cancel_idle, channel_command_cancel_qq_idle_is_idempotent, channel_command_cancel_lark_idle_is_idempotent, channel_command_cancel_telegram_does_not_require_auto_interrupt, channel_command_cancel_invalid_arguments_preserve_state, channel_command_cancel_help_describes_scope, isolated_cancel_test, cancel_fixture, wait_cancel_flag, channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session, channel_command_cancel_isolates_instance_room_thread_sender_and_tenant, channel_command_cancel_unconfirmed_exit_keeps_mutations_busy, channel_command_cancel_busy_commands_preserve_state_and_status_responds, channel_command_cancel_natural_completion_then_stop_is_idempotent, channel_command_cancel_waits_for_feedback_before_releasing_scope, channel_command_cancel_dropped_caller_still_drains_processor。

外部挂载 `dispatch_tests.rs`：356 行，生产0行，全部真实宿主验收/夹具，保留函数：chat_with_system, new, blocking, feedback_blocking, sent_contents, release, wait_for_active_sends, max_active_sends, name, send, stop_typing, listen, context, message, registered_context, compute_max_in_flight_messages_clamps_to_configured_bounds, log_worker_join_result_ignores_success_and_records_join_errors, run_message_dispatch_loop_processes_runtime_commands_until_input_closes, run_message_dispatch_loop_limits_in_flight_workers, run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel, run_message_dispatch_loop_keeps_distinct_telegram_senders_independent, run_message_dispatch_loop_stops_when_semaphore_closed_before_receive, run_message_dispatch_loop_does_not_interrupt_model_queries。

外部挂载 `dispatch_cancel_tests.rs`：358 行，生产0行，全部真实宿主验收/夹具，保留函数：cancel_idle, channel_command_cancel_qq_idle_is_idempotent, channel_command_cancel_lark_idle_is_idempotent, channel_command_cancel_telegram_does_not_require_auto_interrupt, channel_command_cancel_invalid_arguments_preserve_state, channel_command_cancel_help_describes_scope, isolated_cancel_test, cancel_fixture, wait_cancel_flag, channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session, channel_command_cancel_isolates_instance_room_thread_sender_and_tenant, channel_command_cancel_unconfirmed_exit_keeps_mutations_busy, channel_command_cancel_busy_commands_preserve_state_and_status_responds, channel_command_cancel_natural_completion_then_stop_is_idempotent, channel_command_cancel_waits_for_feedback_before_releasing_scope, channel_command_cancel_dropped_caller_still_drains_processor。

## 深度迁移补充（本节覆盖前述旧保留结论）

已读取 final.md 与 remaining-host.json 的 owner=43 残留。全轮门槛仍开放，本任务不声明全轮完成。

- routing.rs：整个上下文投影、同步原生目录桥及 wasm 空目录分支归 channel-support::routing_context，宿主仅公开兼容重导出。真实宿主命令调用保持原命名进入该入口。依赖方向不变；既有 routing 算法不重复计收益。
- session.rs::channel_prompt_event：完整 match 分支移入 channel-runtime::session_events，输入直接使用 session-engine::processor_events::StreamEvent，输出仍为 session-port::PromptEvent。原 processor 回调直接调用迁后的函数。无回调转移业务、无新增 Host trait。
- 四个 scope 行为函数完整迁入 channel-support::host_migration_43_tests::scope，保留所有断言和真实 agent-project 存储。仅参数投影与类型路径调整，未用 mock 替代租户或文件访问。复用原 routing 测试 context 和 session 测试 channel_message 夹具；真实项目仍通过 agent_project::from_directory 注册。
- 支撑库原生 dev-dependency 启用既有 agent-database/test-support，沿用现有线程数据库隔离路径，未改进程环境；无新增锁文件包或依赖边（该库本已有 agent-database 依赖）。宿主注册、processor reserve/discard 等真实端口保留。
- 新增运行库两个事件投影测试，覆盖文本、终态成功/失败、模型工具调用过滤、步骤元数据、上下文路径、工具轮结束与待办续轮。未执行，不声称红绿证据。
- 修复宿主 captured_directory 子进程过滤原先遗漏 session::session_tests 的完整名称，并要求子进程输出含 1 passed；防止零测试被 status.success 误认通过。属于验收挂载修复，未改处理器行为。

### 本轮映射与注册

- `channels::session::session_tests::channel_project_scope_binding_priority_and_immutable_snapshot` → `channel_support::host_migration_43_tests::scope::channel_project_scope_binding_priority_and_immutable_snapshot`，由 lib.rs → host_migration_43_tests.rs → scope.rs 自注册；原宿主定义/属性已删除。

- `channels::session::session_tests::channel_project_scope_invalid_binding_preserves_state` → `channel_support::host_migration_43_tests::scope::channel_project_scope_invalid_binding_preserves_state`，由 lib.rs → host_migration_43_tests.rs → scope.rs 自注册；原宿主定义/属性已删除。

- `channels::session::session_tests::channel_project_scope_project_history_and_routes_are_independent` → `channel_support::host_migration_43_tests::scope::channel_project_scope_project_history_and_routes_are_independent`，由 lib.rs → host_migration_43_tests.rs → scope.rs 自注册；原宿主定义/属性已删除。

- `channels::session::session_tests::channel_project_scope_tenant_cannot_resolve_other_tenant_binding` → `channel_support::host_migration_43_tests::scope::channel_project_scope_tenant_cannot_resolve_other_tenant_binding`，由 lib.rs → host_migration_43_tests.rs → scope.rs 自注册；原宿主定义/属性已删除。

- `crates/vw-agent/src/channels/routing.rs`：本轮 281 → 2 行。

- `crates/vw-agent/src/channels/session.rs`：本轮 213 → 195 行。

- `crates/vw-agent/src/channels/session_tests.rs`：本轮 419 → 318 行。

### 当前仍保留的真实边界

- `session.rs`：195 行；clear_sender_session_id, resolve_or_create_sender_session_id, owned_channel_sessions, create_channel_session, run_session_processor_for_channel, collect_channel_response, get, reserve, discard。

- `session_tests.rs`：318 行；chat_with_system, channel_message, test_context, processor_request, run_session_processor_for_channel_accumulates_and_forwards_docs_output, run_session_processor_for_channel_propagates_docs_error, registered_context, channel_project_scope_sessions_follow_snapshot_and_isolate_rooms, channel_project_scope_unavailable_directory_does_not_cache_fake_session, channel_project_scope_processor_uses_captured_directory, channel_project_scope_memory_reads_only_matching_scope, channel_project_scope_snapshot_rejects_different_sender, isolated_processor_fixture。

- `scope.rs`：27 行；resolve_channel_message_snapshot, scoped_history_key, channel_chat_gate。

- `session_turn.rs`：61 行；take_committed_channel_reply, persist_channel_turn, lock_history, get, now_ms, first_response_source, load, commit_first, commit_snapshot, refresh, spawn_title。

- `mod.rs`：334 行；。

- `scope_delivery_tests.rs`：44 行；name, send, listen, channel_project_scope_instance_reply_and_route_stay_with_sender。

- `mod_tests.rs`：35 行；root_module_reexports_core_channel_types, assert_channel, native_channel_reexports_are_constructible。

- `prompt.rs`：20 行；build_workspace_protocol_messages。

- `dispatch_tests.rs`：356 行；chat_with_system, new, blocking, feedback_blocking, sent_contents, release, wait_for_active_sends, max_active_sends, name, send, stop_typing, listen, context, message, registered_context, compute_max_in_flight_messages_clamps_to_configured_bounds, log_worker_join_result_ignores_success_and_records_join_errors, run_message_dispatch_loop_processes_runtime_commands_until_input_closes, run_message_dispatch_loop_limits_in_flight_workers, run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel, run_message_dispatch_loop_keeps_distinct_telegram_senders_independent, run_message_dispatch_loop_stops_when_semaphore_closed_before_receive, run_message_dispatch_loop_does_not_interrupt_model_queries。

- `dispatch_cancel_tests.rs`：358 行；cancel_idle, channel_command_cancel_qq_idle_is_idempotent, channel_command_cancel_lark_idle_is_idempotent, channel_command_cancel_telegram_does_not_require_auto_interrupt, channel_command_cancel_invalid_arguments_preserve_state, channel_command_cancel_help_describes_scope, isolated_cancel_test, cancel_fixture, wait_cancel_flag, channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session, channel_command_cancel_isolates_instance_room_thread_sender_and_tenant, channel_command_cancel_unconfirmed_exit_keeps_mutations_busy, channel_command_cancel_busy_commands_preserve_state_and_status_responds, channel_command_cancel_natural_completion_then_stop_is_idempotent, channel_command_cancel_waits_for_feedback_before_releasing_scope, channel_command_cancel_dropped_caller_still_drains_processor。

保留原因：session.rs 是具体 processor 启动与 task-local 项目/租户/测试数据库继承、请求工件 ID 和实际会话 I/O；session_tests 中的剩余场景必须调用真实 reserve/get/discard、处理器与流事件，registered_context/channel_message 被宿主命令/调度测试调用；session_turn.rs 是实际存储事务及标题调度端口；scope.rs 仅类型/函数公开兼容和当前租户注入；prompt.rs 仅宿主协议发现；mod.rs 含当前具体工厂/特性组合和其他编号模块注册。dispatch 两组和 scope_delivery 继续验收真实宿主实例选择、取消流和反馈完成顺序，不用模拟执行器替代。剩余 memory_reads 场景仍通过宿主真实消息路径，不能用纯键测试替换。

### 当前静态证据与未来限定命令

已核对共享输入/输出枚举、context API、真实项目存储调用与依赖方向；无领域到 vw-agent 的新依赖或源码 path/include 反挂载。git diff --check 在本任务路径无空白错误。所有 Cargo、构建及测试均未运行；没有编译或行为通过证据。

- 未来 `python3 scripts/cargo_slot.py cargo test -p channel-support --lib host_migration_43_tests -- --test-threads=1`：静态预期 22 个，未运行。

- 未来 `python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_43_tests -- --test-threads=1`：静态预期 27 个，未运行。

- 未来宿主精确过滤 `channels::session::session_tests::channel_project_scope_processor_uses_captured_directory`，必须非零；其余真实处理器/事务场景按上述完整原名过滤。

## 第三轮：真实宿主测试的领域契约化（覆盖前述整文件保留结论）

再次读取最新 remaining-host.json 的任务43残留及原文件、现有 command_history_contracts / command_catalog_contracts。沿用库内已有 test-support 契约模式，没有引入生产 Host trait 或模拟处理器。

session_tests.rs：原七个完整行为函数的夹具、请求构建和断言迁至 channel-runtime/src/session_contracts.rs。宿主保留七个同名 tokio 测试入口，负责真实进程隔离及注入原 processor / 原 RuntimeHost；registered_context 与 channel_message 保留原公开路径的重导出，宿主命令、项目及调度测试继续复用。Request 使用原类型别名对应的 runner_types::Request<agent_approval::ApprovalManager, agent_skills::session_sources::SessionSkillSources>，没有克隆业务请求模型。

真实端口：session_id/get_session 来自原 RuntimeHost，仍调用原宿主 resolve_or_create_sender_session_id / session::get_any；captured_directory 调用原 channel-runtime::manager::message_execution::run_message_execution 并注入同一静态 RuntimeHost；文档输出/失败通过 Processor 函数指针回到原 run_session_processor_for_channel。后者只为测试契约定义，未新增生产回调路径。会话/存储及项目生命周期未被模拟。

调度 RecordingChannel/StaticProvider/context/message 完整夹具迁至 dispatch_test_support.rs，原子计数、阻塞发送和反馈释放保持不变。原 native dispatch 六个完整场景的断言迁至 dispatch_contracts.rs；宿主保留同名薄入口注入真实 RuntimeHost。取消流测试依赖原 dispatch_tests 的夹具公开路径仍有效。两条纯策略用例 compute_max_in_flight_messages_clamps_to_configured_bounds / log_worker_join_result_ignores_success_and_records_join_errors 在 manager::dispatch 下以 host_migration_43_tests 自注册，使用同一私有常量，原宿主定义删除。

lib.rs：session_contracts/dispatch_contracts 只在现有 test-support feature 编入；dispatch_test_support 在 test 或 test-support 编入。当前 vw-agent dev-dependency 已启用 channel-runtime/test-support，未修改其清单。test-support 新增 dep:tempfile（原已有 dev 依赖/锁文件边），只服务现有真实临时目录夹具。普通产品构建不启用这些契约。

### 本轮完整原用例映射

- `channels::session::session_tests::run_session_processor_for_channel_accumulates_and_forwards_docs_output` → `channel_runtime::session_contracts::run_session_processor_for_channel_accumulates_and_forwards_docs_output`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::run_session_processor_for_channel_propagates_docs_error` → `channel_runtime::session_contracts::run_session_processor_for_channel_propagates_docs_error`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms` → `channel_runtime::session_contracts::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::channel_project_scope_unavailable_directory_does_not_cache_fake_session` → `channel_runtime::session_contracts::channel_project_scope_unavailable_directory_does_not_cache_fake_session`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::channel_project_scope_processor_uses_captured_directory` → `channel_runtime::session_contracts::channel_project_scope_processor_uses_captured_directory`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::channel_project_scope_memory_reads_only_matching_scope` → `channel_runtime::session_contracts::channel_project_scope_memory_reads_only_matching_scope`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::session::session_tests::channel_project_scope_snapshot_rejects_different_sender` → `channel_runtime::session_contracts::channel_project_scope_snapshot_rejects_different_sender`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_processes_runtime_commands_until_input_closes` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_processes_runtime_commands_until_input_closes`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_limits_in_flight_workers` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_limits_in_flight_workers`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_keeps_distinct_telegram_senders_independent` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_keeps_distinct_telegram_senders_independent`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_stops_when_semaphore_closed_before_receive` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_stops_when_semaphore_closed_before_receive`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

- `channels::dispatch_tests::run_message_dispatch_loop_does_not_interrupt_model_queries` → `channel_runtime::dispatch_contracts::run_message_dispatch_loop_does_not_interrupt_model_queries`；宿主同名 tokio 入口保留并直接调用；全部断言归领域契约，仍需原生宿主执行。

### 当前物理行数及保留边界

- 宿主 `session_tests.rs`：70 行。

- 宿主 `dispatch_tests.rs`：31 行。

- 宿主 `dispatch_cancel_tests.rs`：358 行。

- 宿主 `session.rs`：195 行。

- 宿主 `session_turn.rs`：61 行。

- 宿主 `scope.rs`：27 行。

- 宿主 `mod.rs`：334 行。

任务原26路径 splitlines：2763 → 830，净减 1933（70.0%）；物理归还的 dispatch_tests / dispatch_cancel_tests 属额外跨挂载范围，未混入此分母。

生产：本轮仅增加 cfg(test) 的原 RuntimeHost 访问入口，生产算法未变，不把迁测试计作生产下沉。仍保留 session.rs 的实际 processor 启动、租户/项目/测试数据库继承和请求工件身份；session_turn.rs 的实际持久化/标题端口；scope.rs 的当前租户注入；mod.rs 的实际工厂/平台/特性挂载；dispatch_cancel_tests 的真实 processor ChannelCancelStream 注册、取消与退出信号/子进程安全隔离。没有把这些真实端口复制进库或用 mock 取代。

### 静态检查及未来命令

已逐条比对宿主包装函数 → 领域契约 → 原 RuntimeHost/processor 的调用链，核对 Request/StreamEvent 和 LlmExecutionResult 的真实公开归属。注册、feature 与 Cargo.toml 依赖边静态核对完成；所有 Cargo、构建和测试均未运行。没有执行测试或声称通过。

- 未来 `python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib host_migration_43_tests -- --test-threads=1`：本轮新增2条 dispatch 策略测试；原有27条加本轮2条，静态预期29个。
- 未来 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channels::session::session_tests -- --test-threads=1`：静态预期7条；必须实际非零且 failed=0。
- 未来 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channels::dispatch_tests -- --test-threads=1`：静态预期6条；两条纯策略原完整名称应从宿主 list 消失。
- domain contracts 不是独立 #[test] 注册，不伪称为库单测通过；它们由上述真实宿主测试驱动。compiled ownership 需未来编译验证，库普通默认功能不会编入契约。

## 第四轮：取消流契约（覆盖此前整组保留结论）

再次读取最新 final.md 与 remaining-host.json；只继续任务43的额外挂载 dispatch_cancel_tests。完整12场景的取消/作用域/命令状态/反馈退出时序断言归 channel-runtime::cancel_contracts，仍驱动原 RuntimeHost。测试专用 CancelSignals 仅借用实际已注册 ChannelCancelStream 的 started/exited/release 原子信号，没有复制状态或模拟处理器；CancelFixture 传递原 Arc ctx/channel、msg/session 和信号引用。TempDir 与 Arc 流由宿主包装函数持有至契约返回。

宿主保留真实 config 子进程隔离、项目注册、/new 请求、register_channel_cancel_stream，以及 take_test_llm_messages 捕获记录的长度、目录 canonicalize 和租户检查。第一条领域契约返回不可变快照给宿主继续此检查。worker spawn 只持有原 Arc ctx/msg 及静态 RuntimeHost，借用信号未搬入 static task。全部原断言保留，五个 idle/help 场景同样注入真实 RuntimeHost。

lib.rs 仅在现有 test-support feature 导出 cancel_contracts；vw-agent 原 dev-dependency 已启用该 feature；无新 Cargo 依赖或生产 Host trait。领域契约不自注册为独立库测试，原宿主完整名称和 tokio 注册保留。

### 全用例映射

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_qq_idle_is_idempotent` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_qq_idle_is_idempotent`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_lark_idle_is_idempotent` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_lark_idle_is_idempotent`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_telegram_does_not_require_auto_interrupt` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_telegram_does_not_require_auto_interrupt`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_invalid_arguments_preserve_state` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_invalid_arguments_preserve_state`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_help_describes_scope` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_help_describes_scope`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_isolates_instance_room_thread_sender_and_tenant` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_isolates_instance_room_thread_sender_and_tenant`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_unconfirmed_exit_keeps_mutations_busy` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_unconfirmed_exit_keeps_mutations_busy`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_busy_commands_preserve_state_and_status_responds` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_busy_commands_preserve_state_and_status_responds`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_natural_completion_then_stop_is_idempotent` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_natural_completion_then_stop_is_idempotent`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_waits_for_feedback_before_releasing_scope` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_waits_for_feedback_before_releasing_scope`。

- `channels::channel_command_cancel_dispatch_tests::channel_command_cancel_dropped_caller_still_drains_processor` → 同名薄宿主入口 → `channel_runtime::cancel_contracts::channel_command_cancel_dropped_caller_still_drains_processor`。

物理宿主 358 → 137 行；领域契约 246 行。生产收益0；assert/assert_eq/assert_ne 静态数量 47 → 47，原 tokio 注册12个仍12个。静态差异核对不等于编译或行为通过。

未来 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channels::channel_command_cancel_dispatch_tests -- --test-threads=1` 静态预期12个，必须非零。未运行任何 Cargo、构建或测试；真实处理器、租户和子进程验证仍需未来原生执行，不以库过滤零用例宣称通过。git diff --check 已核对；编号任务文件维持删除状态。
