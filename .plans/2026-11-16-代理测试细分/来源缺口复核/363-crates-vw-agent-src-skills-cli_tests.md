# 来源复核363：src/skills/cli_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/skills/cli_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/skills/cli_tests.rs`。
- 快照 SHA256：`34a1f2ff0c3f16ee1a9dd24a931e5b16cea046e5b8a34cc90c7e0c30f86d845e`。源码变化后重新阅读。
- 快照词法属性 16 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/skills/cli.rs:171 | crates/vw-agent/src/skills/cli_tests.rs | mod cli_tests | cfg(test) | vw-agent 宿主模块（生产父声明见左列） | 未匹配→133 | 未发现 |

## 身份线索

- `wasm_cli_stub_is_explicitly_unsupported`
- `skills_dir_helper_points_under_workspace`
- `handle_list_covers_empty_and_populated_workspaces`
- `handle_list_loads_manifest_tools_and_tags`
- `handle_audit_reports_success_missing_and_findings`
- `handle_audit_resolves_installed_skill_name`
- `handle_audit_reports_error_for_existing_file_source`
- `handle_remove_removes_existing_skill_and_reports_missing_repeat`
- `handle_remove_reports_missing_when_skills_dir_has_not_been_created`
- `handle_remove_reports_filesystem_error_when_target_is_not_directory`
- `handle_remove_rejects_symlink_that_escapes_skills_directory`
- `handle_install_applies_configured_security_policy`
- `handle_install_rejects_missing_local_source`
- `handle_install_reports_skills_directory_creation_failure`
- `print_install_success_covers_git_and_skills_sh_messages`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
