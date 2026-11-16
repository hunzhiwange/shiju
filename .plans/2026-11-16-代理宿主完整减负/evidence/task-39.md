# 任务39：插件兼容入口与残留装配

代码完成，编译/行为未验证。本轮遵守禁止 Cargo、单元与实机测试的约束；仅做源码、注册、差异核对。

## 实际接线与依赖

agent-plugins 自注册 host_migration_39_tests，下含 cli、skills、install；迁入原断言和临时目录夹具，共 20 条测试。无反向依赖、path/include 宿主源码。复用现有依赖，无新增依赖。

CLI 测试调用真实 agent_plugins::cli::handle_command；技能夹具调用 agent_skills::catalog::load_skills_with_config，插件发现注入真实 discover_plugins。测试本地连接撤销复用 plugin_mcp 的进程连接池，不声称覆盖宿主租户撤销。

生产 install_dir_from_config 原先在宿主与领域 CLI 重复，宿主改为重导出领域函数；领域函数公开供真实宿主入口使用。三个 install_plugin_* 纯转发改为重导出。其余生产能力已在领域，不能计为本轮迁移收益。

## 全文件与全用例映射

### crates/vw-agent/src/plugins/audit.rs
2 → 2 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/cli.rs
8 → 8 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。
保留函数：`load_skills`, `handle_command`, `installation_message`。

### crates/vw-agent/src/plugins/cli_tests.rs
192 → 0 行。整文件迁入 agent-plugins，自注册，宿主文件及挂载删除。
- `plugin_cli_install_copies_local_plugin_after_audit` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_install_copies_local_plugin_after_audit`
- `plugin_cli_enable_disable_toggles_marker` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_enable_disable_toggles_marker`
- `plugin_cli_install_rejects_remote_source_for_now` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_install_rejects_remote_source_for_now`
- `plugin_cli_list_accepts_empty_installation` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_list_accepts_empty_installation`
- `plugin_cli_list_accepts_discovered_plugin` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_list_accepts_discovered_plugin`
- `plugin_cli_audit_rejects_blank_source` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_rejects_blank_source`
- `plugin_cli_audit_accepts_direct_source_and_installed_name` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_accepts_direct_source_and_installed_name`
- `plugin_cli_audit_adds_target_context_for_missing_source` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_adds_target_context_for_missing_source`
- `plugin_cli_remove_deletes_plugin_and_disabled_marker` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_remove_deletes_plugin_and_disabled_marker`

### crates/vw-agent/src/plugins/compat/t29_tests.rs
85 → 85 行。真实宿主会话池、工具执行器、租户/审批、技能调用或平台集成验收保留；继续可迁项：拆出安装生命周期及结果规范化断言归领域，保持真实宿主链路。
- `codex_plugin_compat_t29_reads_resource_over_real_stdio` → 原路径 `crates/vw-agent/src/plugins/compat/t29_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t29_public_chess_resource` → 原路径 `crates/vw-agent/src/plugins/compat/t29_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/compat/t30_tests.rs
125 → 125 行。真实宿主会话池、工具执行器、租户/审批、技能调用或平台集成验收保留；继续可迁项：拆出安装生命周期及结果规范化断言归领域，保持真实宿主链路。
- `codex_plugin_compat_t30_session_binding_uses_real_mcp_and_revokes` → 原路径 `crates/vw-agent/src/plugins/compat/t30_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/compat/t58_tests.rs
69 → 0 行。整文件迁入 agent-plugins，自注册，宿主文件及挂载删除。
- `codex_plugin_compat_t58_reports_missing_runtime_before_execution` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_reports_missing_runtime_before_execution`
- `codex_plugin_compat_t58_rejects_invalid_input` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_rejects_invalid_input`
- `codex_plugin_compat_t58_preserves_isolation` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_preserves_isolation`
- `codex_plugin_compat_t58_recognizes_bundled_python_module_without_importing` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_recognizes_bundled_python_module_without_importing`
- `codex_plugin_compat_t58_rejects_dependency_symlink` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_rejects_dependency_symlink`

### crates/vw-agent/src/plugins/compat/t61_tests.rs
351 → 351 行。真实宿主会话池、工具执行器、租户/审批、技能调用或平台集成验收保留；继续可迁项：拆出安装生命周期及结果规范化断言归领域，保持真实宿主链路。
- `codex_plugin_compat_t61_runs_installed_mcp_side_effect` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_public_skill_lifecycle_and_upgrade` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_public_stdio_original` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_public_marketplace_cli_lifecycle` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_public_hook_original` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_public_http_original` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t61_original_reports_missing_dependency` → 原路径 `crates/vw-agent/src/plugins/compat/t61_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/compat/t62_tests.rs
102 → 102 行。真实宿主会话池、工具执行器、租户/审批、技能调用或平台集成验收保留；继续可迁项：拆出安装生命周期及结果规范化断言归领域，保持真实宿主链路。
- `codex_plugin_compat_t62_calls_actual_public_readonly_service` → 原路径 `crates/vw-agent/src/plugins/compat/t62_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t62_rejects_invalid_input` → 原路径 `crates/vw-agent/src/plugins/compat/t62_tests.rs`（真实宿主边界保留）
- `codex_plugin_compat_t62_preserves_isolation` → 原路径 `crates/vw-agent/src/plugins/compat/t62_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/compat/t64_tests.rs
82 → 82 行。真实宿主会话池、工具执行器、租户/审批、技能调用或平台集成验收保留；继续可迁项：拆出安装生命周期及结果规范化断言归领域，保持真实宿主链路。
- `codex_plugin_compat_t64_uses_platform_command_and_unicode_paths` → 原路径 `crates/vw-agent/src/plugins/compat/t64_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/hook_handler.rs
8 → 8 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。
保留函数：`trusted_plugin_hook_handlers`, `codex_handlers`。

### crates/vw-agent/src/plugins/hook_handler_tests.rs
506 → 506 行。真实宿主 HookHost/HookRunner 绑定、进程执行及日志验收保留；继续可迁项：将处理器非宿主策略断言与真实端口验收拆分，不能复制完整 Host。
- `trusted_plugin_before_tool_call_can_cancel` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_after_tool_call_receives_json_payload` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_before_tool_call_reports_spawn_failure` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_before_tool_call_reports_timeout` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_before_tool_call_redacts_and_truncates_stdout_diagnostic` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_untrusted_hook_is_not_registered` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `plugin_hook_manifest_load_failure_reports_plugin_and_path_context` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `untrusted_plugin_hook_skip_does_not_warn` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `disabled_plugin_hooks_skip_discovery` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_unsupported_plugin_hook_is_not_registered` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）
- `trusted_plugin_before_llm_call_rejects_before_model_request` → 原路径 `crates/vw-agent/src/plugins/hook_handler_tests.rs`（真实宿主边界保留）

### crates/vw-agent/src/plugins/hook_trust.rs
2 → 2 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/hooks.rs
2 → 2 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/install.rs
32 → 14 行。只留连接撤销的具体宿主组合；目录投影与安装能力重导出领域。
保留函数：`set_plugin_enabled`, `remove_installed_plugin`。

### crates/vw-agent/src/plugins/install_tests.rs
12 → 0 行。整文件迁入 agent-plugins，自注册，宿主文件及挂载删除。
- `plugin_install_dir_uses_config_parent` → `agent_plugins::host_migration_39_tests::install::plugin_install_dir_uses_config_parent`

### crates/vw-agent/src/plugins/loader.rs
2 → 2 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/manifest.rs
2 → 2 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/mcp.rs
17 → 17 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。
保留函数：`merge_enabled_plugin_mcp_servers`, `load_plugin_mcp_servers_from_path`。

### crates/vw-agent/src/plugins/mod.rs
150 → 138 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

### crates/vw-agent/src/plugins/skills.rs
14 → 14 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。
保留函数：`load_plugin_skills`, `load_plugin_skill_inventory`。

### crates/vw-agent/src/plugins/skills_tests.rs
199 → 0 行。整文件迁入 agent-plugins，自注册，宿主文件及挂载删除。
- `plugin_skills_are_loaded_from_manifest_component_path` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_are_loaded_from_manifest_component_path`
- `plugin_skills_follow_configured_security_policy` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_follow_configured_security_policy`
- `plugin_skills_workspace_skill_overrides_plugin_skill` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_workspace_skill_overrides_plugin_skill`
- `plugin_skills_disabled_plugin_skills_are_skipped` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_disabled_plugin_skills_are_skipped`
- `plugin_skills_invalid_plugin_source_returns_empty` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_invalid_plugin_source_returns_empty`

### crates/vw-agent/src/plugins/types.rs
6 → 6 行。宿主兼容模块路径或实际组合端口保留；领域实现已存在，本轮不计迁移收益。

依赖测试夹具组合 plugin_install::dependencies::inspect_plugin_dependencies 的既有输入端口，复用真实审计与声明加载，不另写探测算法。宿主 dependencies.rs 组合保持原样（其它任务所有），未计生产收益。

## 行数与减量门槛

当前读取的22个来源文件：1968 → 1466 行，净减 502 行（25.5%），未达到70%。按文件名粗分：生产/转发 245 → 215，测试 1723 → 1251；转发、注释及注册不冒充业务算法。保留逐文件原因与继续可迁项见上。由收口任务继续验收70%门槛，不宣称本组达标。

本轮新增领域测试含夹具与注册行数：510。领域生产 CLI 仅改函数可见性，生产算法新增0；宿主目录算法删除，使用领域同一实现。

任务1 external-mounts.json 中未发现归属本任务的宿主外测试挂载；本组旧 cli/skills/install 模块注册已删除，Hook 和 compat 注册仍为真实宿主验收。

## 未来验证

`python3 scripts/cargo_slot.py cargo test -p agent-plugins --lib host_migration_39_tests -- --test-threads=1`：预期20条（CLI 9、技能5、目录1、依赖5），必须确认非零且全部通过。

宿主保留用例使用上列完整函数名逐项过滤：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <完整函数名> -- --exact --test-threads=1`（执行前补齐 plugins 模块全限定名）；ignore 场景需显式 --ignored 并满足网络、平台与 PATH 条件。未来先以 --list 证明迁出20条旧宿主用例消失，再检查保留边界非零，零测试不算通过。

全部 Cargo、编译、行为、网络、性能检查均未运行，未声称编译收益。静态差异检查无空白错误。
