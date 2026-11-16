# 来源复核106：gateway-pty/src/pty_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-pty/src/pty_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-pty/src/pty_tests.rs`。
- 快照 SHA256：`ceb963ead811297b10625a8a33227f3f968126f01c8ce4d29b2a9a72505577eb`。源码变化后重新阅读。
- 快照词法属性 10 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/pty.rs:36 | crates/libs/gateway-pty/src/pty_tests.rs | mod pty_tests | cfg(not(target_arch="wasm32")); cfg(test) | gateway-pty（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `router_builds_with_app_state`
- `pty_connect_query_deserializes_directory_and_cursor`
- `pty_connect_query_allows_missing_optional_fields`
- `pty_connect_query_rejects_non_integer_cursor`
- `pty_connect_query_preserves_cursor_boundaries_and_blank_directory`
- `pty_list_returns_empty_for_new_instance`
- `pty_create_rejects_blank_cwd_without_starting_process`
- `pty_get_missing_session_returns_not_found`
- `pty_update_missing_session_returns_not_found`
- `pty_remove_missing_session_is_idempotent`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
