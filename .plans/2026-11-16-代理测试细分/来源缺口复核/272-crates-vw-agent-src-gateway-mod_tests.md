# 来源复核272：src/gateway/mod_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/mod_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/mod_tests.rs`。
- 快照 SHA256：`5dac2e0702a0cd0dd967321f4cf2209d9b671e683b3bc517ed32c48e81782bbb`。源码变化后重新阅读。
- 快照词法属性 18 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/mod.rs:43 | crates/vw-agent/src/gateway/mod_tests.rs | mod mod_tests | cfg(not(target_arch="wasm32")); cfg(test) | model_types, axum, parking_lot, tower | 未匹配→133 | 未发现 |

## 身份线索

- `workflow_chat_messages_path_matches_collection_and_application_routes`
- `workflow_chat_messages_path_rejects_empty_or_partial_matches`
- `timeout_for_path_uses_extended_window_for_long_running_sse`
- `run_gateway_rejects_public_bind_without_tunnel_or_opt_in`
- `bootstrap_router_registers_v1_agent_scheduler_project_routes`
- `bootstrap_router_rejects_client_routes_without_tenant_headers`
- `bootstrap_hooks_routes_remain_behind_gateway_authentication`
- `bootstrap_router_keeps_v1_health_and_webhook_outside_tenant_middleware`
- `request_timeout_middleware_returns_inner_response_before_deadline`
- `request_timeout_middleware_returns_timeout_after_deadline`
- `request_timeout_middleware_preserves_workflow_response_past_default_deadline`
- `request_timeout_middleware_times_out_workflow_after_extended_deadline`
- `public_skey_auth_path_matches_only_global_health_routes`
- `skey_auth_middleware_allows_options_and_public_paths_without_token`
- `skey_auth_middleware_rejects_public_path_lookalikes_without_token`
- `skey_auth_middleware_accepts_valid_skey`
- `skey_auth_middleware_rejects_missing_or_invalid_skey`
- `acp_install_timeout_covers_installer_deadline_only_for_single_agent_routes`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
