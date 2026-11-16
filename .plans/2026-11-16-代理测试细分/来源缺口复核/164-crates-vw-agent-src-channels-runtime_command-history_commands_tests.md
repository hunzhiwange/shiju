# 来源复核164：src/channels/runtime_command/history_commands_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs`。
- 快照 SHA256：`3f8b07c0ff2abbdf9e61e3f5e4a76425e9139444d5de09241e4533fd17e3f854`。源码变化后重新阅读。
- 快照词法属性 11 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/mod.rs:353 | crates/vw-agent/src/channels/runtime_command/history_commands_tests.rs | mod channel_session_history_tests | cfg(test) | vw-agent 宿主模块（生产父声明见左列） | 未匹配→133 | 未发现 |

## 身份线索

- `channel_session_history_lists_real_anchors_without_mutation`
- `channel_session_history_fork_switches_and_preserves_source`
- `channel_session_history_foreign_anchors_preserve_cache_and_binding`
- `channel_session_history_reset_requires_confirmation_and_reloads_cache`
- `channel_session_history_stale_confirmation_is_rejected`
- `channel_session_history_cross_scope_and_busy_are_rejected`
- `channel_session_history_rejects_forged_binding_across_projects_and_tenants`
- `channel_session_history_switching_sessions_invalidates_confirmation`
- `channel_session_history_part_changes_invalidate_confirmation`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
