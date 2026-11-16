# 来源复核250：src/gateway/api/handlers/integrations_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/integrations_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/integrations_tests.rs`。
- 快照 SHA256：`13a10280a2710a0e89d6417dba733e9f859433381d002ad208e06b72fee41f62`。源码变化后重新阅读。
- 快照词法属性 15 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/integrations.rs:46 | crates/vw-agent/src/gateway/api/handlers/integrations_tests.rs | mod integrations_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, tempfile, tokio | 未匹配→133 | 未发现 |

## 身份线索

- `exported_handlers_are_available`
- `integrations_requires_auth_when_pairing_enabled`
- `integration_settings_requires_auth_when_pairing_enabled`
- `integrations_lists_registry_entries_with_status`
- `integration_settings_include_revision_and_active_provider`
- `integration_credentials_put_requires_auth_before_processing_body`
- `integration_credentials_put_rejects_stale_revision`
- `integration_credentials_put_maps_unknown_integration_to_not_found`
- `integration_credentials_put_maps_unsupported_api_url_to_internal_server`
- `integration_credentials_put_maps_unknown_field_to_bad_request`
- `integration_credentials_put_maps_invalid_update_to_bad_request`
- `integration_credentials_put_reports_unchanged_when_revision_is_same`
- `integration_credentials_put_saves_and_updates_state`
- `integration_credentials_put_accepts_valid_bearer_token`
- `integration_credentials_put_reports_config_persistence_failure`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
