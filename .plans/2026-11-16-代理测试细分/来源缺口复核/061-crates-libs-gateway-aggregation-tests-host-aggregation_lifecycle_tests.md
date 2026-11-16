# 来源复核061：gateway-aggregation/tests/host/aggregation_lifecycle_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs`。
- 快照 SHA256：`33cbf1e3bf4315850d049a69f729076cdad29dcd3c39dfb882b7c4a55a4c67ea`。源码变化后重新阅读。
- 快照词法属性 9 个，不是当前 cfg 展开后的有效测试数。

| crates/libs/gateway-aggregation/tests/host/aggregation_publish_tests.rs:445 | crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs | mod lifecycle_tests | cfg(not(target_arch="wasm32")); cfg(all(test,target_os="macos")) | gateway-aggregation（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `aggregation_t19_aggregation_lifecycle_preserves_contract`
- `aggregation_t19_restart_rotates_credentials_and_repeated_events_preserve_files`
- `aggregation_t19_external_edit_and_revoked_permission_fail_without_overwrite`
- `aggregation_t19_gateway_owner_shutdown_restores_its_client`
- `aggregation_t19_codex_restores_selected_provider_and_reconnects_catalog`
- `aggregation_t19_desktop_keeps_projection_then_republishes_on_start`
- `aggregation_t19_service_stop_preserves_session_relay_capability_and_listener`
- `aggregation_t19_host_stop_and_start_use_authorized_router`
- `aggregation_t19_failed_start_preserves_direct_files_and_retryable_intent`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
