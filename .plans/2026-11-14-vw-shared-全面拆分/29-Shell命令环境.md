# 任务 29: Shell命令环境

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 29】依赖：批次 28（串行写入屏障）。数据前置任务：无。

状态：未开始

目的：迁移Shell命令环境职责，保持现有行为。

来源任务：系统底层全面拆分任务23。

预计会话范围：单一职责迁移；大文件在本职责内分文件，不扩展业务范围。

## 文件与真实上下文

- 源文件：`crates/vw-shared/src/shell/mod.rs`、`crates/vw-shared/src/shell/tests.rs`。
- 目标：`crates/libs/shell-command/Cargo.toml`、`crates/libs/shell-command/src/lib.rs`，实现及独立测试文件保持原基名；mod.rs 入口按所属模块组织。
- 清单接线：根 `Cargo.toml`、`crates/vw-shared/Cargo.toml`；直接消费者从 [审计](audit.md) 与 [引用快照](consumer-refs.md) 筛选本卡符号，修改命中的实际 Cargo.toml 和源码。
- 当前公开符号：effective_path_env, shell_profile_env_var, resolve_executable, apply_augmented_path_std_command, apply_augmented_path_tokio_command, std_command, tokio_command, std_system_command, tokio_system_command, apply_git_defaults_std_command, apply_git_defaults_tokio_command, git_std_command, git_tokio_command, PREFERRED, ACCEPTABLE, kill_tree。
- 当前源码与测试出现的根标识：collect, collections, env, ffi, fs, io, libc, once_cell, os, path, process, sync, tempfile, thread, time, tokio, unix；逐项核对后才作为业务或 dev 依赖，不把模块名当外包。

## 步骤与专属约束

1. 阅读上述文件完整实现、测试及调用方，对照 audit.md 确认前置类型已落地；符号同名必须保持类型唯一身份。
2. 590 行共享 EFFECTIVE_PATH、PREFERRED、ACCEPTABLE 与 BLACKLIST，路径探测/命令构造/kill_tree 同属进程入口且共享环境缓存，留同库按 environment/command/process 分文件；shell-tokio 原可选 feature 和 tokio process 保留，不改变 std_system_command 旁路含义、git defaults 或平台 kill 行为。
3. 将本卡实现和现有独立测试源码迁至目标，修正 crate/super、serde 字符串路径及 #[path]；不改变断言，不添加猜测 API。保持所有 cfg、平台限制及资源布局；非本卡测试留到其归属任务。
4. 接上 members、版本化路径依赖、门面 pub use 和直接消费者。门面重导出同一底层类型，不能复制定义。底层不得依赖 vw-shared；移除原实现，保留必要历史路径导出。

## 静态审查与完成证据

- 仅运行 `git diff --check`、`git diff --stat`，以及 `rg -n 'crate::|super::|include_str!|include_bytes!|#\[path|#\[cfg'` 限定本卡源/目标目录，人工核对模块路径、资源与条件编译。
- `rg -n 'vw_shared' crates --glob '*.rs'` 追踪本卡符号旧调用与聚合 use；审查全部受影响清单，确认无底层回指和复制状态。命中旧门面不等同失败，仓库内部直接调用必须完成迁移。
- 证据：实际迁移文件清单、直接消费者路径与依赖改动、保留资源/feature/cfg、静态差异审查结果。动态行为一律记录“未验证”。
- 静态审查完成后按公共规则中文提交并删本卡，未动态验证不保留卡。

## 以后统一验收（本次不执行）

覆盖本卡原有测试断言、所有直接消费者导入与目标编译；核对上述专属约束中协议、错误和平台行为。纯迁移不需要制造失败测试。保留测试过滤名和源码归属便于以后授权定向验证；不自动运行 Cargo、测试或实机。

既有测试源码函数索引（含辅助函数，未来授权后筛选真实 #[test]）：shell_env_test_lock, set_os, remove, drop, basename_returns_file_name_only_for_valid_paths, user_home_dir_prefers_home_and_falls_back_to_userprofile, user_home_dir_returns_none_when_home_variables_are_missing, shell_profile_paths_follow_shell_startup_precedence, strip_matching_quotes_only_removes_balanced_outer_quotes, replace_shell_home_and_path_vars_expands_supported_tokens, extract_path_assignment_rhs_supports_export_and_plain_assignments, extract_env_assignment_rhs_supports_custom_variable_name, brew_shellenv_dirs_detects_homebrew_prefixes, which_finds_program_from_process_path, which_returns_none_when_process_path_is_missing, which_returns_none_when_program_is_absent_from_present_path, push_unique_path_keeps_existing_dirs_once, push_known_path_keeps_missing_dir_and_dedupes, split_path_value_skips_empty_missing_and_duplicate_entries, infer_path_from_shell_profiles_returns_none_without_assignments, infer_path_from_shell_profiles_reads_path_exports, infer_path_from_shell_profiles_applies_profiles_in_startup_order, infer_env_var_from_shell_profiles_trims_name_and_uses_last_assignment, infer_env_var_from_shell_profiles_filters_blank_result, infer_env_var_from_shell_profiles_expands_home_and_path_tokens, effective_path_env_uses_profile_path_entries, effective_path_env_keeps_common_dirs_under_minimal_daemon_path, build_effective_path_uses_process_path_entries, build_effective_path_returns_none_without_process_path, shell_profile_env_var_reads_from_profile_when_process_env_missing, shell_profile_env_var_falls_back_when_process_env_is_blank, shell_profile_env_var_prefers_non_empty_process_env, shell_profile_env_var_returns_none_without_env_or_home, resolve_executable_accepts_existing_absolute_path, resolve_executable_accepts_existing_relative_path, resolve_executable_rejects_empty_and_missing_path_inputs, resolve_executable_finds_program_from_effective_path, resolve_executable_finds_program_from_profile_augmented_path, resolved_program_or_original_keeps_unresolved_program_name, which_in_path_value_finds_program_in_custom_path, which_in_path_value_returns_none_when_program_is_absent, std_command_uses_profile_augmented_path, apply_augmented_path_std_command_sets_path_when_available, tokio_command_uses_resolved_program_and_augmented_path, std_system_command_does_not_inject_augmented_path, tokio_system_command_uses_plain_program, git_std_command_applies_git_default_environment, git_tokio_command_applies_git_default_environment, fallback_uses_macos_default_shell, fallback_uses_bin_sh_when_bash_is_not_on_path, fallback_uses_bash_found_on_path, fallback_prefers_configured_git_bash_file, fallback_finds_git_bash_relative_to_git_on_path, fallback_uses_comspec_when_git_bash_is_unavailable, preferred_and_acceptable_shells_initialize_from_environment, kill_tree_returns_without_killing_when_exited_callback_is_true, kill_tree_kills_child_when_process_group_signal_fails, kill_tree_uses_process_group_signal_when_available。

业务依赖根标识匹配：once_cell, time, tokio。目标库不得反向依赖门面；测试源码已列于文件节。
