# 来源复核389：src/workflow/native_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/workflow/native_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/workflow/native_tests.rs`。
- 快照 SHA256：`dffe8ffb105b1e0f4d30c7d714db17f695fbd2d76159141906da8363d0cc6203`。源码变化后重新阅读。
- 快照词法属性 13 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/workflow/native.rs:112 | crates/vw-agent/src/workflow/native_tests.rs | mod tests | cfg(not(target_arch="wasm32")); cfg(not(target_arch="wasm32")); cfg(test) | serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `native_workflow_bound_command_executes_graph_and_returns_answer`
- `native_workflow_unmatched_command_preserves_normal_session`
- `native_workflow_skill_palette_selection_dispatches_same_graph`
- `native_workflow_rejects_cross_tenant_binding`
- `native_workflow_rejects_file_outside_project`
- `native_workflow_failed_node_never_reports_success`
- `native_workflow_preserves_cancellation_and_tool_restrictions`
- `native_workflow_provider_names_preserve_tool_restrictions`
- `native_workflow_provider_names_preserve_agent_restrictions`
- `native_workflow_cloud_graph_gates_execution_on_valid_plan_and_preview`
- `task_pool_actions_absent_steps_have_no_side_effects`
- `task_pool_actions_explicit_git_steps_use_native_tool_permissions`
- `task_pool_actions_upstream_failure_skips_downstream_send`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
