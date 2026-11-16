# 任务51静态交付证据

代码完成，编译/行为未验证。本轮按公共计划禁止运行任何 Cargo、构建、单元或实机测试。未声称编译收益或行为通过。

## 范围与行数

原13文件 2653 行；现宿主 1247 行，净减 1406 行（53.0%）；领域新增测试及注册 1496 行，lib.rs 注册另3行。原生产/转发 293 行，原测试/夹具 2360 行。现生产/转发 271 行，现测试/夹具 976 行。粗分中的生产主要为注册、re-export与宿主适配，非新增生产下沉成果。

净减未达70%，交收口任务验收。保留真实宿主验收不替换为模拟端口。继续可迁部分是混合用例中的领域断言；需先拆分真实存储验收，不能整体改用 TestHost 冒充真实持久化验证。

## 生产入口与依赖方向

实际调用链仍为 gateway → vw-agent::task_plan_run → ensure_host → task_plan_runner::runner，GatewaySessionRunner 调用真实会话 processor。PlanRunHost 具体实现负责当前租户、真实会话存储、首响应事务、时间和递归策略。原领域生产在上一轮已经下沉，本次生产迁移收益为0；删除3个测试专用转发函数及 prepare_plan_directory 别名。依赖保持 vw-agent → task-plan-runner → session-engine/session-store 等；未添加反向依赖、path/include宿主源码或依赖。

inspect_goal_task_files、inspect_goal_task_file、build_goal_continuation_prompt、goal_turn_session_id、task、全部原测试逐项归属见下。task 为局部测试构造器，宿主策略契约及领域目标测试各保留最小构造器。控制测试直接使用 session-engine 的真实 ChildAgentCoordinator。

接收库 lib.rs 自注册 host_migration_51_tests，再注册4个同目录模块；旧 control_tests 挂载/文件删除，其余宿主挂载仅承接保留契约。复用已有 test_host::install/Once 与隔离临时目录；不新增全局宿主，不复制App。本次读取任务1 evidence，未发现本族跨目录测试挂载；源码注册以当前入口为准。

## 全文件映射与保留原因

| 原文件 | 原行数 | 现宿主行数 | 归属/具体职责与继续可迁项 |
|---|---:|---:|---|
| `prompts.rs` | 24 | 16 | 公共提示词转发并确保策略宿主安装；删两个测试专用for_dir转发 |
| `control.rs` | 1 | 1 | 保留公共类型/调度/控制重导出、真实宿主端口转发或模块注册；无剩余领域算法 |
| `control_tests.rs` | 152 | 0 | 整组迁接收库，不保留 |
| `session_bind_tests.rs` | 413 | 413 | 全部6条真实会话存储验收及最小脚本夹具保留，覆盖parent/history/事件顺序/失败/legacy替换；可另拆纯快照断言，不能用模拟端口替代 |
| `goal_tests.rs` | 147 | 36 | 只留真实递归策略接线及局部task构造器 |
| `session_bind.rs` | 6 | 6 | 保留公共类型/调度/控制重导出、真实宿主端口转发或模块注册；无剩余领域算法 |
| `runner_tests.rs` | 1565 | 502 | 领域执行、路径、清单、后代等待迁库；保留网关 options、ACP grant、消息ID、真实首响应和流式持久化用例；流式顺序断言未来可拆分，但真实存储断言须保留 |
| `runner.rs` | 176 | 174 | 具体GatewaySessionRunner、权限继承、processor options、ID分配、PlanRunHost存储实现和ensure_host入口 |
| `mod.rs` | 58 | 52 | 保留公共类型/调度/控制重导出、真实宿主端口转发或模块注册；无剩余领域算法 |
| `goal.rs` | 24 | 18 | 公共领域入口转发并确保宿主安装；删测试专用inspect_goal_task_file |
| `schedule.rs` | 2 | 2 | 保留公共类型/调度/控制重导出、真实宿主端口转发或模块注册；无剩余领域算法 |
| `prompts_tests.rs` | 83 | 25 | 只留真实递归策略接线 |
| `model.rs` | 2 | 2 | 保留公共类型/调度/控制重导出、真实宿主端口转发或模块注册；无剩余领域算法 |

## 全用例映射

| 原用例 | 当前完整名 | 原因 |
|---|---|---|
| `control_tests.rs::task_plan_run_stop_is_tenant_scoped_and_cancels_descendants` | `host_migration_51_tests::control_tests::task_plan_run_stop_is_tenant_scoped_and_cancels_descendants` | 领域原断言迁移 |
| `control_tests.rs::task_plan_run_control_bounds_terminal_snapshots_and_preserves_terminal_stop` | `host_migration_51_tests::control_tests::task_plan_run_control_bounds_terminal_snapshots_and_preserves_terminal_stop` | 领域原断言迁移 |
| `goal_tests.rs::goal_task_files_require_completed_status_and_validation_evidence` | `host_migration_51_tests::goal_tests::goal_task_files_require_completed_status_and_validation_evidence` | 领域原断言迁移 |
| `goal_tests.rs::goal_task_files_accept_expected_failure_only_with_successful_completion_check` | `host_migration_51_tests::goal_tests::goal_task_files_accept_expected_failure_only_with_successful_completion_check` | 领域原断言迁移 |
| `goal_tests.rs::execution_prompt_separates_expected_failure_from_completion_evidence` | `host_migration_51_tests::goal_tests::execution_prompt_separates_expected_failure_from_completion_evidence` | 领域原断言迁移 |
| `goal_tests.rs::goal_mode_session_ids_are_stable` | `host_migration_51_tests::goal_tests::goal_mode_session_ids_are_stable` | 领域原断言迁移 |
| `goal_tests.rs::goal_continuation_keeps_outer_plan_and_reuses_recursive_policy` | `task_plan_run::goal_tests::goal_continuation_keeps_outer_plan_and_reuses_recursive_policy` | 宿主真实策略接线 |
| `goal_tests.rs::acp_background_approval_plan_rejects_external_task_files` | `host_migration_51_tests::goal_tests::acp_background_approval_plan_rejects_external_task_files` | 领域原断言迁移 |
| `prompts_tests.rs::build_planning_prompt_requires_fixed_plan_dir_and_tool_writes` | `host_migration_51_tests::prompts_tests::build_planning_prompt_requires_fixed_plan_dir_and_tool_writes` | 领域原断言迁移 |
| `prompts_tests.rs::plan_dir_abs_uses_user_storage_and_sanitizes_task_id` | `host_migration_51_tests::prompts_tests::plan_dir_abs_uses_user_storage_and_sanitizes_task_id` | 领域原断言迁移 |
| `prompts_tests.rs::goal_mode_planning_prompt_requires_final_validation_task` | `host_migration_51_tests::prompts_tests::goal_mode_planning_prompt_requires_final_validation_task` | 领域原断言迁移 |
| `prompts_tests.rs::execution_prompt_preserves_outer_task_and_allows_internal_orchestration` | `task_plan_run::prompts_tests::execution_prompt_preserves_outer_task_and_allows_internal_orchestration` | 宿主真实策略接线 |
| `prompts_tests.rs::execution_prompt_names_canonical_plan_and_task_paths` | `host_migration_51_tests::prompts_tests::execution_prompt_names_canonical_plan_and_task_paths` | 领域原断言迁移 |
| `runner_tests.rs::run_plan_delays_subtask_finished_until_nested_agents_are_terminal` | `host_migration_51_tests::runner_tests::run_plan_delays_subtask_finished_until_nested_agents_are_terminal` | 领域原断言迁移 |
| `runner_tests.rs::run_plan_ignores_legacy_project_cache` | `host_migration_51_tests::runner_tests::run_plan_ignores_legacy_project_cache` | 领域原断言迁移 |
| `runner_tests.rs::task_plan_run_updates_only_canonical_user_plan_task_file` | `host_migration_51_tests::runner_tests::task_plan_run_updates_only_canonical_user_plan_task_file` | 领域原断言迁移 |
| `runner_tests.rs::run_plan_reports_missing_home_and_creation_target` | `host_migration_51_tests::runner_tests::run_plan_reports_missing_home_and_creation_target` | 领域原断言迁移 |
| `runner_tests.rs::run_plan_streams_each_subtask_before_completion` | `task_plan_run::runner_tests::run_plan_streams_each_subtask_before_completion` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::goal_mode_continues_after_subtask_retries_and_finishes_via_checklist` | `host_migration_51_tests::runner_tests::goal_mode_continues_after_subtask_retries_and_finishes_via_checklist` | 领域原断言迁移 |
| `runner_tests.rs::run_plan_errors_when_planning_session_skips_writing_tasks_json` | `host_migration_51_tests::runner_tests::run_plan_errors_when_planning_session_skips_writing_tasks_json` | 领域原断言迁移 |
| `runner_tests.rs::goal_mode_finishes_from_completed_task_checklist_without_evaluator` | `host_migration_51_tests::runner_tests::goal_mode_finishes_from_completed_task_checklist_without_evaluator` | 领域原断言迁移 |
| `runner_tests.rs::subtask_finished_requires_canonical_task_file_completion` | `host_migration_51_tests::runner_tests::subtask_finished_requires_canonical_task_file_completion` | 领域原断言迁移 |
| `runner_tests.rs::goal_continuation_receives_prior_execution_evidence_and_canonical_paths` | `host_migration_51_tests::runner_tests::goal_continuation_receives_prior_execution_evidence_and_canonical_paths` | 领域原断言迁移 |
| `runner_tests.rs::goal_mode_never_finishes_without_validation_evidence` | `host_migration_51_tests::runner_tests::goal_mode_never_finishes_without_validation_evidence` | 领域原断言迁移 |
| `runner_tests.rs::goal_mode_blocks_on_explicit_external_blocker` | `host_migration_51_tests::runner_tests::goal_mode_blocks_on_explicit_external_blocker` | 领域原断言迁移 |
| `runner_tests.rs::gateway_session_options_always_enable_full_access` | `task_plan_run::runner_tests::gateway_session_options_always_enable_full_access` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::acp_background_approval_plan_run_defaults_to_approval` | `task_plan_run::runner_tests::acp_background_approval_plan_run_defaults_to_approval` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::gateway_session_options_respects_confirmed_acp_full_access` | `task_plan_run::runner_tests::gateway_session_options_respects_confirmed_acp_full_access` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::acp_background_approval_plan_directory_stays_in_workspace` | `host_migration_51_tests::runner_tests::acp_background_approval_plan_directory_stays_in_workspace` | 领域原断言迁移 |
| `runner_tests.rs::acp_background_approval_plan_inherits_current_workspace_grant` | `task_plan_run::runner_tests::acp_background_approval_plan_inherits_current_workspace_grant` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::gateway_plan_run_preallocates_assistant_message_id` | `task_plan_run::runner_tests::gateway_plan_run_preallocates_assistant_message_id` | 真实存储/权限/首次响应边界 |
| `runner_tests.rs::plan_run_first_response_publishes_only_successful_stage_history` | `task_plan_run::runner_tests::plan_run_first_response_publishes_only_successful_stage_history` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::plan_run_binds_child_session_info_with_parent` | `task_plan_run::session_bind_tests::plan_run_binds_child_session_info_with_parent` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::plan_run_binds_session_info_without_parent` | `task_plan_run::session_bind_tests::plan_run_binds_session_info_without_parent` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::plan_run_binds_and_persists_history_through_run_task_plan` | `task_plan_run::session_bind_tests::plan_run_binds_and_persists_history_through_run_task_plan` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::run_plan_persists_subtask_details_before_finished_event` | `task_plan_run::session_bind_tests::run_plan_persists_subtask_details_before_finished_event` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::run_plan_does_not_report_subtask_success_when_session_persistence_fails` | `task_plan_run::session_bind_tests::run_plan_does_not_report_subtask_success_when_session_persistence_fails` | 真实存储/权限/首次响应边界 |
| `session_bind_tests.rs::plan_run_retry_replaces_legacy_failure_history_and_preserves_title` | `task_plan_run::session_bind_tests::plan_run_retry_replaces_legacy_failure_history_and_preserves_title` | 真实存储/权限/首次响应边界 |

## 未来验证

- `python3 scripts/cargo_slot.py cargo test -p task-plan-runner --lib host_migration_51_tests -- --test-threads=1`：Unix预期 23 条，非Unix减去2条unix用例；必须非零。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_plan_run:: -- --list`：确认迁库完整名消失，仅保留上表 15 条宿主用例；不得把迁出后零命中当通过。
- 宿主按上表每个保留完整名执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib <完整名> -- --exact --test-threads=1`，各预期1条。
- 全部编译、运行、行为检查未运行；静态核对注册、依赖方向、名称/断言保留及 git diff --check。
