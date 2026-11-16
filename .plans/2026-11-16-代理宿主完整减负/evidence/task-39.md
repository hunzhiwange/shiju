# 任务39 深迁证据（替代首轮保留结论）

代码与静态接线完成；禁止 Cargo、构建与运行测试，编译和行为未验证。读取 final.md/remaining-host.json，任务39首轮残留1466行。本轮不将真实端口依赖作为整组保留理由。

## 真实入口和边界

生产实现原本已在 agent-plugins/plugin-mcp/plugin-hooks；本轮无新增生产迁移收益。复用 task38 的 fail-closed Host（仅测试可见性扩大），复用 task37 的真实协议连接 Host；不复制 App、审批或宿主状态。
Hook 原断言移至 hooks.rs，真正执行脚本、信任摘要、超时、取消、载荷和脱敏；日志目标改为实际 agent_plugins::hook_handler。宿主仅新增 plugin_hook_host_registers_and_cancels，用实际宿主 Host 和 runner 验证注册及取消。
t29 原协议断言整体迁入，真实 stdio/HTTP 服务和资源不变。t61/t62 将安装、源哈希、协议发现、调用响应归领域；改为实际 pool.with_peer -> peer.call_tool -> 生产 normalize_mcp_tool_result。原 model_text、文件副作用和隔离断言保留。实际 McpTool 适配器的调用仍由宿主 t64 和 t30 覆盖；迁入用例不冒充审批/租户验收。
CLI、依赖和技能沿用首轮同一生产领域服务。无反向依赖或宿主源码挂载。原 source fixtures 只作资源引用。现有依赖足够，未新增依赖。

## 全文件和原用例映射

### crates/vw-agent/src/plugins/audit.rs
基线 2 → 当前 2 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/cli.rs
基线 8 → 当前 8 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/cli_tests.rs
基线 192 → 当前 0 行。
- `plugin_cli_install_copies_local_plugin_after_audit` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_install_copies_local_plugin_after_audit`。
- `plugin_cli_enable_disable_toggles_marker` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_enable_disable_toggles_marker`。
- `plugin_cli_install_rejects_remote_source_for_now` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_install_rejects_remote_source_for_now`。
- `plugin_cli_list_accepts_empty_installation` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_list_accepts_empty_installation`。
- `plugin_cli_list_accepts_discovered_plugin` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_list_accepts_discovered_plugin`。
- `plugin_cli_audit_rejects_blank_source` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_rejects_blank_source`。
- `plugin_cli_audit_accepts_direct_source_and_installed_name` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_accepts_direct_source_and_installed_name`。
- `plugin_cli_audit_adds_target_context_for_missing_source` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_audit_adds_target_context_for_missing_source`。
- `plugin_cli_remove_deletes_plugin_and_disabled_marker` → `agent_plugins::host_migration_39_tests::cli::plugin_cli_remove_deletes_plugin_and_disabled_marker`。

### crates/vw-agent/src/plugins/compat/t29_tests.rs
基线 85 → 当前 0 行。
- `codex_plugin_compat_t29_reads_resource_over_real_stdio` → `agent_plugins::host_migration_39_tests::t29::codex_plugin_compat_t29_reads_resource_over_real_stdio`。
- `codex_plugin_compat_t29_public_chess_resource` → `agent_plugins::host_migration_39_tests::t29::codex_plugin_compat_t29_public_chess_resource`。

### crates/vw-agent/src/plugins/compat/t30_tests.rs
基线 125 → 当前 125 行。
- `codex_plugin_compat_t30_session_binding_uses_real_mcp_and_revokes` → `vw_agent::plugins::compat::t30_tests::codex_plugin_compat_t30_session_binding_uses_real_mcp_and_revokes`。
保留真实 ToolRuntimeContext、审批拒绝/允许、租户错配及撤销全过程，不能替换为领域无权限 Host。

### crates/vw-agent/src/plugins/compat/t58_tests.rs
基线 69 → 当前 0 行。
- `codex_plugin_compat_t58_reports_missing_runtime_before_execution` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_reports_missing_runtime_before_execution`。
- `codex_plugin_compat_t58_rejects_invalid_input` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_rejects_invalid_input`。
- `codex_plugin_compat_t58_preserves_isolation` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_preserves_isolation`。
- `codex_plugin_compat_t58_recognizes_bundled_python_module_without_importing` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_recognizes_bundled_python_module_without_importing`。
- `codex_plugin_compat_t58_rejects_dependency_symlink` → `agent_plugins::host_migration_39_tests::dependencies::codex_plugin_compat_t58_rejects_dependency_symlink`。

### crates/vw-agent/src/plugins/compat/t61_tests.rs
基线 351 → 当前 78 行。
- `codex_plugin_compat_t61_runs_installed_mcp_side_effect` → `agent_plugins::host_migration_39_tests::t61_mcp::codex_plugin_compat_t61_runs_installed_mcp_side_effect`。
- `codex_plugin_compat_t61_public_skill_lifecycle_and_upgrade` → `vw_agent::plugins::compat::t61_tests::codex_plugin_compat_t61_public_skill_lifecycle_and_upgrade`。
- `codex_plugin_compat_t61_public_stdio_original` → `agent_plugins::host_migration_39_tests::t61_mcp::codex_plugin_compat_t61_public_stdio_original`。
- `codex_plugin_compat_t61_public_marketplace_cli_lifecycle` → `agent_plugins::host_migration_39_tests::t61::codex_plugin_compat_t61_public_marketplace_cli_lifecycle`。
- `codex_plugin_compat_t61_public_hook_original` → `agent_plugins::host_migration_39_tests::t61::codex_plugin_compat_t61_public_hook_original`。
- `codex_plugin_compat_t61_public_http_original` → `agent_plugins::host_migration_39_tests::t61_mcp::codex_plugin_compat_t61_public_http_original`。
- `codex_plugin_compat_t61_original_reports_missing_dependency` → `agent_plugins::host_migration_39_tests::t61::codex_plugin_compat_t61_original_reports_missing_dependency`。
仅保留 SkillTool、实际安全上下文、技能注入消息和升级链路，含原始归档及哈希夹具。

### crates/vw-agent/src/plugins/compat/t62_tests.rs
基线 102 → 当前 0 行。
- `codex_plugin_compat_t62_calls_actual_public_readonly_service` → `agent_plugins::host_migration_39_tests::t62::codex_plugin_compat_t62_calls_actual_public_readonly_service`。
- `codex_plugin_compat_t62_rejects_invalid_input` → `agent_plugins::host_migration_39_tests::t62::codex_plugin_compat_t62_rejects_invalid_input`。
- `codex_plugin_compat_t62_preserves_isolation` → `agent_plugins::host_migration_39_tests::t62::codex_plugin_compat_t62_preserves_isolation`。

### crates/vw-agent/src/plugins/compat/t64_tests.rs
基线 82 → 当前 82 行。
- `codex_plugin_compat_t64_uses_platform_command_and_unicode_paths` → `vw_agent::plugins::compat::t64_tests::codex_plugin_compat_t64_uses_platform_command_and_unicode_paths`。
保留真实宿主 McpTool 与平台/Unicode 路径接线，作为迁出协议断言后的适配器验收。

### crates/vw-agent/src/plugins/hook_handler.rs
基线 8 → 当前 8 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/hook_handler_tests.rs
基线 506 → 当前 27 行。
- `trusted_plugin_before_tool_call_can_cancel` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_before_tool_call_can_cancel`。
- `trusted_plugin_after_tool_call_receives_json_payload` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_after_tool_call_receives_json_payload`。
- `trusted_plugin_before_tool_call_reports_spawn_failure` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_before_tool_call_reports_spawn_failure`。
- `trusted_plugin_before_tool_call_reports_timeout` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_before_tool_call_reports_timeout`。
- `trusted_plugin_before_tool_call_redacts_and_truncates_stdout_diagnostic` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_before_tool_call_redacts_and_truncates_stdout_diagnostic`。
- `trusted_plugin_untrusted_hook_is_not_registered` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_untrusted_hook_is_not_registered`。
- `plugin_hook_manifest_load_failure_reports_plugin_and_path_context` → `agent_plugins::host_migration_39_tests::hooks::plugin_hook_manifest_load_failure_reports_plugin_and_path_context`。
- `untrusted_plugin_hook_skip_does_not_warn` → `agent_plugins::host_migration_39_tests::hooks::untrusted_plugin_hook_skip_does_not_warn`。
- `disabled_plugin_hooks_skip_discovery` → `agent_plugins::host_migration_39_tests::hooks::disabled_plugin_hooks_skip_discovery`。
- `trusted_unsupported_plugin_hook_is_not_registered` → `agent_plugins::host_migration_39_tests::hooks::trusted_unsupported_plugin_hook_is_not_registered`。
- `trusted_plugin_before_llm_call_rejects_before_model_request` → `agent_plugins::host_migration_39_tests::hooks::trusted_plugin_before_llm_call_rejects_before_model_request`。

### crates/vw-agent/src/plugins/hook_trust.rs
基线 2 → 当前 2 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/hooks.rs
基线 2 → 当前 2 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/install.rs
基线 32 → 当前 14 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/install_tests.rs
基线 12 → 当前 0 行。
- `plugin_install_dir_uses_config_parent` → `agent_plugins::host_migration_39_tests::install::plugin_install_dir_uses_config_parent`。

### crates/vw-agent/src/plugins/loader.rs
基线 2 → 当前 2 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/manifest.rs
基线 2 → 当前 2 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/mcp.rs
基线 17 → 当前 17 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/mod.rs
基线 150 → 当前 138 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/skills.rs
基线 14 → 当前 14 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

### crates/vw-agent/src/plugins/skills_tests.rs
基线 199 → 当前 0 行。
- `plugin_skills_are_loaded_from_manifest_component_path` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_are_loaded_from_manifest_component_path`。
- `plugin_skills_follow_configured_security_policy` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_follow_configured_security_policy`。
- `plugin_skills_workspace_skill_overrides_plugin_skill` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_workspace_skill_overrides_plugin_skill`。
- `plugin_skills_disabled_plugin_skills_are_skipped` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_disabled_plugin_skills_are_skipped`。
- `plugin_skills_invalid_plugin_source_returns_empty` → `agent_plugins::host_migration_39_tests::skills::plugin_skills_invalid_plugin_source_returns_empty`。

### crates/vw-agent/src/plugins/types.rs
基线 6 → 当前 6 行。
保留：公共兼容导出或实际宿主组合；已无源码文件者由父模块直接导出领域入口。不计生产算法迁移收益。

## 行数和未来验证

22个来源按 splitlines：1968 → 527，累计净减 1441（73.2%）。相对首轮1466残留再减 939。物理减量包括收口任务删除兼容转发，不全算本轮收益。
领域 host_migration_39_tests 当前 42 条声明，ignore 5 条（平台 cfg 另计）；新目录含夹具 1447 行。原断言全量映射如上；生产算法本轮新增0。

`python3 scripts/cargo_slot.py cargo test -p agent-plugins --lib host_migration_39_tests -- --test-threads=1`：未来核对非零、平台实际数量和全部结果；ignore 原外部服务、PATH、依赖及 XDG 隔离条件原样保留，需分别 --ignored。
`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib plugins::hook_handler_tests::plugin_hook_host_registers_and_cancels -- --exact`：Unix 预期1。另保留 t30、t61 SkillTool、t64 真实宿主过滤，未来 --list 核对迁出声明在宿主消失，零测试不算通过。

静态检查：原测试名称映射、模块自注册、include_bytes 资源路径和差异空白检查；未运行 Cargo、单元、网络、平台、性能或实机检查，无编译收益结论。
