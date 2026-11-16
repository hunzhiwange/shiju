# 来源复核128：src/acp/mounts_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/acp/mounts_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/acp/mounts_tests.rs`。
- 快照 SHA256：`59378890ad20d6493f80b41edca8808061396c87711ff0b161fe3f1814f0594f`。源码变化后重新阅读。
- 快照词法属性 7 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/acp/mod.rs:19 | crates/vw-agent/src/acp/mounts_tests.rs | mod mounts_tests | cfg_attr(test,allow(unused_imports)); cfg(test) | acp_server, acp_types, serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `acp_server_skill_overlay_accepts_native_directory`
- `acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access`
- `acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing`
- `acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session`
- `acp_server_skill_overlay_permission_change_revokes_read_and_invocation`
- `acp_server_skill_overlay_retains_native_skill_permission_denial`
- `acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
