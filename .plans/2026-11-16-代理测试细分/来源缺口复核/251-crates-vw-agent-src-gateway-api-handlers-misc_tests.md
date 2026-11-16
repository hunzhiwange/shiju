# 来源复核251：src/gateway/api/handlers/misc_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/gateway/api/handlers/misc_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/gateway/api/handlers/misc_tests.rs`。
- 快照 SHA256：`d9fe7839069ae630de017646be387a59189422b5caa7c5b2dc84ab602ed558fe`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/gateway/api/handlers/misc.rs:53 | crates/vw-agent/src/gateway/api/handlers/misc_tests.rs | mod misc_tests | cfg(not(target_arch="wasm32")); cfg(test) | axum, tower | 未匹配→133 | 未发现 |

## 身份线索

- `router_builds_with_unit_state`
- `router_dispatches_all_get_routes`
- `router_rejects_non_get_methods_and_unknown_paths`
- `not_implemented_returns_501`
- `command_list_returns_named_command_metadata`
- `agent_list_returns_keyed_agent_metadata`
- `skill_list_returns_named_and_located_skill_metadata`
- `skill_info_serializes_all_public_response_fields`
- `list_handlers_ignore_legacy_invalid_project_json`
- `agent_profile_api_catalog_includes_disabled_only_when_requested`
- `agent_profile_api_copy_has_unique_custom_identity_and_survives_reload`
- `agent_config_cleanup_catalog_copy_is_global_and_tenant_isolated`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
