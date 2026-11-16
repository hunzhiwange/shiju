# 来源复核158：src/channels/config_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/channels/config_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/channels/config_tests.rs`。
- 快照 SHA256：`8c8a81cc49ccd05e43e6001ce24e30bfb3f5988bdbda5430341355e08ce73628`。源码变化后重新阅读。
- 快照词法属性 21 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/config.rs:30 | crates/vw-agent/src/channels/config_tests.rs | mod config_tests | cfg(test) | model_types | 未匹配→133 | 未发现 |

## 身份线索

- `runtime_defaults_preserve_configured_values`
- `runtime_defaults_prefer_complete_main_agent_model_binding`
- `runtime_defaults_ignore_incomplete_main_agent_model_binding`
- `runtime_defaults_use_builtin_provider_and_model_when_unset`
- `resolved_defaults_use_hardcoded_fallbacks_when_fields_are_none`
- `runtime_autonomy_policy_preserves_all_runtime_fields`
- `timeout_budget_saturates_on_large_values`
- `runtime_config_path_uses_vibewindow_dir_when_present`
- `runtime_defaults_snapshot_uses_context_without_cached_config`
- `runtime_defaults_snapshot_uses_context_when_config_path_is_not_cached`
- `runtime_defaults_snapshot_prefers_cached_config_state`
- `snapshot_non_cli_excluded_tools_clones_current_list`
- `decrypt_optional_secret_for_runtime_reload_handles_all_value_shapes`
- `config_file_stamp_reports_existing_files_only`
- `load_runtime_defaults_reports_read_and_parse_errors`
- `load_runtime_defaults_reads_toml_config_and_policy`
- `maybe_apply_runtime_config_update_skips_missing_paths_and_unchanged_stamp`
- `maybe_apply_runtime_config_update_returns_load_errors_for_changed_files`
- `maybe_apply_runtime_config_update_applies_changed_config_to_runtime_state`
- `maybe_apply_runtime_config_update_does_not_mutate_state_when_provider_creation_fails`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
