# 来源复核248：src/gateway/api/handlers/global_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/global_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/global_tests.rs`。
- 快照 SHA256：`ee04d2c093dd6e05bb443ace3983e8c9f9ef23f1baf5e638312ea9e3aefe7418`。源码变化后重新阅读。
- 快照词法属性 24 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/global.rs:26 | crates/vw-agent/src/gateway/api/handlers/global_tests.rs | mod global_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, parking_lot, tempfile, tokio_stream, tower, vw_api_transport, serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `router_builds_with_app_state`
- `global_config_routes_isolate_platform_and_company_updates`
- `local_global_config_route_writes_shared_database_only`
- `global_config_api_projects_shared_skeys_and_ignores_conflicting_legacy_sqlite`
- `gateway_bootstrap_loads_default_tenant_skeys_from_shared_store_without_legacy_fallback`
- `global_health_reports_ok_status`
- `global_config_get_serializes_current_config`
- `global_config_get_maps_unusable_config_directory_to_bad_request`
- `config_response_maps_secret_encryption_failure_to_internal_error`
- `global_config_acp_get_returns_merged_agent_specs`
- `global_config_acp_get_trims_and_filters_user_overrides`
- `global_config_patch_accepts_object_patch`
- `global_config_patch_preserves_mcp_servers`
- `global_config_patch_rejects_non_object_patch`
- `global_config_patch_stateful_updates_cached_config_and_pairing`
- `global_config_patch_stateful_rejects_invalid_patch_without_runtime_update`
- `global_config_patch_stateful_concurrent_fields_preserve_both_values`
- `global_dispose_returns_true_and_publishes_global_event`
- `global_event_sse_response_uses_event_stream_content_type`
- `global_event_sse_streams_connected_and_published_events`
- `global_event_sse_preserves_published_event_directory`
- `global_event_sse_streams_heartbeat_and_stops_after_receiver_closes`
- `agent_unified_selection_install_status_uses_gateway_executable_without_download`
- `global_acp_defaults_route_returns_builtin_launch_fields`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
