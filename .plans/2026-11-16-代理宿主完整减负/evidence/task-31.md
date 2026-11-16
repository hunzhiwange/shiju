# Task 31 evidence

代码完成，编译/行为未验证。本轮按公共计划禁止 Cargo、单元测试、实机及构建；仅源码和差异核对。

## 全文件和全用例映射

- `crates/vw-agent/src/tools/shell/processes_tests.rs` (299 行) → `crates/libs/tool-shell/src/host_migration_31_tests/processes_tests.rs` (299 行)。
  - `shell_process_survives_caller_runtime_drop` → 同名完整函数，断言保留。
  - `shell_process_scope_rejects_other_owner` → 同名完整函数，断言保留。
  - `shell_process_capacity_rejects_without_eviction` → 同名完整函数，断言保留。
  - `shell_process_spawn_failure_preserves_io_error_and_capacity` → 同名完整函数，断言保留。
  - `shell_process_shutdown_reaps_real_child` → 同名完整函数，断言保留。
  - `shell_output_wait_expiry_keeps_process_alive` → 同名完整函数，断言保留。
  - `shell_output_exit_drains_tail` → 同名完整函数，断言保留。
  - `shell_output_real_pipes_support_replay_overflow_and_utf8` → 同名完整函数，断言保留。
  - `shell_session_scope_cleanup_preserves_other_tenants_and_sessions` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/long_running_tests.rs` (239 行) → `crates/vw-agent/src/tools/shell/long_running_tests.rs` (239 行)。
  - `shell_long_running_real_entry_survives_twenty_minutes` → 同名完整函数，断言保留。
  - `shell_long_running_real_entry_can_cancel` → 同名完整函数，断言保留。
  - `shell_long_running_rejects_cross_session_poll` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/termination_tests.rs` (153 行) → `crates/libs/tool-shell/src/host_migration_31_tests/termination_tests.rs` (153 行)。
  - `shell_cancel_stops_descendants` → 同名完整函数，断言保留。
  - `shell_cancel_is_idempotent` → 同名完整函数，断言保留。
  - `shell_cancel_rejects_other_scope` → 同名完整函数，断言保留。
  - `shell_deadline_is_distinct_from_yield` → 同名完整函数，断言保留。
  - `shell_shutdown_reaps_children` → 同名完整函数，断言保留。
  - `shell_cancel_rejects_remote_runtime_before_spawn` → 同名完整函数，断言保留。
  - `shell_cancel_wakes_waiters` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/termination_windows_tests.rs` (131 行) → `crates/libs/tool-shell/src/host_migration_31_tests/termination_windows_tests.rs` (131 行)。
  - `shell_process_windows_fixture` → 同名完整函数，断言保留。
  - `shell_process_windows_cleanup_stops_descendants` → 同名完整函数，断言保留。
  - `shell_process_windows_rejects_remote_runtime_before_spawn` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/session_tools_tests.rs` (234 行) → `crates/vw-agent/src/tools/shell/session_tools_tests.rs` (234 行)。
  - `shell_session_tools_start_wait_cancel` → 同名完整函数，断言保留。
  - `shell_session_tools_survive_registry_rebuild` → 同名完整函数，断言保留。
  - `shell_session_tools_preserve_security_checks` → 同名完整函数，断言保留。
  - `shell_session_tools_expose_running_without_exit_code` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/mod.rs` (36 行) → `crates/vw-agent/src/tools/shell/mod.rs` (23 行)。
- `crates/vw-agent/src/tools/shell/environment_tests.rs` (52 行) → `crates/libs/tool-shell/src/host_migration_31_tests/environment_tests.rs` (52 行)。
  - `shell_environment_removes_explicit_secret_overrides` → 同名完整函数，断言保留。
  - `shell_environment_preserves_windows_runtime_locations_without_case_duplicates` → 同名完整函数，断言保留。
  - `shell_environment_does_not_add_windows_variables_on_unix` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/tests.rs` (1226 行) → `crates/libs/tool-shell/src/host_migration_31_tests/tests.rs` (1227 行)。
  - `shell_call_preserves_real_exit_code_without_parsing_output` → 同名完整函数，断言保留。
  - `shell_full_access_disables_workspace_sandbox` → 同名完整函数，断言保留。
  - `shell_workspace_scoped_full_autonomy_keeps_sandbox` → 同名完整函数，断言保留。
  - `shell_tool_name` → 同名完整函数，断言保留。
  - `shell_tool_spec_uses_bash_id` → 同名完整函数，断言保留。
  - `shell_tool_description` → 同名完整函数，断言保留。
  - `shell_tool_schema_has_command` → 同名完整函数，断言保留。
  - `extract_command_argument_supports_aliases` → 同名完整函数，断言保留。
  - `shell_argument_helpers_trim_defaults_and_reject_invalid_timeouts` → 同名完整函数，断言保留。
  - `output_helpers_normalize_merge_and_append_notes_without_extra_blank_lines` → 同名完整函数，断言保留。
  - `interpretation_results_keep_semantic_success_and_error_details` → 同名完整函数，断言保留。
  - `dedupe_paths_preserves_first_occurrence_order` → 同名完整函数，断言保留。
  - `shell_executes_allowed_command` → 同名完整函数，断言保留。
  - `shell_executes_command_from_cmd_alias` → 同名完整函数，断言保留。
  - `shell_workdir_blocks_external_absolute_path_without_allowlist` → 同名完整函数，断言保留。
  - `shell_workdir_allows_absolute_path_in_allowed_roots` → 同名完整函数，断言保留。
  - `shell_blocks_disallowed_command` → 同名完整函数，断言保留。
  - `shell_allows_readonly_command_in_readonly_mode` → 同名完整函数，断言保留。
  - `shell_blocks_non_readonly_command_in_readonly_mode` → 同名完整函数，断言保留。
  - `shell_missing_command_param` → 同名完整函数，断言保留。
  - `shell_wrong_type_param` → 同名完整函数，断言保留。
  - `shell_captures_exit_code` → 同名完整函数，断言保留。
  - `shell_blocks_absolute_path_argument` → 同名完整函数，断言保留。
  - `shell_blocks_option_assignment_path_argument` → 同名完整函数，断言保留。
  - `shell_blocks_short_option_attached_path_argument` → 同名完整函数，断言保留。
  - `shell_blocks_tilde_user_path_argument` → 同名完整函数，断言保留。
  - `shell_blocks_input_redirection_path_bypass` → 同名完整函数，断言保留。
  - `shell_strip_policy_allows_common_stderr_redirects` → 同名完整函数，断言保留。
  - `shell_strip_policy_still_blocks_unsupported_redirects` → 同名完整函数，断言保留。
  - `shell_allow_policy_executes_file_redirect` → 同名完整函数，断言保留。
  - `shell_does_not_leak_api_key` → 同名完整函数，断言保留。
  - `shell_preserves_path_and_home_for_env_command` → 同名完整函数，断言保留。
  - `shell_blocks_plain_variable_expansion` → 同名完整函数，断言保留。
  - `shell_allows_configured_env_passthrough` → 同名完整函数，断言保留。
  - `invalid_shell_env_passthrough_names_are_filtered` → 同名完整函数，断言保留。
  - `apply_allowed_shell_environment_injects_effective_path` → 同名完整函数，断言保留。
  - `shell_requires_approval_for_medium_risk_command` → 同名完整函数，断言保留。
  - `shell_initial_wait_defaults_to_thirty_seconds` → 同名完整函数，断言保留。
  - `shell_yield_rejects_out_of_range_values` → 同名完整函数，断言保留。
  - `shell_safe_env_vars_excludes_secrets` → 同名完整函数，断言保留。
  - `shell_safe_env_vars_includes_essentials` → 同名完整函数，断言保留。
  - `shell_blocks_rate_limited` → 同名完整函数，断言保留。
  - `shell_handles_nonexistent_command` → 同名完整函数，断言保留。
  - `shell_captures_stderr_output` → 同名完整函数，断言保留。
  - `shell_record_action_budget_exhaustion` → 同名完整函数，断言保留。
  - `shell_syscall_detector_writes_anomaly_log` → 同名完整函数，断言保留。
- `crates/vw-agent/src/tools/shell/processes.rs` (16 行) → `crates/vw-agent/src/tools/shell/processes.rs` (2 行)。
- `crates/vw-agent/src/tools/shell/session_tools.rs` (3 行) → `crates/vw-agent/src/tools/shell/session_tools.rs` (3 行)。

## 生产入口、依赖与保留边界

- 本轮没有迁移生产算法：ShellTool、ShellProcesses、BashWaitTool、BashCancelTool 原已由 tool-shell 实现；宿主保留 AgentContextHost 泛型绑定及真实装配，不把已有下沉计作收益。
- tools/mod.rs 真实导出 shell::session_tools::{BashCancelTool,BashWaitTool}；shell/mod.rs 绑定 ShellTool<AgentContextHost>，processes.rs 向宿主生命周期导出 shutdown_shell_processes；依赖始终 vw-agent → tool-shell → tool-context，接收库不依赖 vw-agent，不挂载宿主源码。
- 接收库 shell/mod.rs 自注册 host_migration_31_tests，其下 5 个行为模块完整承接原测试。迁移测试直接访问同领域私有字段/parse_args，不再依赖测试公开转发；processes 的两个既有测试辅助方法增加 cfg(test) 可用性，生产默认能力不变。
- host.rs 是最小 ContextHost 外部端口夹具：固定测试租户、独立 task-local、无宿主工作树租约；其它未使用能力全部 panic。实际 OS 进程、管道、权限策略与临时目录继续使用真实实现。真实租户/租约与执行桥接验收仍由宿主承担。
- 全局环境修改沿用 Mutex + EnvGuard 恢复方式，新测试进程内单独锁定；没有拷贝 App/AppState。Windows FIXTURE 完整名称改为 shell::host_migration_31_tests::termination_windows_tests::shell_process_windows_fixture。
- dev-dependencies 增加 config-security（真实配置）、permission-types 与 vw-api-transport（Host 签名），tokio fs/rt-multi-thread（原测试已使用）。Cargo.lock 仅新增 tool-shell 的三个依赖边。

保留逐符号职责：
- mod.rs：ShellTool、环境函数、sandbox/security 等公共兼容导出与真实宿主绑定；测试 import 仅服务宿主边界用例。
- processes.rs：shutdown_shell_processes 等原公共导出；不含领域测试挂载。
- session_tools.rs：BashWaitTool、BashCancelTool 的真实 AgentContextHost 类型别名。
- session_tools_tests.rs：registry 验证 default_tools_with_runtime 真实注册；context/call/start 使用真实 execute_tool_from_registry；shell_session_tools_start_wait_cancel 保留三工具注册链，survive_registry_rebuild 保留宿主注册表与调用运行时销毁边界，preserve_security_checks 保留真实执行器鉴权、跨会话/工作区/租户拒绝，expose_running_without_exit_code 保留执行器结果协议。均不以模拟注册表替换；后续仍可将重复的协议断言拆分，但本轮保留整函数以保全真实入口验收。
- long_running_tests.rs：context/call/ProcessCleanup::drop/start/poll/append 均服务 execute_tool_call 同步桥接；三个测试分别覆盖二十分钟运行（原 ignore 保留）、跨桥接取消与真实租户作用域拒绝。wait_until/read_pid/assert_gone 是真实 OS 回收证据夹具。不能移到领域库并声称仍验证宿主同步桥接。

## 行数与未来验证

来源总行数 2389 → 501，净减 1888 (79.0%)。宿主装配文件物理行数（含原测试挂载/import）由 55 → 28 行；宿主测试 2334 → 473 行。迁入领域测试文件 1862 行；另有领域注册夹具，不作为生产迁移收益。领域生产算法净增 0 行，processes cfg 调整 2 处、shell 注册 4 行。

- `processes_tests.rs`：静态 9 个 test 属性（平台条件决定实际数量）。
- `termination_tests.rs`：静态 7 个 test 属性（平台条件决定实际数量）。
- `termination_windows_tests.rs`：静态 3 个 test 属性（平台条件决定实际数量）。
- `environment_tests.rs`：静态 3 个 test 属性（平台条件决定实际数量）。
- `tests.rs`：静态 46 个 test 属性（平台条件决定实际数量）。

未来命令（本轮全部未运行，未取得编译或行为通过证据）：

```sh
python3 scripts/cargo_slot.py cargo test -p tool-shell --lib host_migration_31_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::shell::session_tools_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::shell::long_running_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::shell -- --list
```

宿主 session_tools_tests 预期 4；Unix long_running_tests 预期 2 执行、1 ignored，二十分钟项需未来显式 --ignored 实机验收。领域 Unix 预期 64，Windows 预期 59（按当前 cfg 静态计数，必须未来 list 校准并确认非零）。源映射中 tests.rs 的旧路径为 tools::shell::tests::tests::<name>，新路径为 shell::host_migration_31_tests::tests::tests::<name>；其余领域模块新路径 shell::host_migration_31_tests::<文件名去.rs>::<name>。旧 processes_tests 的旧挂载名为 tools::shell::processes::tests；termination 模块旧挂载在 processes 下。未来宿主 list 应证明上述迁出组已消失，不能把零测试作为通过。

静态核对：原每个测试函数均完整迁移或保留；未删除断言；旧领域模块挂载已删除；没有反向依赖或 path/include 加载宿主；git diff --check 已执行。本轮没有运行 Cargo、测试、构建、格式化或实机进程验证；不宣称构建耗时收益。

真实生产分类：宿主原有生产声明/导出共 18 行，迁后仍 18 行；其余装配文件行属于测试注册/import 或空行。领域真实生产算法前后不变；新增测试 Host 与环境锁均 cfg(test)。不将转发清理计为业务下沉。
