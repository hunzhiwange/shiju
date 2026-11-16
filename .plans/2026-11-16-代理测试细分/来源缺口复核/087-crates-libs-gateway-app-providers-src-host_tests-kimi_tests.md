# 来源复核087：gateway-app-providers/src/host_tests/kimi_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-app-providers/src/host_tests/kimi_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-app-providers/src/host_tests/kimi_tests.rs`。
- 快照 SHA256：`77f5d287b022a800109797fa4ebb31fe7a3d12b9a35f5fc29371ff03e03aa8c4`。源码变化后重新阅读。
- 快照词法属性 9 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/app_providers.rs:234 | crates/libs/gateway-app-providers/src/host_tests/kimi_tests.rs | mod kimi_tests | cfg(not(target_arch="wasm32")); cfg(all(test,target_os="macos")) | gateway-app-providers（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `cc04_kimi_sdk_api_member_edit_remove_and_catalog_delete`
- `cc04_kimi_import_is_read_only_and_does_not_overwrite_catalog`
- `cc04_kimi_permission_tenant_and_host_denials_leave_no_changes`
- `cc04_kimi_first_activation_and_reference_denials_are_atomic`
- `cc04_kimi_existing_official_connection_is_consumed_and_never_forged`
- `cc04_kimi_editor_is_checked_by_server_and_import_never_overwrites`
- `cc04_ui_kimi_form_roundtrip_preserves_credentials_and_other_nodes`
- `cc04_ui_kimi_form_invalid_denied_and_redirect_leave_no_side_effects`
- `cc04_kimi_bound_official_connection_survives_gateway_edits`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
