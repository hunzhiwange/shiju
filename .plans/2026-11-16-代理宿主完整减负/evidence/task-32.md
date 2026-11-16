# Task 32 evidence

代码及静态接线完成，编译/行为未验证；减量未达 70%，须收口审查保留范围。没有运行 Cargo（含 fmt/list/metadata）、构建、单元或实机测试。

## 全文件/全用例映射

- `crates/vw-agent/src/tools/toolset/mod_tests.rs`：1057 → 973 行。
  - `delegate_agents_auto_load_project_skills_by_default_and_preserve_explicit_skills` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实项目技能文件发现接线；领域合并入口已在 factory/agents，尚可继续拆分纯策略断言。
  - `delegate_agents_require_explicit_skills_when_project_auto_load_is_disabled` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实项目技能文件发现接线；领域合并入口已在 factory/agents，尚可继续拆分纯策略断言。
  - `configured_subagent_opt_out_overrides_builtin_default` → `crates/libs/tool-assembly/src/host_migration_32_tests.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `agent_tool_parallel_limit_uses_current_agent_and_clamps_bounds` → `crates/libs/tool-assembly/src/host_migration_32_tests.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `runtime_context_and_scalar_parsing_cover_core_branches` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；具体 ToolRuntimeContext<AgentContextHost> 默认/上下文接线；混合解析断言按完整函数单位保留，后续可将上下文域拆出。
  - `classify_message_and_binary_detection_cover_errors_and_content` → `crates/libs/tool-assembly/src/host_migration_32_tests.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `block_on_bridges_when_called_inside_tokio_runtime` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；具体宿主 Tokio 桥接生命周期，不能用领域 fixture 替代。
  - `arc_delegating_tool_forwards_methods` → `crates/libs/tool-assembly/src/host_migration_32_tests.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `arc_delegating_tool_forwards_v2_contract_methods` → `crates/libs/tool-assembly/src/host_migration_32_tests.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `default_tools_include_native_capability_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `default_context_constructors_use_current_directory_roots` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；具体 ToolRuntimeContext<AgentContextHost> 默认/上下文接线；混合解析断言按完整函数单位保留，后续可将上下文域拆出。
  - `default_tools_wrapper_uses_native_runtime` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `all_tools_with_runtime_respects_optional_tool_configurations` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `all_tools_wrapper_registers_default_network_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `all_tools_with_runtime_registers_brave_search_without_ipc_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `all_tools_with_runtime_filters_delegate_agents` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `canonical_agent_tool_registers_when_delegate_agent_exists` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `codex_discovery_tool_factory_registers_search_for_deferred_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `codex_discovery_tool_factory_omits_search_without_deferred_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `codex_discovery_tool_factory_searches_authorized_catalog_without_remote_calls` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `codex_discovery_tool_factory_respects_disabled_and_denied_search` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `execution_environment_registers_agent_tool_from_loaded_config` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `default_config_registers_agent_tool_with_builtin_subagents` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `default_toolset_registers_thread_tools_without_delegate_agents` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 thread 工具工厂及请求 disable_tools 过滤接线，不能用名字相同的假工具替代。
  - `thread_tools_visible_without_tool_search` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 thread 工具工厂及请求 disable_tools 过滤接线，不能用名字相同的假工具替代。
  - `restricted_toolset_can_exclude_thread_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；直接验收 session::processor 请求权限过滤入口；不属于 tool-assembly，继续可迁入该入口实际领域所有者。
  - `workflow_and_goal_sessions_inherit_recursive_agent_tool_controls` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 FactoryHost 构造与运行时能力/配置选择接线；保留具体工具注册和 AgentTool 行为验收，领域选择逻辑已在 factory/runtime。
  - `execute_tool_call_runs_dynamic_mcp_tool_through_context_registry` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `execute_tool_from_registry_uses_supplied_tool_list` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 AgentContextHost 权限及执行 registry 接线，保留原 EchoTool。
  - `register_dynamic_mcp_tools_skips_disabled_servers_and_keeps_existing_tools` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 MCP 配置解析、stdio 子进程、会话连接及发现工具构造接线；当前仍为真实宿主集成用例。
  - `shell_outer_timeout_yield_does_not_cancel_process` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实 bash/wait/cancel 注册、桥接 runtime 销毁与进程存活边界。
  - `codex_plugin_compat_t57_toolset_bridge_preserves_turn_and_tenant` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实租户 task-local、插件快照与目录隔离端口；保留实际 Host 验收。
  - `agent_config_cleanup_delegate_fixed_selection_preserves_strict_boundary` → `crates/vw-agent/src/tools/toolset/mod_tests.rs` 同名完整函数；真实项目技能文件发现接线；领域合并入口已在 factory/agents，尚可继续拆分纯策略断言。
- `crates/vw-agent/src/tools/toolset/mod.rs`：492 → 487 行。
- `crates/vw-agent/src/tools/toolset/tests.rs`：142 → 0 行。
  - `full_access_context_disables_workspace_only_for_tool_security` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `full_access_context_allows_high_risk_curl_request` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `full_access_context_allows_shell_paths_outside_workspace` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `full_access_context_bypasses_all_bash_security_checks` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `full_access_context_preserves_and_allows_redirected_shell_command` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。
  - `agcollab_delegated_security_intersects_parent_and_role` → `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs` 同名完整函数；断言原样保留，仅路径与 ContextHost 类型改为领域端口。

## 生产归属与保留边界

本轮没有新增生产逻辑迁移收益；原有实现已经位于 tool-assembly。删除重复 FactoryHost::block_on 成员属于静态修复，不计算为领域下沉。

- `mcp_connections_for_context`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execution_tools_for_context`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `agent_tool_parallel_limit_for_context`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execution_environment_for_config`：仅宿主测试使用的具体泛型绑定；仍调用领域 runtime。
- `tool_specs_for_context`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execute_tool_call`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execute_bound_app_tool`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execute_agent_tool_call`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `execute_tool_from_registry`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `block_on`：保留租户和 ToolUseContext 跨线程、跨 Tokio runtime 生命周期。
- `discover_dynamic_mcp_tools`：绑定 AgentContextHost 的发现入口，调用领域 mcp。
- `default_tools`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `default_tools_with_runtime`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `all_tools`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `all_tools_with_runtime`：保留已有调用者需要的 AgentContextHost 泛型绑定；算法仍唯一在 tool-assembly 的 runtime/factory。
- `enabled_delegate_agents_with_project_skills`：宿主测试绑定，验证真实项目技能发现端口。
- `initialize_worktree_host`：GitToolHost 实现调用 worktree::initialize_host，属于真实全局宿主生命周期。
- `FactoryHost`：construct 持有真实 Shell/File/Memory/Cron/Skill/Thread/Office 等工具构造；batch/search_tool/agent_tool 绑定具体工具；project_scope/project_skills 访问实际租户项目及技能；connections_for_factory/load_runtime_mcp_config/native_tools 保留外部资源接线。
- `RuntimeHost`：load_project_config/mcp_connections/discovery_state/register_app 绑定真实项目配置、连接和 MCP app 注册。

## 注册与依赖方向

- tool-assembly/src/lib.rs 直接 cfg(test) 注册 host_migration_32_tests，内部注册 security 与显式失败的 ContextHost fixture；fixture 不复制 App/AppState，不替代真实鉴权、文件和进程验收。安全测试仍执行真实 SecurityPolicy。
- 删除宿主 tests.rs 与 mod tests 挂载；mod_tests 保留宿主边界，迁走的 5 个函数从原模块删除，无 path/include 反向加载。
- 依赖：vw-agent → tool-assembly → tool-context/agent-security/config-agent；新增仅 dev-dependencies（tempfile/tokio/tokio-util/permission-types）。宿主现有 test-support 依赖保持。
- 直接调用：tools/mod.rs 重导出执行、MCP 连接和工具集入口；Word/Excel control 测试调用 execution_tools_for_context；领域 runtime 直接调用 factory 与 security。任务1 external-mounts 中无 toolset 跨目录挂载。

## 分项行数与减量

- 宿主生产及测试注册：mod.rs=487。
- 宿主测试及夹具：mod_tests.rs=973。
- 领域新增测试及夹具：host_migration_32_tests.rs=153, host.rs=50, security.rs=142。
- 宿主文件族：1691 → 1460，净减 231（13.7%），低于 70%。mod.rs 保留实际端口和泛型绑定；mod_tests.rs 保留上述真实工具构造、MCP、租户及进程边界；tests.rs 整体迁出。
- 继续可迁项：项目技能合并断言、混合上下文/解析用例、请求权限过滤用例，以及工厂用例中的纯选择断言；需要把领域断言与真实 Host 接线验收进一步分离。收口不得将本任务视为已达减量门槛。
- 生产实现增量为 0；领域测试新增模块全部是测试/fixture。宿主 mod.rs 内 cfg(test) 适配不计为生产迁移，物理源码统计保留。未量测编译耗时，不宣称编译收益。

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_32_tests -- --test-threads=1`：静态预期 11 条，必须确认非零及零失败。
- 宿主保留完整测试过滤前缀：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::toolset::mod_tests:: -- --test-threads=1`；逐项名称见上表，静态预期 30 条。
- 迁出的 tools::toolset::tests::* 和上表 5 个 mod_tests 函数未来用宿主 --list 确认消失；零测试仅证明迁走，不能记为行为通过。
- 全部编译和行为检查未运行。当前仅源码、注册、路径、依赖及 git diff --check 静态核对。

## 深度恢复（覆盖前轮保留结论）

读取 final.md 与 remaining-host.json：任务32原残留1460行。本轮将 factory/runtime 的选择和发现断言从具体应用装配分离；未更改生产算法。

- 原宿主 `tools::toolset::mod_tests::runtime_context_and_scalar_parsing_cover_core_branches` → `tool_assembly::host_migration_32_tests::assembly::runtime_context_and_scalar_parsing_cover_core_branches`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::default_context_constructors_use_current_directory_roots` → `tool_assembly::host_migration_32_tests::assembly::default_context_constructors_use_current_directory_roots`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::default_tools_wrapper_uses_native_runtime` → `tool_assembly::host_migration_32_tests::assembly::default_tools_wrapper_uses_native_runtime`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities` → `tool_assembly::host_migration_32_tests::assembly::default_tools_with_runtime_omits_batch_when_runtime_has_no_capabilities`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::all_tools_with_runtime_respects_optional_tool_configurations` → `tool_assembly::host_migration_32_tests::assembly::all_tools_with_runtime_respects_optional_tool_configurations`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::all_tools_with_runtime_registers_brave_search_without_ipc_tools` → `tool_assembly::host_migration_32_tests::assembly::all_tools_with_runtime_registers_brave_search_without_ipc_tools`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::all_tools_with_runtime_filters_delegate_agents` → `tool_assembly::host_migration_32_tests::assembly::all_tools_with_runtime_filters_delegate_agents`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::canonical_agent_tool_registers_when_delegate_agent_exists` → `tool_assembly::host_migration_32_tests::assembly::canonical_agent_tool_registers_when_delegate_agent_exists`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config` → `tool_assembly::host_migration_32_tests::assembly::tool_specs_for_context_registers_dynamic_mcp_tools_from_workspace_config`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::codex_discovery_tool_factory_registers_search_for_deferred_tools` → `tool_assembly::host_migration_32_tests::assembly::codex_discovery_tool_factory_registers_search_for_deferred_tools`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::codex_discovery_tool_factory_omits_search_without_deferred_tools` → `tool_assembly::host_migration_32_tests::assembly::codex_discovery_tool_factory_omits_search_without_deferred_tools`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::codex_discovery_tool_factory_searches_authorized_catalog_without_remote_calls` → `tool_assembly::host_migration_32_tests::assembly::codex_discovery_tool_factory_searches_authorized_catalog_without_remote_calls`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::codex_discovery_tool_factory_respects_disabled_and_denied_search` → `tool_assembly::host_migration_32_tests::assembly::codex_discovery_tool_factory_respects_disabled_and_denied_search`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::execution_environment_registers_agent_tool_from_loaded_config` → `tool_assembly::host_migration_32_tests::assembly::execution_environment_registers_agent_tool_from_loaded_config`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::default_config_registers_agent_tool_with_builtin_subagents` → `tool_assembly::host_migration_32_tests::assembly::default_config_registers_agent_tool_with_builtin_subagents`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::default_toolset_registers_thread_tools_without_delegate_agents` → `tool_assembly::host_migration_32_tests::assembly::default_toolset_registers_thread_tools_without_delegate_agents`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::execute_tool_call_runs_dynamic_mcp_tool_through_context_registry` → `tool_assembly::host_migration_32_tests::assembly::execute_tool_call_runs_dynamic_mcp_tool_through_context_registry`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::execute_tool_from_registry_uses_supplied_tool_list` → `tool_assembly::host_migration_32_tests::assembly::execute_tool_from_registry_uses_supplied_tool_list`；完整断言保留，工厂与上下文绑定领域 Host。
- 原宿主 `tools::toolset::mod_tests::register_dynamic_mcp_tools_skips_disabled_servers_and_keeps_existing_tools` → `tool_assembly::host_migration_32_tests::assembly::register_dynamic_mcp_tools_skips_disabled_servers_and_keeps_existing_tools`；完整断言保留，工厂与上下文绑定领域 Host。

构造端口Selected只提供标识，不实现工具算法，执行明确报错；仅验证领域调用了哪些构造请求。原 all_tools_wrapper_registers_default_network_tools 与 default_tools_include_native_capability_tools 留宿主验证真实装配，不能把端口测试视为真实工具执行验收。MCP使用独立WireHost、真实协议连接池和本地Python stdio服务；不依赖用户全局配置，无OAuth/审批替代。租户快照、项目技能文件发现、真实AgentTool状态、thread请求过滤、runtime桥接、bash进程生存继续保留宿主。

- 宿主 `crates/vw-agent/src/tools/toolset/mod.rs`：485 行。
- 宿主 `crates/vw-agent/src/tools/toolset/mod_tests.rs`：469 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests.rs`：157 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests/assembly.rs`：676 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests/factory_host.rs`：116 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests/host.rs`：50 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests/mcp_host.rs`：40 行。
- 领域测试/夹具 `crates/libs/tool-assembly/src/host_migration_32_tests/security.rs`：142 行。

生产代码未迁移；移除不再使用的宿主测试parse_tool_input导入，其余生产端口和真实调用方向同前。领域自注册assembly/factory_host/mcp_host，未新增依赖，沿用现有库依赖。删除迁出测试旧定义及其专用夹具，保留原模块挂载仅用于真实边界。

未来领域命令仍为 `python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_32_tests -- --test-threads=1`，现静态预期30条；宿主同前过滤，现静态预期11条。全部编译、构建与行为检查未运行，只有源码及diff静态核对。

宿主总计1691 → 954，净减737（43.6%）；仍不足70%。原因：生产485行主要为具体工具构造和宿主资源端口，测试保留真实宿主验收。后续可迁项仅剩混合真实技能发现中的纯合并断言及thread请求过滤（实际所有者不是tool-assembly）；不得将缺口宣称达标。

## 第三轮恢复：真实领域构造与技能发现

已读取最新final.md（宿主51915行）和remaining-host.json（本任务485+469行）。修复前轮按函数末尾截取导致的孤立Python片段：完整fixture一直保存在领域assembly.rs，此次仅清除宿主无效尾段，没有删除有效测试。源码静态审查发现并修复，不宣称编译通过。

真实生产迁移：宿主FactoryHost::construct → tool_assembly::domain_construction::construct_domain_tool::<AgentContextHost>，以下每个构造分支完整保留参数、security/runtime/memory与cfg；返回None的应用特定构造仍由宿主match处理：

- `ShellTool` → `domain_construction.rs` 同名分支。
- `BashWaitTool` → `domain_construction.rs` 同名分支。
- `BashCancelTool` → `domain_construction.rs` 同名分支。
- `ShellToolWithSyscallDetector` → `domain_construction.rs` 同名分支。
- `PowerShellTool` → `domain_construction.rs` 同名分支。
- `FileReadTool` → `domain_construction.rs` 同名分支。
- `FileEditTool` → `domain_construction.rs` 同名分支。
- `FileWriteTool` → `domain_construction.rs` 同名分支。
- `ApplyPatchTool` → `domain_construction.rs` 同名分支。
- `LsTool` → `domain_construction.rs` 同名分支。
- `GlobTool` → `domain_construction.rs` 同名分支。
- `GrepTool` → `domain_construction.rs` 同名分支。
- `FileReadToolWithSession` → `domain_construction.rs` 同名分支。
- `PdfReadTool` → `domain_construction.rs` 同名分支。
- `HttpRequestTool` → `domain_construction.rs` 同名分支。
- `WebFetchTool` → `domain_construction.rs` 同名分支。
- `ExaWebSearchTool` → `domain_construction.rs` 同名分支。
- `WebSearchTool` → `domain_construction.rs` 同名分支。
- `MemoryStoreTool` → `domain_construction.rs` 同名分支。
- `MemoryRecallTool` → `domain_construction.rs` 同名分支。
- `MemoryForgetTool` → `domain_construction.rs` 同名分支。

新增生产依赖tool-shell/tool-memory/tool-services；dev依赖agent-skills。无反向vw-agent依赖，静态核对这四个清单均无tool-assembly依赖。Cargo.lock仅手工同步本包依赖名，不运行Cargo。领域构造夹具优先调用真实生产构造入口，因此Shell/File/Web/Memory已不使用标识receipt；其余应用端口仍拒绝执行receipt。

- `delegate_agents_auto_load_project_skills_by_default_and_preserve_explicit_skills` 原宿主mod_tests → 领域host_migration_32_tests::assembly，同名断言；FactoryHost::project_skills使用agent_skills::catalog真实文件发现，非模拟技能名。
- `delegate_agents_require_explicit_skills_when_project_auto_load_is_disabled` 原宿主mod_tests → 领域host_migration_32_tests::assembly，同名断言；FactoryHost::project_skills使用agent_skills::catalog真实文件发现，非模拟技能名。
- `agent_config_cleanup_delegate_fixed_selection_preserves_strict_boundary` 原宿主mod_tests → 领域host_migration_32_tests::assembly，同名断言；FactoryHost::project_skills使用agent_skills::catalog真实文件发现，非模拟技能名。
- `crates/vw-agent/src/tools/toolset/mod.rs`：376 行；真实宿主端口/集成验收。
- `crates/vw-agent/src/tools/toolset/mod_tests.rs`：257 行；真实宿主端口/集成验收。
- `crates/libs/tool-assembly/src/domain_construction.rs`：124 行；领域生产构造。

宿主1691 → 633，净减1058（62.6%）；仍未达70%。具体保留：mod.rs持有Config/Cron/Schedule/Browser/Skill/Thread等应用构造、租户与连接及Tokio桥接；mod_tests.rs保留8条真实装配、请求过滤、AgentTool状态、进程与租户快照测试。没有通过改名移动真实宿主端口来冲量。

未来原命令领域预期33条、宿主预期8条，新增迁移3条用宿主--list确认消失。当前仅diff/源码/依赖静态核对，任何编译/测试/构建均未运行。当前无编号任务文件需要删除。
