# 任务38 静态迁移证据
代码完成；编译/行为未验证。按用户要求未运行 Cargo、构建、单元或实机测试。

## 所有者与接线
实际接收库选择 agent-plugins：被测 codex_handlers、trusted_plugin_hook_handlers、信任准入及真实命令执行均由该库持有；plugin-hooks 已持有协议和 HookHost，没有新增生产算法，不能计作本次生产迁移。依赖方向 agent-plugins → plugin-hooks → tool-context/tool-mcp，无反向依赖宿主。
领域自注册 src/lib.rs::host_migration_38_tests；原 compat 模块完整移除对应测试挂载。t41/t44 只留真实宿主循环调用的 fixture/runner/register，不留已迁测试。测试端口 Host 仅提供 task-local、会话容器与 HookRunner，MCP/配置/鉴权未用端口明确失败；真实目录、信任文件与命令仍执行原算法。

## 全文件与逐用例映射

### crates/vw-agent/src/plugins/compat/hook_async.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_command.rs
20 → 20 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_compact.rs
23 → 23 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_manifest.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_matcher.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_model.rs
8 → 8 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_permission.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_post_tool.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_pre_tool.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_prompt.rs
8 → 8 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_session.rs
41 → 41 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_stop.rs
15 → 15 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_subagent.rs
29 → 29 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/hook_trust_content.rs
2 → 2 行。保留：实际调用者的 AgentContextHost 类型绑定/公共协议重导出；hook_session 还实现真实租户配置、清理、连接端口

### crates/vw-agent/src/plugins/compat/platform_tests.rs
42 → 0 行。agent-plugins/src/host_migration_38_tests/platform_tests.rs
- `codex_plugin_compat_t64_executes_host_hook_command` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/plugin_window_fixture.rs
115 → 115 行。保留：真实 AppState/鉴权 HTTP 服务及原生窗口进程 fixture，不能复制 AppState 到领域库
- `plugin_window_gateway_fixture` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/session_snapshot.rs
38 → 38 行。保留：实际租户与技能/MCP turn admission 装配

### crates/vw-agent/src/plugins/compat/t32_tests.rs
72 → 0 行。agent-plugins/src/host_migration_38_tests/t32_tests.rs
- `codex_plugin_compat_t32_runtime_requires_explicit_content_trust` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t33_tests.rs
84 → 0 行。agent-plugins/src/host_migration_38_tests/t33_tests.rs
- `codex_plugin_compat_t33_executes_only_matching_handlers` → 领域同名（原断言保留）
- `codex_plugin_compat_t33_preserves_isolation` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t34_tests.rs
258 → 0 行。agent-plugins/src/host_migration_38_tests/t34_tests.rs
- `codex_plugin_compat_t34_interprets_command_exit_protocol` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_applies_updated_input` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_spills_context_only_in_own_turn` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_receives_host_input_and_environment` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_bounds_command_runtime_and_output` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_rejects_missing_session_context` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_model_hook_is_observation_only` → 领域同名（原断言保留）
- `codex_plugin_compat_t34_rejects_data_symlink_escape` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t35_tests.rs
136 → 0 行。agent-plugins/src/host_migration_38_tests/t35_tests.rs
- `codex_plugin_compat_t35_session_end_runs_async_declaration_synchronously` → 领域同名（原断言保留）
- `codex_plugin_compat_t35_async_command_cannot_revoke_decision` → 领域同名（原断言保留）
- `codex_plugin_compat_t35_finished_turn_kills_real_process` → 领域同名（原断言保留）
- `codex_plugin_compat_t35_delivers_only_to_original_live_turn` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t36_tests.rs
66 → 66 行。保留：真实 execute_tool_from_registry 前置拦截/文件副作用边界
- `codex_plugin_compat_t36_blocks_tool_before_side_effect` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t37_tests.rs
86 → 0 行。agent-plugins/src/host_migration_38_tests/t37_tests.rs
- `codex_plugin_compat_t37_preserves_host_permission_denial` → 领域同名（原断言保留）
- `codex_plugin_compat_t37_runs_real_command_in_own_turn` → 领域同名（原断言保留）
- `codex_plugin_compat_t37_real_command_timeout_fails_closed` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t38_tests.rs
119 → 119 行。保留：真实工具成功失败执行与后置事件顺序边界
- `codex_plugin_compat_t38_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t38_delivers_result_context_once` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t38_captures_failed_large_result` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t39_tests.rs
86 → 0 行。agent-plugins/src/host_migration_38_tests/t39_tests.rs
- `codex_plugin_compat_t39_emits_session_events_once` → 领域同名（原断言保留）
- `codex_plugin_compat_t39_preserves_isolation` → 领域同名（原断言保留）
- `codex_plugin_compat_t39_failed_start_cleans_up` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t40_tests.rs
90 → 0 行。agent-plugins/src/host_migration_38_tests/t40_tests.rs
- `codex_plugin_compat_t40_blocks_before_provider_request` → 领域同名（原断言保留）
- `codex_plugin_compat_t40_rejects_invalid_input` → 领域同名（原断言保留）
- `codex_plugin_compat_t40_preserves_isolation` → 领域同名（原断言保留）
- `codex_plugin_compat_t40_requires_trusted_source` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t41_tests.rs
96 → 35 行。agent-plugins/src/host_migration_38_tests/t41_tests.rs
- `codex_plugin_compat_t41_wraps_actual_model_request` → 领域同名（原断言保留）
- `codex_plugin_compat_t41_rejects_invalid_input` → 领域同名（原断言保留）
- `codex_plugin_compat_t41_preserves_isolation` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t42_tests.rs
138 → 138 行。保留：真实 session compaction checkpoint/restore 边界
- `codex_plugin_compat_t42_emits_events_around_compaction` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t42_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t42_applies_stop_at_the_correct_boundary` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t43_tests.rs
264 → 264 行。保留：真实 child_agent create/run/继续执行边界
- `codex_plugin_compat_t43_binds_events_to_child_session` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t43_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t43_persistent_success_failure_and_cancellation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t43_routes_context_and_stop_to_child` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t43_continues_only_the_blocked_child` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t44_tests.rs
156 → 41 行。agent-plugins/src/host_migration_38_tests/t44_tests.rs
- `codex_plugin_compat_t44_distinguishes_stop_and_interrupt` → 领域同名（原断言保留）
- `codex_plugin_compat_t44_preserves_isolation` → 领域同名（原断言保留）
- `codex_plugin_compat_t44_cancellation_remains_responsive` → 领域同名（原断言保留）
- `codex_plugin_compat_t44_cancels_in_flight_stop_and_bounds_interrupt` → 领域同名（原断言保留）
- `codex_plugin_compat_t44_session_close_does_not_emit_stop_or_interrupt` → 领域同名（原断言保留）

### crates/vw-agent/src/plugins/compat/t45_tests.rs
133 → 133 行。保留：真实 MCP 服务器发现、授权、连接与 exposure 拒绝边界，测试端口不能替代此验收
- `codex_plugin_compat_t45_calls_only_authorized_hook_server` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t45_rejects_invalid_input` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t45_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t45_admits_trusted_mcp_and_skips_session_end` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t45_respects_tool_authorization` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t54_tests.rs
195 → 195 行。保留：真实 AppState 路由升级鉴权边界
- `codex_plugin_compat_t54_requires_authorized_install_request` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t54_rejects_invalid_input` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t54_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t54_gateway_client_activates_audited_package` → 宿主原完整模块同名（断言保留）

### crates/vw-agent/src/plugins/compat/t57_tests.rs
356 → 356 行。保留：真实 turn admission/tenant/skills/MCP 进程退休、升级互斥边界
- `codex_plugin_compat_t57_switches_version_at_turn_boundary` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_rejects_invalid_input` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_preserves_isolation` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_refreshes_required_hooks_at_boundary` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_joins_cancelled_work_before_releasing_turn` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_defers_new_store_until_next_turn` → 宿主原完整模块同名（断言保留）
- `codex_plugin_compat_t57_retires_old_mcp_process_after_turn` → 宿主原完整模块同名（断言保留）

## 数量及不足70%的逐文件原因
来源物理行数 2688 → 1658，净减 1030（38.3%）；迁移 34 条用例。逐文件原因及行数见上表。生产实现净迁移 0；原来即领域实现，宿主剩余仅绑定和端口。迁移测试文件均为测试职责（含辅助函数），没有把测试夹具算生产收益。
继续可迁项：t36/t38 领域工具执行断言可由 tool-context 真实执行工厂承接，但本任务不复制宿主注册表；t42/t43 需各自 session/child-agent 领域入口完整承接实际生命周期；t45 需已有真实 MCP 授权/连接端口复用；t54 和窗口 fixture 为真实路由验收，t57 为真实跨领域 turn admission 集成。收口需按上表保留量验收，未宣称70%达标。

## 未来验证
`python3 scripts/cargo_slot.py cargo test -p agent-plugins --lib host_migration_38_tests -- --test-threads=1`：Unix 预期 34 条（非 Unix 受原 cfg(unix) 限制），必须 ran>0、failed=0。
宿主未来逐个使用 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib plugins::compat::<上表保留模块>::<上表原名> -- --exact --test-threads=1`；ignored 窗口 fixture 由原生 harness 显式启动。迁出旧完整名需使用宿主 --list 证明消失，不能把零测试称为通过。
静态核对：测试名逐条存在；每个迁移文件 assert 文本数量保持；include_str fixture 已复制到接收库自身 testdata；无 path/include 挂载宿主源码。未声称编译收益、行为通过或运行测试数。

## 深度迁移补充（2026-11-16）
本节覆盖上文 t36/t38、t42隔离、t43事件和 t57发布选择保留说明。读取 final.md/remaining-host.json 的任务38残留1658行后，复查真实入口，将9条完整用例继续迁入 agent-plugins::host_migration_38_tests。

- t36: `codex_plugin_compat_t36_blocks_tool_before_side_effect` → 领域同名。

- t38: `codex_plugin_compat_t38_preserves_isolation`, `codex_plugin_compat_t38_delivers_result_context_once`, `codex_plugin_compat_t38_captures_failed_large_result` → 领域同名。

- t42: `codex_plugin_compat_t42_preserves_isolation` → 领域同名。

- t43: `codex_plugin_compat_t43_binds_events_to_child_session`, `codex_plugin_compat_t43_preserves_isolation` → 领域同名。

- t57: `codex_plugin_compat_t57_switches_version_at_turn_boundary`, `codex_plugin_compat_t57_defers_new_store_until_next_turn` → 领域同名。

t36/t38 使用 tool-context::executor::execute_tool_from_registry 原工厂、tool-types::Tool 原接口；Host 的前后置和权限方法直接委托真实 HookRunner，未重写算法。真实文件副作用、拒绝、输入改写、失败大结果和上下文一次消费断言全部保留。t43 两条事件测试调用 plugin-hooks::hook_subagent::emit_boundary 原入口；真实 child_agent 持久化/成功失败取消与继续执行仍在宿主。t42 仅迁会话隔离，真实 compaction/checkpoint 两条保留。t57 两条发布锁/新增目录选择使用真实 PluginTurnSnapshot::capture/selection 和 upgrade::activate_plugin_upgrade；直接保留 snapshot data 克隆以证明数据不持有生命周期锁。配置 runner 端口为无 hooks，关闭连接端口为空因为此测试未打开 MCP；真实 MCP 进程退休、tenant、required hooks、异步关闭仍在宿主原测试。此两条不再证明 ToolUseContext 装配，由保留的其余 t57 覆盖。

33个原来源目前剩余 1334 行；相对2688行净减 1354 行（50.4%）。本轮生产算法迁移仍为0，新增仅领域测试与窄端口接线；共享 fixture 不计生产收益。未达到70%，保留大头为真实网关/窗口、child_agent、compaction和跨领域turn/MCP集成，逐文件原保留原因除本节覆盖项仍有效。
未来 agent-plugins --lib host_migration_38_tests 过滤：原34+新增9=43条（Unix，其他平台保留原cfg限制），必须非零；所有编译和行为检查未运行。静态检查原名称/断言、入口签名和差异，未运行Cargo、构建或测试。
