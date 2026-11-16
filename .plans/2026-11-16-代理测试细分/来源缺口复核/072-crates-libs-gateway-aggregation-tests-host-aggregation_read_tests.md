# 来源复核072：gateway-aggregation/tests/host/aggregation_read_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-aggregation/tests/host/aggregation_read_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-aggregation/tests/host/aggregation_read_tests.rs`。
- 快照 SHA256：`0a9cdf5a742c58a72c81ab7852290b66e13f623288f9c7f5a96ccc442868ce03`。源码变化后重新阅读。
- 快照词法属性 10 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/aggregation_read_tests.rs:2 | crates/libs/gateway-aggregation/tests/host/aggregation_read_tests.rs | include! | cfg(not(target_arch="wasm32")); cfg(all(test,target_os="macos")) | gateway-aggregation（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `aggregation_t39_aggregation_read_preserves_contract`
- `aggregation_t39_denies_missing_credentials_host_and_real_foreign_project`
- `aggregation_t39_unclaimed_state_and_pending_are_conflicts_without_recovery`
- `aggregation_t39_preview_blocks_invalid_default_and_failover_without_issuing_token`
- `aggregation_t39_revision_changes_with_native_inputs_and_rejects_unsafe_paths`
- `aggregation_t39_rejects_unknown_preview_fields_and_non_codex_catalog_policy`
- `aggregation_t39_codex_preview_uses_saved_profile_without_publishing_catalog`
- `aggregation_t39_foreign_tenant_state_is_forbidden_before_preview`
- `aggregation_t39_corrupt_native_configuration_cannot_receive_preview_token`
- `aggregation_t39_desktop_preview_uses_native_mapping_without_creating_profile`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
