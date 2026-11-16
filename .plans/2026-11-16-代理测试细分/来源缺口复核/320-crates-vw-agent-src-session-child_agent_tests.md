# 来源复核320：src/session/child_agent_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/session/child_agent_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/session/child_agent_tests.rs`。
- 快照 SHA256：`c4218a4b8b79bccb152035c13e49e19e8b5e821e709aae557a020f4315dc91ae`。源码变化后重新阅读。
- 快照词法属性 14 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/mod.rs:14 | crates/vw-agent/src/session/child_agent_tests.rs | mod child_agent_tests | cfg(test) | serde_json, tokio, model_types | 未匹配→133 | 未发现 |

## 身份线索

- `child_agent_stream_request_keeps_complete_model_reference`
- `session_protocol_child_agent_uses_same_contract`
- `child_agent_run_creates_persistent_child_and_waits`
- `child_first_response_gateway_loads_workspace_agents_and_publishes`
- `child_agent_handoff_execution_returns_last_visible_result`
- `child_agent_handoff_plain_text_is_preserved`
- `child_agent_handoff_research_returns_sources_and_persists_full_stream`
- `child_agent_failure_persists_child_history`
- `agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate`
- `agcollab_child_rejects_cross_workspace_and_tenant`
- `agcollab_stored_history_and_permissions_are_snapshotted`
- `agcollab_child_request_has_real_identity_and_untrusted_message_guidance`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
