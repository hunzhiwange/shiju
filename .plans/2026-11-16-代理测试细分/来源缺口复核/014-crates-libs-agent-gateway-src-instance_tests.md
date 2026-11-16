# 来源复核014：agent-gateway/src/instance_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/agent-gateway/src/instance_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/agent-gateway/src/instance_tests.rs`。
- 快照 SHA256：`29feeee40803866f859c0bd4f7e3721e2f4caa40b4f0fe853ae10e60e488eab0`。源码变化后重新阅读。
- 快照词法属性 24 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/instance.rs:13 | crates/libs/agent-gateway/src/instance_tests.rs | mod instance_tests | cfg(not(target_arch="wasm32")); cfg(test) | agent-gateway（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `instance_query_deserializes_explicit_and_missing_directory`
- `resolve_directory_prefers_query_over_header`
- `resolve_directory_preserves_non_empty_query_value`
- `resolve_directory_uses_header_when_query_is_blank`
- `resolve_directory_ignores_blank_header`
- `resolve_directory_ignores_invalid_header_value`
- `resolve_directory_uses_current_dir_without_query_or_header`
- `display_directory_falls_back_to_dot_when_current_dir_fails`
- `display_directory_returns_supplied_path`
- `with_instance_runs_callback_inside_requested_directory`
- `with_instance_returns_callback_error`
- `with_instance_ignores_legacy_invalid_project_json`
- `with_instance_returns_callback_error_when_legacy_project_json_is_invalid`
- `instance_init_noops_without_project_context`
- `load_local_project_rejects_blank_project_id`
- `load_local_project_returns_not_found_for_missing_project`
- `load_local_project_returns_stored_project`
- `normalize_rel_path_handles_empty_root_and_dot`
- `normalize_rel_path_strips_root_from_absolute_path`
- `normalize_rel_path_maps_absolute_root_to_empty_path`
- `normalize_rel_path_keeps_relative_path_without_leading_slashes`
- `normalize_rel_path_rejects_absolute_path_outside_root`
- `normalize_rel_path_rejects_sibling_with_shared_text_prefix`
- `normalize_rel_path_normalizes_backslashes_after_stripping_root`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
