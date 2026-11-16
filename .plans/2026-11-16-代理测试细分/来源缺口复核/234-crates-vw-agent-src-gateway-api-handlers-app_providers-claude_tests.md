# 来源复核234：src/gateway/api/handlers/app_providers/claude_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs`。
- 快照 SHA256：`b44766cc2a818698a09b94224b7593153feb681ac993ba4ceddaaa5c310629b4`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/app_providers.rs:172 | crates/vw-agent/src/gateway/api/handlers/app_providers/claude_tests.rs | mod claude_tests | cfg(not(target_arch="wasm32")); cfg(all(test,target_os="macos")) | axum, serde_json, sha2, vw_api_app_identity, vw_api_app_provider_base, vw_api_transport, vw_api_app_provider_environment, gateway_http, vw_api_app_provider_edit | 未匹配→133 | 未发现 |

## 身份线索

- `cc04_ui_paths_saved_directory_is_used_by_native_activation_and_import`
- `cc04_claude_sdk_api_switches_four_variants_and_cleans_credentials`
- `cc04_claude_current_edit_reapply_import_and_inactive_edit_share_adapter`
- `cc04_claude_denied_and_invalid_requests_leave_database_and_files_untouched`
- `cc04_claude_sync_projects_existing_universal_owner_and_applies_current_child`
- `cc04_claude_live_failure_reports_saved_catalog_and_reapply_repairs_it`
- `cc04_claude_legacy_path_first_activation_and_symlink_rejection`
- `cc04_claude_registered_route_requires_gateway_authentication`
- `cc04_claude_first_activation_creates_missing_config_and_catalog_defaults_do_not`
- `cc04_claude_committed_exclusives_survive_catalog_edits_until_switch_cleans_them`
- `cc04_ui_claude_sdk_round_trip_projects_key_fields_on_the_authorized_host`
- `cc04_ui_claude_sdk_validation_and_permission_failures_do_not_write`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
