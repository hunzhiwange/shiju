# 来源复核262：src/gateway/api/handlers/session/session_ops_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs`。
- 快照 SHA256：`763f9a06cf01ce5195c6cffe0382ea622cafa6bb4bfb9fd42e43aeb34e212298`。源码变化后重新阅读。
- 快照词法属性 11 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/session/session_ops.rs:176 | crates/vw-agent/src/gateway/api/handlers/session/session_ops_tests.rs | mod session_ops_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, tempfile, tokio, vw_api_session, vw_api_todo | 未匹配→133 | 未发现 |

## 身份线索

- `session_handlers_are_available`
- `task_session_summary_needs_refresh_only_for_empty_task_board_summaries`
- `refresh_task_session_summaries_preserves_summary_when_refresh_fails`
- `gateway_first_response_create_without_title_reserves_an_unpublished_session`
- `ui_session_list_respects_updated_boundary_and_zero_limit`
- `ui_session_status_preserves_existing_non_idle_state`
- `ui_session_patch_ignores_empty_fields_and_uses_header_scope`
- `session_todo_put_replaces_and_clears_saved_items`
- `scoped_mutations_do_not_modify_sessions_from_another_instance`
- `session_ops_cover_lifecycle_filters_and_instance_boundaries`
- `session_ops_cover_diff_summary_reset_and_title_errors`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
