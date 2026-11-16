# 任务51恢复深迁证据（替代首次交付记录）

代码完成，编译/行为未验证。按用户要求没有运行任何 Cargo、构建、测试或可执行验证。静态 diff --check 通过；源码比较确认本次11条迁移测试函数仅修改命名空间及宿主初始化，断言原样保留。

## 收口残留处理

已读取 final.md 与 remaining-host.json：本任务12个残留文件共1247行。真实会话存储用例也由 task-plan-runner 所有，不应因调用宿主转发便整体留宿主。改用真实 session_store::ui_store 接口承接6条绑定/历史用例和2条执行持久化用例。TestHost 从内存 HashMap 改为真实 SessionStore 端口转发，保留取消后禁止加载历史断言；没有复制App或新增模拟存储。

启用 session-store/test-support、agent-database/test-support：沿用已有进程隔离测试SQLite与数据库线程继承能力，不读生产数据库，不新增环境变量配置或自建全局宿主；Once安装方式不变。外部模型仍仅经 SessionRunner 脚本端口模拟。共享 test_host 的 host_split_5_tests、host_split_6_tests 未来也需验证，因为从内存存储切换到真实存储。

## 真实生产接线

将 gateway_session_options 从宿主迁入 task-plan-runner::session_options，所有分支原样保留。GatewaySessionRunner::run_session 直接引用库导出的转换函数；真实ACP权限查询仍由宿主 acp_full_access 实现。接收库 lib.rs 注册生产模块并导出函数；没有反向依赖。删除无人调用的宿主 run_task_plan_in_home 与 persist_plan_run_session_history 转发。selected_acp_agent 两处保留局部3行转换，分别服务实际调用者，不新建抽象。

原先已下沉的模型、调度、目标、提示词、控制与执行算法不计本次生产迁移成果。本次真实生产迁移为 gateway_session_options；其它净减为测试/夹具迁移和无调用转发清理。保留两条真实递归策略接线测试，避免把实际宿主策略换成测试策略。

## 行数与文件归属

统一splitlines口径：基线 2653 行 → 首轮1247行 → 当前 350 行，累计净减 2303 行（86.8%），本轮再减 897 行。当前生产/重导出/宿主装配 227 行，测试/夹具 123 行。领域新增文件的测试与夹具共 2365 行，真实生产 session_options.rs 41 行，test_host.rs 21 行。lib.rs 注册单独存在，不宣称编译耗时收益。

| 来源 | 基线行数 | 当前宿主行数 | 当前具体职责 |
|---|---:|---:|---|
| `prompts.rs` | 24 | 16 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |
| `control.rs` | 1 | 1 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |
| `control_tests.rs` | 152 | 0 | 首轮整组迁库，旧文件及注册删除 |
| `session_bind_tests.rs` | 413 | 0 | 六条真实存储用例完整迁库，删除旧文件及注册 |
| `goal_tests.rs` | 147 | 36 | 真实递归策略接线及task局部构造器 |
| `session_bind.rs` | 6 | 3 | 仅ACP宿主测试所需ensure_plan_run_session_info入口 |
| `runner_tests.rs` | 1565 | 62 | 仅实际ACP grant继承与消息ID两条薄边界 |
| `runner.rs` | 176 | 138 | GatewaySessionRunner、真实租户ACP授权、processor装配、消息ID与PlanRunHost实现；options已迁库 |
| `mod.rs` | 58 | 47 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |
| `goal.rs` | 24 | 18 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |
| `schedule.rs` | 2 | 2 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |
| `prompts_tests.rs` | 83 | 25 | 真实递归策略接线 |
| `model.rs` | 2 | 2 | 公共领域入口/类型重导出、ensure_host安装或模块注册；无残留领域分支 |

## 全量用例映射

| 基线用例 | 真实目标完整名 |
|---|---|
| `control_tests.rs::task_plan_run_stop_is_tenant_scoped_and_cancels_descendants` | `host_migration_51_tests::control_tests::task_plan_run_stop_is_tenant_scoped_and_cancels_descendants` |
| `control_tests.rs::task_plan_run_control_bounds_terminal_snapshots_and_preserves_terminal_stop` | `host_migration_51_tests::control_tests::task_plan_run_control_bounds_terminal_snapshots_and_preserves_terminal_stop` |
| `session_bind_tests.rs::plan_run_binds_child_session_info_with_parent` | `host_migration_51_tests::session_bind_tests::plan_run_binds_child_session_info_with_parent` |
| `session_bind_tests.rs::plan_run_binds_session_info_without_parent` | `host_migration_51_tests::session_bind_tests::plan_run_binds_session_info_without_parent` |
| `session_bind_tests.rs::plan_run_binds_and_persists_history_through_run_task_plan` | `host_migration_51_tests::session_bind_tests::plan_run_binds_and_persists_history_through_run_task_plan` |
| `session_bind_tests.rs::run_plan_persists_subtask_details_before_finished_event` | `host_migration_51_tests::session_bind_tests::run_plan_persists_subtask_details_before_finished_event` |
| `session_bind_tests.rs::run_plan_does_not_report_subtask_success_when_session_persistence_fails` | `host_migration_51_tests::session_bind_tests::run_plan_does_not_report_subtask_success_when_session_persistence_fails` |
| `session_bind_tests.rs::plan_run_retry_replaces_legacy_failure_history_and_preserves_title` | `host_migration_51_tests::session_bind_tests::plan_run_retry_replaces_legacy_failure_history_and_preserves_title` |
| `goal_tests.rs::goal_task_files_require_completed_status_and_validation_evidence` | `host_migration_51_tests::goal_tests::goal_task_files_require_completed_status_and_validation_evidence` |
| `goal_tests.rs::goal_task_files_accept_expected_failure_only_with_successful_completion_check` | `host_migration_51_tests::goal_tests::goal_task_files_accept_expected_failure_only_with_successful_completion_check` |
| `goal_tests.rs::execution_prompt_separates_expected_failure_from_completion_evidence` | `host_migration_51_tests::goal_tests::execution_prompt_separates_expected_failure_from_completion_evidence` |
| `goal_tests.rs::goal_mode_session_ids_are_stable` | `host_migration_51_tests::goal_tests::goal_mode_session_ids_are_stable` |
| `goal_tests.rs::goal_continuation_keeps_outer_plan_and_reuses_recursive_policy` | `task_plan_run::goal_tests::goal_continuation_keeps_outer_plan_and_reuses_recursive_policy` |
| `goal_tests.rs::acp_background_approval_plan_rejects_external_task_files` | `host_migration_51_tests::goal_tests::acp_background_approval_plan_rejects_external_task_files` |
| `runner_tests.rs::run_plan_delays_subtask_finished_until_nested_agents_are_terminal` | `host_migration_51_tests::runner_tests::run_plan_delays_subtask_finished_until_nested_agents_are_terminal` |
| `runner_tests.rs::run_plan_ignores_legacy_project_cache` | `host_migration_51_tests::runner_tests::run_plan_ignores_legacy_project_cache` |
| `runner_tests.rs::task_plan_run_updates_only_canonical_user_plan_task_file` | `host_migration_51_tests::runner_tests::task_plan_run_updates_only_canonical_user_plan_task_file` |
| `runner_tests.rs::run_plan_reports_missing_home_and_creation_target` | `host_migration_51_tests::runner_tests::run_plan_reports_missing_home_and_creation_target` |
| `runner_tests.rs::run_plan_streams_each_subtask_before_completion` | `host_migration_51_tests::persistence_tests::run_plan_streams_each_subtask_before_completion` |
| `runner_tests.rs::goal_mode_continues_after_subtask_retries_and_finishes_via_checklist` | `host_migration_51_tests::runner_tests::goal_mode_continues_after_subtask_retries_and_finishes_via_checklist` |
| `runner_tests.rs::run_plan_errors_when_planning_session_skips_writing_tasks_json` | `host_migration_51_tests::runner_tests::run_plan_errors_when_planning_session_skips_writing_tasks_json` |
| `runner_tests.rs::goal_mode_finishes_from_completed_task_checklist_without_evaluator` | `host_migration_51_tests::runner_tests::goal_mode_finishes_from_completed_task_checklist_without_evaluator` |
| `runner_tests.rs::subtask_finished_requires_canonical_task_file_completion` | `host_migration_51_tests::runner_tests::subtask_finished_requires_canonical_task_file_completion` |
| `runner_tests.rs::goal_continuation_receives_prior_execution_evidence_and_canonical_paths` | `host_migration_51_tests::runner_tests::goal_continuation_receives_prior_execution_evidence_and_canonical_paths` |
| `runner_tests.rs::goal_mode_never_finishes_without_validation_evidence` | `host_migration_51_tests::runner_tests::goal_mode_never_finishes_without_validation_evidence` |
| `runner_tests.rs::goal_mode_blocks_on_explicit_external_blocker` | `host_migration_51_tests::runner_tests::goal_mode_blocks_on_explicit_external_blocker` |
| `runner_tests.rs::gateway_session_options_always_enable_full_access` | `host_migration_51_tests::options_tests::gateway_session_options_always_enable_full_access` |
| `runner_tests.rs::acp_background_approval_plan_run_defaults_to_approval` | `host_migration_51_tests::options_tests::acp_background_approval_plan_run_defaults_to_approval` |
| `runner_tests.rs::gateway_session_options_respects_confirmed_acp_full_access` | `host_migration_51_tests::options_tests::gateway_session_options_respects_confirmed_acp_full_access` |
| `runner_tests.rs::acp_background_approval_plan_directory_stays_in_workspace` | `host_migration_51_tests::runner_tests::acp_background_approval_plan_directory_stays_in_workspace` |
| `runner_tests.rs::acp_background_approval_plan_inherits_current_workspace_grant` | `task_plan_run::runner_tests::acp_background_approval_plan_inherits_current_workspace_grant` |
| `runner_tests.rs::gateway_plan_run_preallocates_assistant_message_id` | `task_plan_run::runner_tests::gateway_plan_run_preallocates_assistant_message_id` |
| `runner_tests.rs::plan_run_first_response_publishes_only_successful_stage_history` | `host_migration_51_tests::persistence_tests::plan_run_first_response_publishes_only_successful_stage_history` |
| `prompts_tests.rs::build_planning_prompt_requires_fixed_plan_dir_and_tool_writes` | `host_migration_51_tests::prompts_tests::build_planning_prompt_requires_fixed_plan_dir_and_tool_writes` |
| `prompts_tests.rs::plan_dir_abs_uses_user_storage_and_sanitizes_task_id` | `host_migration_51_tests::prompts_tests::plan_dir_abs_uses_user_storage_and_sanitizes_task_id` |
| `prompts_tests.rs::goal_mode_planning_prompt_requires_final_validation_task` | `host_migration_51_tests::prompts_tests::goal_mode_planning_prompt_requires_final_validation_task` |
| `prompts_tests.rs::execution_prompt_preserves_outer_task_and_allows_internal_orchestration` | `task_plan_run::prompts_tests::execution_prompt_preserves_outer_task_and_allows_internal_orchestration` |
| `prompts_tests.rs::execution_prompt_names_canonical_plan_and_task_paths` | `host_migration_51_tests::prompts_tests::execution_prompt_names_canonical_plan_and_task_paths` |

## 未来精确验证（本轮均未运行）

- `python3 scripts/cargo_slot.py cargo test -p task-plan-runner --lib host_migration_51_tests -- --test-threads=1`：Unix预期34条，非Unix32条。
- `python3 scripts/cargo_slot.py cargo test -p task-plan-runner --lib host_split_5_tests -- --test-threads=1` 与 `host_split_6_tests`：覆盖共用真实存储夹具变更，每组必须非零，预期数以本库真实list核实。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_plan_run:: -- --list`：应仅保留4条（ACP授权、消息ID、两条递归策略），迁库测试不能继续挂载宿主。
- 宿主4条按上表完整名逐条执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <完整名> -- --exact --test-threads=1`，各预期1条。

静态边界：已核对旧挂载删除、新模块自注册、真实生产调用与依赖方向。没有宏展开或编译证明；SQLite/线程/取消/授权的运行结果均待未来验证。整轮行数门槛由收口重新统计，本任务不替代全局验收。
