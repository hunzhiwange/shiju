# 来源复核103：gateway-instance/src/instance_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-instance/src/instance_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-instance/src/instance_tests.rs`。
- 快照 SHA256：`e38ea4b1fec3e794cefe5231b6f49c65943aed684f834bee42cde17a91f96ba0`。源码变化后重新阅读。
- 快照词法属性 20 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/instance.rs:40 | crates/libs/gateway-instance/src/instance_tests.rs | mod instance_tests | cfg(not(target_arch="wasm32")); cfg(test) | gateway-instance（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `router_builds_with_app_state`
- `response_structs_serialize_stable_api_shapes`
- `router_path_get_prefers_query_directory_over_header`
- `router_vcs_get_uses_directory_header_when_query_missing`
- `router_instance_dispose_accepts_post_only`
- `path_get_reports_global_and_instance_paths`
- `path_get_uses_directory_header_when_query_missing`
- `branch_for_worktree_returns_empty_for_blank_worktree`
- `branch_for_worktree_returns_empty_when_directory_is_missing`
- `vcs_get_returns_empty_branch_for_non_git_directory`
- `vcs_get_reports_current_git_branch`
- `instance_event_sse_builds_event_stream_response`
- `instance_event_sse_sends_connected_event`
- `instance_event_sse_emits_initial_and_periodic_heartbeat_events`
- `instance_event_sse_filters_events_by_instance_directory`
- `instance_event_sse_forwards_events_without_directory_scope`
- `instance_event_sse_stops_when_receiver_is_dropped`
- `instance_dispose_returns_true_and_clears_cached_instance`
- `instance_dispose_clears_directory_approval_manager`
- `instance_dispose_publishes_disposed_event_for_directory`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
