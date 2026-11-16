# 任务36证据

代码完成，编译/行为未验证。遵照用户要求，未运行任何 Cargo、构建、单元或实机测试；仅源码、注册、依赖和差异静态核对。

## 归属和边界

生产实现此前已下沉，不计为本轮生产迁移收益。删除无调用的兼容转发/组合，测试由 agent-plugins 自注册 host_migration_36_tests；不反向依赖 vw-agent，也不 path/include 宿主 Rust 源码。夹具继续读取原 testdata，证据汇总调用原 Python 脚本。

技能测试使用真实 agent_skills::catalog 合并与 agent_plugins::skills 发现、真实 tool_services::skill 调用及 session_state 回放；测试端口只注入无活动会话的文件发现与默认代理，未知审批/钩子调用 panic。禁用测试无活动 MCP 会话，关闭回调为空；真实宿主撤销保留原实现。

## 全文件与用例映射

### agents.rs

6 → 6 行；宿主保留。

load_metadata 注入现有路径审计，dependencies 的直接调用者。


### closure_tests.rs

23 → 0 行；agent-plugins/src/host_migration_36_tests/closure_tests.rs。


### commands.rs

10 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### discovery.rs

6 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### git_source.rs

16 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### host_files.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### identity.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### installed_state.rs

13 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### legacy_manifest.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### marketplace.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### npm_archive.rs

2 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### npm_source.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### onboarding.rs

6 → 6 行；宿主保留。

entry 注入宿主可信运行时技能库存。


### path_audit.rs

2 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### portable_manifest.rs

2 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### skill_frontmatter.rs

2 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### skill_invocation.rs

2 → 2 行；宿主保留。

供宿主现有调用者使用的薄领域重导出，无领域业务分支。


### skill_roots.rs

5 → 5 行；宿主保留。

skill_roots/package_format/skill_files 供 dependencies 直接调用。


### t09_tests.rs

76 → 0 行；agent-plugins/src/host_migration_36_tests/t09_tests.rs。

- `plugins::compat::t09_tests::codex_plugin_compat_t09_preserves_same_name_marketplace_provenance_in_runtime_merge` → `host_migration_36_tests::t09_tests::codex_plugin_compat_t09_preserves_same_name_marketplace_provenance_in_runtime_merge`，原断言保留。

### t10_tests.rs

370 → 0 行；agent-plugins/src/host_migration_36_tests/t10_tests.rs。

- `plugins::compat::t10_tests::codex_plugin_compat_t10_invokes_namespaced_skill_with_arguments` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_invokes_namespaced_skill_with_arguments`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_rejects_invalid_input` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_rejects_invalid_input`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_preserves_isolation` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_preserves_isolation`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_rejects_ambiguous_namespace_and_keeps_unique_alias` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_rejects_ambiguous_namespace_and_keeps_unique_alias`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_uses_exact_plugin_links_without_display_name_authorization` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_uses_exact_plugin_links_without_display_name_authorization`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_preserves_delegate_identity_boundaries` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_preserves_delegate_identity_boundaries`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_catalog_and_preloads_preserve_qualified_identity` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_catalog_and_preloads_preserve_qualified_identity`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_relative_resources_and_scripts_use_skill_directory` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_relative_resources_and_scripts_use_skill_directory`，原断言保留。
- `plugins::compat::t10_tests::codex_plugin_compat_t10_preserves_user_invocation_restriction` → `host_migration_36_tests::t10_tests::codex_plugin_compat_t10_preserves_user_invocation_restriction`，原断言保留。

### t11_tests.rs

136 → 0 行；agent-plugins/src/host_migration_36_tests/t11_tests.rs。

- `plugins::compat::t11_tests::codex_plugin_compat_t11_invokes_migrated_command` → `host_migration_36_tests::t11_tests::codex_plugin_compat_t11_invokes_migrated_command`，原断言保留。
- `plugins::compat::t11_tests::codex_plugin_compat_t11_preserves_isolation` → `host_migration_36_tests::t11_tests::codex_plugin_compat_t11_preserves_isolation`，原断言保留。

### t12_tests.rs

111 → 0 行；agent-plugins/src/host_migration_36_tests/t12_tests.rs。

- `plugins::compat::t12_tests::codex_plugin_compat_t12_enforces_policy_in_real_invocation_and_replay_admission` → `host_migration_36_tests::t12_tests::codex_plugin_compat_t12_enforces_policy_in_real_invocation_and_replay_admission`，原断言保留。

### t13_tests.rs

381 → 0 行；agent-plugins/src/host_migration_36_tests/t13_tests.rs。

- `plugins::compat::t13_tests::codex_plugin_compat_t13_invokes_onboarding_only_on_request` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_invokes_onboarding_only_on_request`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_preserves_isolation` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_preserves_isolation`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_reads_both_manifest_positions_and_legacy_alternatives` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_reads_both_manifest_positions_and_legacy_alternatives`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_does_not_add_roots_or_select_name_collision_losers` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_does_not_add_roots_or_select_name_collision_losers`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_preserves_user_invocation_and_security_restrictions` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_preserves_user_invocation_and_security_restrictions`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_keeps_absent_or_replaced_overlay_inert` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_keeps_absent_or_replaced_overlay_inert`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_does_not_borrow_a_shadowing_installation` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_does_not_borrow_a_shadowing_installation`，原断言保留。
- `plugins::compat::t13_tests::codex_plugin_compat_t13_uses_final_skill_admission_after_local_shadowing` → `host_migration_36_tests::t13_tests::codex_plugin_compat_t13_uses_final_skill_admission_after_local_shadowing`，原断言保留。

### t59_tests.rs

135 → 135 行；宿主保留。

四条真实宿主工具注册、执行、跨所有者取消、工作目录与包字节边界。

- `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_exec_arguments_and_cancellation` → `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_exec_arguments_and_cancellation`，原断言保留。
- `plugins::compat::t59_tests::codex_plugin_compat_t59_rejects_removed_exec_command` → `plugins::compat::t59_tests::codex_plugin_compat_t59_rejects_removed_exec_command`，原断言保留。
- `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_isolation` → `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_isolation`，原断言保留。
- `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_workdir_and_package_bytes` → `plugins::compat::t59_tests::codex_plugin_compat_t59_preserves_workdir_and_package_bytes`，原断言保留。

### t60_tests.rs

42 → 42 行；宿主保留。

三条真实宿主权限下 apply_patch 成功、事务失败及目录隔离。

- `plugins::compat::t60_tests::codex_plugin_compat_t60_applies_patch_with_host_permissions` → `plugins::compat::t60_tests::codex_plugin_compat_t60_applies_patch_with_host_permissions`，原断言保留。
- `plugins::compat::t60_tests::codex_plugin_compat_t60_rejects_invalid_input` → `plugins::compat::t60_tests::codex_plugin_compat_t60_rejects_invalid_input`，原断言保留。
- `plugins::compat::t60_tests::codex_plugin_compat_t60_preserves_isolation` → `plugins::compat::t60_tests::codex_plugin_compat_t60_preserves_isolation`，原断言保留。

### t66_tests.rs

74 → 0 行；agent-plugins/src/host_migration_36_tests/t66_tests.rs。

- `plugins::compat::t66_tests::codex_plugin_compat_t66_rejects_missing_in_scope_evidence` → `host_migration_36_tests::t66_tests::codex_plugin_compat_t66_rejects_missing_in_scope_evidence`，原断言保留。
- `plugins::compat::t66_tests::codex_plugin_compat_t66_rejects_invalid_input` → `host_migration_36_tests::t66_tests::codex_plugin_compat_t66_rejects_invalid_input`，原断言保留。
- `plugins::compat::t66_tests::codex_plugin_compat_t66_preserves_isolation` → `host_migration_36_tests::t66_tests::codex_plugin_compat_t66_preserves_isolation`，原断言保留。

### transaction_install.rs

2 → 0 行；删除无调用兼容入口，真实实现仍在 agent-plugins/plugin-install。


### upgrade.rs

5 → 5 行；宿主保留。

activate_plugin_upgrade 注入真实租户 MCP 撤销。


## 行数与未来验证

当前 HEAD 同路径口径：来源 1437 行 → 211 行，净减 1226 行（85.3%）。删除项为领域测试/夹具及无调用转发，不声称此前下沉的生产代码有新增收益。

领域测试/夹具 1232 行，加注册文件 32 行；新增生产实现 0 行。未来领域精确过滤预计 24 条（Unix 包含脚本场景；其它平台少 1）：

`python3 scripts/cargo_slot.py cargo test -p agent-plugins --lib host_migration_36_tests -- --test-threads=1`

未来宿主分别用 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib plugins::compat::t59_tests -- --test-threads=1`（4 条，非 wasm）和同入口 `plugins::compat::t60_tests`（3 条）。列表核对迁出原完整名称消失，不能将零测试视为通过。所有编译、行为和构建收益均未验证。
