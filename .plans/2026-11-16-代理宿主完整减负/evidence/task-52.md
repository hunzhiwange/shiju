# 任务52：代码完成，编译/行为未验证

本轮依用户约束未运行任何 Cargo、构建、单元或实机测试。仅源码、注册、依赖和 git diff --check 静态核对。旧图索引早于共享目录迁移，本轮以实际源码为准。

## 生产归属与调用

`vw-agent::skills::handle_command → skills::cli::handle_command → agent_skills::cli::handle_command`：宿主传入 workspace、SkillsConfig 和真实 plugins::load_plugin_skills 闭包；领域执行 List/Audit/Install/Remove，沿用安全配置、初始化次序、路径检查和原错误。print_install_success 的 InstallResult 分支迁为领域 print_install_result。

`vw-agent::skills::init_skills_dir → init::init_skills_dir → agent_skills::init::init_skills_dir`：完整组合真实内置技能预加载和下载策略，不使用 mock。接收库新依赖 skill-core 仅取得原 SkillCommands 类型，无反向宿主依赖；Cargo.lock 同步已有包依赖边。tokio dev feature 补 fs 承接原异步文件场景。

catalog/loader/discovery 等生产实现此前已下沉，本轮仅承接其测试，不计为新增生产迁移。现存 Config 夹具仅有 workspace_dir 与 SkillsConfig，移除不参与断言的 config_path 赋值；插件库存回调为空用于纯本地场景，原文件没有插件业务断言。宿主保留真实命令转发错误断言，不以空回调替代宿主验收。

## 全文件与用例映射

|原文件|原行数|当前宿主行数|归属/目标|
|---|---:|---:|---|
|`agent_metadata.rs`|3|3|保留兼容导出：pub use agent_skills::agent_metadata::*;|
|`audit/mod.rs`|3|3|保留兼容导出：pub use agent_skills::audit::*;|
|`cli.rs`|171|9|领域 cli::handle_command/print_install_result；宿主保留真实插件库存端口|
|`cli_tests.rs`|385|0|crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs|

用例 `cli_tests.rs::wasm_cli_stub_is_explicitly_unsupported` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::wasm_cli_stub_is_explicitly_unsupported`（名称与断言保留）。


用例 `cli_tests.rs::skills_dir_helper_points_under_workspace` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::skills_dir_helper_points_under_workspace`（名称与断言保留）。


用例 `cli_tests.rs::handle_list_covers_empty_and_populated_workspaces` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_list_covers_empty_and_populated_workspaces`（名称与断言保留）。


用例 `cli_tests.rs::handle_list_loads_manifest_tools_and_tags` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_list_loads_manifest_tools_and_tags`（名称与断言保留）。


用例 `cli_tests.rs::handle_audit_reports_success_missing_and_findings` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_audit_reports_success_missing_and_findings`（名称与断言保留）。


用例 `cli_tests.rs::handle_audit_resolves_installed_skill_name` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_audit_resolves_installed_skill_name`（名称与断言保留）。


用例 `cli_tests.rs::handle_audit_reports_error_for_existing_file_source` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_audit_reports_error_for_existing_file_source`（名称与断言保留）。


用例 `cli_tests.rs::handle_remove_rejects_each_unsafe_name_form` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_remove_rejects_each_unsafe_name_form`（名称与断言保留）。


用例 `cli_tests.rs::handle_remove_removes_existing_skill_and_reports_missing_repeat` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_remove_removes_existing_skill_and_reports_missing_repeat`（名称与断言保留）。


用例 `cli_tests.rs::handle_remove_reports_missing_when_skills_dir_has_not_been_created` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_remove_reports_missing_when_skills_dir_has_not_been_created`（名称与断言保留）。


用例 `cli_tests.rs::handle_remove_reports_filesystem_error_when_target_is_not_directory` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_remove_reports_filesystem_error_when_target_is_not_directory`（名称与断言保留）。


用例 `cli_tests.rs::handle_remove_rejects_symlink_that_escapes_skills_directory` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_remove_rejects_symlink_that_escapes_skills_directory`（名称与断言保留）。


用例 `cli_tests.rs::handle_install_applies_configured_security_policy` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_install_applies_configured_security_policy`（名称与断言保留）。


用例 `cli_tests.rs::handle_install_rejects_missing_local_source` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_install_rejects_missing_local_source`（名称与断言保留）。


用例 `cli_tests.rs::handle_install_reports_skills_directory_creation_failure` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::handle_install_reports_skills_directory_creation_failure`（名称与断言保留）。


用例 `cli_tests.rs::print_install_success_covers_git_and_skills_sh_messages` → `crates/libs/agent-skills/src/host_migration_52_tests/cli_tests.rs::print_install_success_covers_git_and_skills_sh_messages`（名称与断言保留）。

|`discovery.rs`|3|3|保留兼容导出：pub use agent_skills::discovery::*;|
|`discovery_tests.rs`|155|0|crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs|

用例 `discovery_tests.rs::skill_discovery_loads_global_skills_from_unicode_userprofile_without_home` → `crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs::skill_discovery_loads_global_skills_from_unicode_userprofile_without_home`（名称与断言保留）。


用例 `discovery_tests.rs::skill_discovery_uses_userprofile_when_home_is_empty` → `crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs::skill_discovery_uses_userprofile_when_home_is_empty`（名称与断言保留）。


用例 `discovery_tests.rs::skill_discovery_omits_global_sources_without_home_variables` → `crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs::skill_discovery_omits_global_sources_without_home_variables`（名称与断言保留）。


用例 `discovery_tests.rs::skill_discovery_parses_claude_invocation_frontmatter` → `crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs::skill_discovery_parses_claude_invocation_frontmatter`（名称与断言保留）。


用例 `discovery_tests.rs::skill_discovery_applies_source_precedence_and_dynamic_paths` → `crates/libs/agent-skills/src/host_migration_52_tests/discovery_tests.rs::skill_discovery_applies_source_precedence_and_dynamic_paths`（名称与断言保留）。

|`init.rs`|58|1|领域 init::init_skills_dir；宿主兼容重导出|
|`init_tests.rs`|86|0|crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs|

用例 `init_tests.rs::init_creates_skills_directory_and_readme` → `crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs::init_creates_skills_directory_and_readme`（名称与断言保留）。


用例 `init_tests.rs::init_preserves_existing_readme` → `crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs::init_preserves_existing_readme`（名称与断言保留）。


用例 `init_tests.rs::init_returns_error_when_skills_path_is_file` → `crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs::init_returns_error_when_skills_path_is_file`（名称与断言保留）。


用例 `init_tests.rs::init_returns_error_when_readme_write_fails` → `crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs::init_returns_error_when_readme_write_fails`（名称与断言保留）。


用例 `init_tests.rs::init_returns_error_when_policy_path_is_directory` → `crates/libs/agent-skills/src/host_migration_52_tests/init_tests.rs::init_returns_error_when_policy_path_is_directory`（名称与断言保留）。

|`installer.rs`|7|2|保留兼容导出：mod installer_tests;|
|`installer_tests.rs`|64|0|crates/libs/agent-skills/src/host_migration_52_tests/installer_tests.rs|

用例 `installer_tests.rs::configured_skill_security_controls_loading_and_install_audit` → `crates/libs/agent-skills/src/host_migration_52_tests/installer_tests.rs::configured_skill_security_controls_loading_and_install_audit`（名称与断言保留）。

|`invoked.rs`|3|3|保留兼容导出：pub use agent_skills::invoked::*;|
|`invoked_tests.rs`|73|0|crates/libs/agent-skills/src/host_migration_52_tests/invoked_tests.rs|

用例 `invoked_tests.rs::invoked_skills_restore_after_compaction_resume_and_fork` → `crates/libs/agent-skills/src/host_migration_52_tests/invoked_tests.rs::invoked_skills_restore_after_compaction_resume_and_fork`（名称与断言保留）。


用例 `invoked_tests.rs::invoked_skill_reload_uses_current_file_state` → `crates/libs/agent-skills/src/host_migration_52_tests/invoked_tests.rs::invoked_skill_reload_uses_current_file_state`（名称与断言保留）。

|`listing.rs`|3|3|保留兼容导出：pub use agent_skills::listing::*;|
|`listing_tests.rs`|114|0|crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs|

用例 `listing_tests.rs::skill_listing_sends_only_budgeted_deltas_per_agent` → `crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs::skill_listing_sends_only_budgeted_deltas_per_agent`（名称与断言保留）。


用例 `listing_tests.rs::skill_listing_truncates_descriptions_and_respects_budget_deterministically` → `crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs::skill_listing_truncates_descriptions_and_respects_budget_deterministically`（名称与断言保留）。


用例 `listing_tests.rs::codex_skills_catalog_includes_canonical_source_and_usage_without_body` → `crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs::codex_skills_catalog_includes_canonical_source_and_usage_without_body`（名称与断言保留）。


用例 `listing_tests.rs::codex_skills_catalog_preserves_source_priority_and_omits_unverified_or_disabled_items` → `crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs::codex_skills_catalog_preserves_source_priority_and_omits_unverified_or_disabled_items`（名称与断言保留）。


用例 `listing_tests.rs::codex_skills_catalog_does_not_reopen_body_when_preparing_metadata` → `crates/libs/agent-skills/src/host_migration_52_tests/listing_tests.rs::codex_skills_catalog_does_not_reopen_body_when_preparing_metadata`（名称与断言保留）。

|`loader.rs`|3|3|保留兼容导出：pub use agent_skills::loader::*;|
|`marketplace/mod.rs`|3|3|保留兼容导出：pub use agent_skills::marketplace::*;|
|`mod.rs`|219|190|宿主运行时配置、租户插件库存与 session sources 装配；公共兼容导出|
|`mod_tests.rs`|36|18|crates/libs/agent-skills/src/host_migration_52_tests/mod_tests.rs；命令转发断言保留原宿主文件|

用例 `mod_tests.rs::discover_default_sources_includes_legacy_workspace_directory` → `crates/libs/agent-skills/src/host_migration_52_tests/mod_tests.rs::discover_default_sources_includes_legacy_workspace_directory`（名称与断言保留）。


用例 `mod_tests.rs::handle_command_forwards_remove_validation_errors` → `crates/vw-agent/src/skills/mod_tests.rs::handle_command_forwards_remove_validation_errors`（名称与断言保留）。

|`policy.rs`|3|3|保留兼容导出：pub use agent_skills::policy::*;|
|`prompt.rs`|2|2|保留兼容导出：pub use agent_skills::prompt::*;|
|`session_mounts.rs`|2|2|保留兼容导出：pub use agent_skills::session_mounts::*;|
|`session_state.rs`|2|2|保留兼容导出：pub use agent_skills::session_state::*;|
|`shiju.rs`|2|2|保留兼容导出：pub use agent_skills::shiju::*;|
|`source.rs`|3|3|保留兼容导出：pub use agent_skills::source::*;|
|`symlink_tests/mod.rs`|11|0|crates/libs/agent-skills/src/host_migration_52_tests/symlink_tests/mod.rs|
|`symlink_tests/tests.rs`|177|0|crates/libs/agent-skills/src/host_migration_52_tests/symlink_tests/tests.rs|

用例 `symlink_tests/tests.rs::test_skills_symlink_unix_edge_cases` → `crates/libs/agent-skills/src/host_migration_52_tests/symlink_tests/tests.rs::test_skills_symlink_unix_edge_cases`（名称与断言保留）。


用例 `symlink_tests/tests.rs::test_skills_symlink_permissions_and_safety` → `crates/libs/agent-skills/src/host_migration_52_tests/symlink_tests/tests.rs::test_skills_symlink_permissions_and_safety`（名称与断言保留）。

|`tests/helpers.rs`|155|0|crates/libs/agent-skills/src/host_migration_52_tests/tests/helpers.rs|
|`tests/load_skills.rs`|467|0|crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs|

用例 `tests/load_skills.rs::load_empty_skills_dir` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_empty_skills_dir`（名称与断言保留）。


用例 `tests/load_skills.rs::load_skill_from_toml` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_skill_from_toml`（名称与断言保留）。


用例 `tests/load_skills.rs::load_skill_from_md` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_skill_from_md`（名称与断言保留）。


用例 `tests/load_skills.rs::load_markdown_skill_prefers_frontmatter_description` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_markdown_skill_prefers_frontmatter_description`（名称与断言保留）。


用例 `tests/load_skills.rs::load_nonexistent_dir` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_nonexistent_dir`（名称与断言保留）。


用例 `tests/load_skills.rs::load_ignores_files_in_skills_dir` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_ignores_files_in_skills_dir`（名称与断言保留）。


用例 `tests/load_skills.rs::load_ignores_dir_without_manifest` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_ignores_dir_without_manifest`（名称与断言保留）。


用例 `tests/load_skills.rs::load_multiple_skills` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_multiple_skills`（名称与断言保留）。


用例 `tests/load_skills.rs::toml_skill_with_multiple_tools` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::toml_skill_with_multiple_tools`（名称与断言保留）。


用例 `tests/load_skills.rs::toml_skill_minimal` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::toml_skill_minimal`（名称与断言保留）。


用例 `tests/load_skills.rs::toml_skill_invalid_syntax_skipped` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::toml_skill_invalid_syntax_skipped`（名称与断言保留）。


用例 `tests/load_skills.rs::md_skill_heading_only` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::md_skill_heading_only`（名称与断言保留）。


用例 `tests/load_skills.rs::toml_prefers_over_md` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::toml_prefers_over_md`（名称与断言保留）。


用例 `tests/load_skills.rs::skills_dir_path` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::skills_dir_path`（名称与断言保留）。


用例 `tests/load_skills.rs::load_skills_discovers_workspace_ancestor_and_global_sources` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_skills_discovers_workspace_ancestor_and_global_sources`（名称与断言保留）。


用例 `tests/load_skills.rs::common_project_skill_roots_load_without_selecting_codex_provider` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::common_project_skill_roots_load_without_selecting_codex_provider`（名称与断言保留）。


用例 `tests/load_skills.rs::load_skills_uses_configured_directory_provider` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::load_skills_uses_configured_directory_provider`（名称与断言保留）。


用例 `tests/load_skills.rs::workspace_skill_roots_follow_directory_provider_contract` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::workspace_skill_roots_follow_directory_provider_contract`（名称与断言保留）。


用例 `tests/load_skills.rs::configured_prompt_mode_controls_loaded_skill_detail` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/load_skills.rs::configured_prompt_mode_controls_loaded_skill_detail`（名称与断言保留）。

|`tests/mod.rs`|6|0|crates/libs/agent-skills/src/host_migration_52_tests/tests/mod.rs|
|`tests/skills_init.rs`|168|0|crates/libs/agent-skills/src/host_migration_52_tests/tests/skills_init.rs|

用例 `tests/skills_init.rs::init_skills_creates_readme` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/skills_init.rs::init_skills_creates_readme`（名称与断言保留）。


用例 `tests/skills_init.rs::init_skills_idempotent` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/skills_init.rs::init_skills_idempotent`（名称与断言保留）。


用例 `tests/skills_init.rs::load_skills_with_config_compact_mode_uses_metadata_only` → `crates/libs/agent-skills/src/host_migration_52_tests/tests/skills_init.rs::load_skills_with_config_compact_mode_uses_metadata_only`（名称与断言保留）。

|`types.rs`|3|3|保留兼容导出：pub use agent_skills::types::*;|

## 数量与静态结论

来源 31 文件，迁移前 2390 行；当前 258 行，净减 2132 行（89.2%，超过70%）。原测试 60 条；领域承接 59 条（含 WASM 专用1条），宿主保留1条。

宿主生产及文档/转发 493 → 240 行；测试及夹具 1897 → 18 行。领域新注册测试/夹具共 1875 行；生产 cli/init 新增组合从原宿主迁入，未计既有实现收益。

领域 lib.rs 自注册 host_migration_52_tests，各子模块本地注册；宿主 cli/init/installer 原测试挂载、旧 tests 和 symlink_tests 挂载已移除。未使用 path/include 加载宿主源码。全场景保留，环境变量 RAII 和统一锁随组迁移，CLI 的独立锁合并至同组 helpers 锁。task-1 证据没有指向本组额外跨目录挂载。

## 未来验证（未执行）

`python3 scripts/cargo_slot.py cargo test -p agent-skills --lib host_migration_52_tests -- --test-threads=1`：原生 Unix 静态预期 58 条，WASM 用例按条件排除；其它平台另排除 Unix 专用用例。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib skills::mod_tests::handle_command_forwards_remove_validation_errors -- --exact --test-threads=1`：预期1条。迁出测试须通过宿主 list 证明消失，不能以零测试声称通过。skill-core 未新注册测试，无该库空过滤命令。

以上数量为源码静态计数，未来必须核对真实 list/ran；没有编译、行为通过或构建耗时收益结论。
