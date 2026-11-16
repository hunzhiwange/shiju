# 来源复核395：src/workflow/video_edit/cloud_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/workflow/video_edit/cloud_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/workflow/video_edit/cloud_tests.rs`。
- 快照 SHA256：`4d066173ca444726f1bdb896ce44ca608927d86717589f9da26974cb8197e8bc`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/workflow/video_edit/mod.rs:12 | crates/vw-agent/src/workflow/video_edit/cloud_tests.rs | mod cloud_tests | cfg(not(target_arch="wasm32")); cfg(not(target_arch="wasm32")); cfg(test) | serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `video_edit_cloud_transport_hidden_skill_is_rejected`
- `video_edit_cloud_transport_discovers_current_instance_and_full_private_receipt`
- `video_edit_cloud_transport_version_source_and_authorization_fail_closed`
- `video_edit_cloud_transport_missing_credentials_never_logs_in_or_falls_back`
- `video_edit_cloud_transport_native_tool_and_path_denials_prevent_process`
- `video_edit_cloud_transport_cancel_before_discovery_starts_no_process`
- `video_edit_cloud_transport_errors_and_unknown_states_are_not_success`
- `video_edit_cloud_transport_duplicate_visible_instances_are_ambiguous`
- `video_edit_cloud_transport_no_project_uses_native_workspace`
- `video_edit_cloud_transport_inflight_cancel_stops_process_before_private_cleanup`
- `video_edit_cloud_transport_native_approval_denial_prevents_process`
- `video_edit_cloud_transport_native_hooks_never_receive_private_payloads`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
