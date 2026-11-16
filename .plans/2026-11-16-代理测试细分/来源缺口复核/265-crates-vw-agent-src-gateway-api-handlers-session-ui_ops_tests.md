# 来源复核265：src/gateway/api/handlers/session/ui_ops_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs`。
- 快照 SHA256：`f07fe2fb6e96e7859fa467ec57caf37fabd4d8144dde692d97ec5b383415bb04`。源码变化后重新阅读。
- 快照词法属性 26 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/session/ui_ops.rs:353 | crates/vw-agent/src/gateway/api/handlers/session/ui_ops_tests.rs | mod ui_ops_tests | cfg(not(target_arch="wasm32")); cfg(test) | axum, rusqlite, tempfile, vw_api_session, vw_api_todo, session_info | 未匹配→133 | 未发现 |

## 身份线索

- `session_ui_summary_omits_message_payload`
- `session_ui_summary_refreshes_a_stale_delta_before_first_response`
- `session_ui_messages_enforces_page_limit`
- `session_ui_messages_rejects_foreign_cursor`
- `session_ui_message_delta_rejects_invalid_and_missing_targets`
- `session_ui_message_delta_rejects_stale_version`
- `ui_handlers_are_available`
- `session_step_llm_raw_get_isolates_tenants`
- `session_step_llm_raw_get_maps_storage_error_to_internal_error`
- `session_ui_save_propagates_session_store_failure`
- `ui_content_save_preserves_runtime_calls_and_steps`
- `ui_snapshot_handlers_return_empty_results_for_unknown_session`
- `session_archived_put_empty_list_clears_scope`
- `session_ui_get_maps_invalid_json_llm_raw_to_internal_error`
- `session_ui_save_returns_conflict_for_stale_content`
- `agent_unified_selection_summary_projects_server_owned_identity`
- `gateway_first_response_ui_save_cannot_publish_a_new_session`
- `session_history_ui_save_rejects_newer_incomplete_content_with_conflict`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
