# 来源复核078：gateway-app-providers/src/host_tests/editor_merge_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/gateway-app-providers/src/host_tests/editor_merge_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/gateway-app-providers/src/host_tests/editor_merge_tests.rs`。
- 快照 SHA256：`f724c5994e56ede7f5d0682389fbaa09dc6f435d4338d16bec9c3313bbbdf8d3`。源码变化后重新阅读。
- 快照词法属性 8 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/app_providers.rs:162 | crates/libs/gateway-app-providers/src/host_tests/editor_merge_tests.rs | mod editor_merge_tests | cfg(not(target_arch="wasm32")); cfg(all(test,target_os="macos")) | gateway-app-providers（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `cc04_editor_merge_sdk_api_keeps_external_fields_and_requires_fresh_choice`
- `cc04_editor_merge_inactive_is_catalog_only_and_permission_denial_has_no_effect`
- `cc04_editor_merge_keep_theirs_saves_only_other_user_edits`
- `cc04_editor_merge_rejects_wrong_binding_stale_record_and_invalid_projection`
- `cc04_editor_merge_symlink_and_state_change_are_rejected`
- `cc04_editor_merge_registered_route_requires_gateway_authentication`
- `cc04_ui_preview_editor_operation_keeps_file_when_cancelled_and_saves_after_retry`
- `cc04_ui_preview_saved_fallback_is_redacted_authorized_and_read_only`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
