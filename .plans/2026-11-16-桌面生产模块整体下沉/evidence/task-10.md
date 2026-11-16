# 任务10：静态交付证据

实现与真实接线已完成；本轮未编译、未测试、未绘制，未执行任何 Cargo 命令。按用户覆盖仅做静态交付，后续验证需另行授权。

## 生产归属与调用链

- 桌面消息路由仍调用 task_board::update::update；原消息枚举不另建别名。update 先委派既有 model/logs/settings/subtasks/local，再委派新 bulk/manual_worktree 局部更新，未消费消息才进入外部效果适配。所有新模块在 update/mod.rs 注册；host_module_10_tests 已在 lib.rs 用 cfg(test) 注册。
- LocalState 接回任务可变借用、编辑ID、详情标签、偏好字段及项目路径输入。卡片释放、编辑开关、创建/保存结果、复制/删除/归档、拖放排序、导入与模板、偏好回写意图和日志页签决策由领域拥有；宿主只执行 LocalEffect。领域状态继续是原 TaskDraft/Task/TaskBoardSettings/日志状态类型，局部借用结构只借本域字段，不复制 App、不用 Deref。
- update::bulk 持有完整批量操作转换及选择清理，返回 Persist/Archive/Delete/Move/Merge 和日志同步ID。bulk.rs 构造完整网关元数据、移动/删除/归档载荷，并拥有手动合并队列筛选和日志回写。宿主只装配网关客户端与异步结果消息。
- update::manual_worktree 拥有二次确认、互斥、忙槽位拒绝、日志生命周期、成功/失败反馈和刷新策略；返回通知/Toast/原生清理与回收/延迟Tick等具名效果。宿主保存通知ID、Toast计时器、GatewayClient绑定。
- update::worktree 拥有快照节流、缺项目清空、加载完成、维护结果与维护准入；领域选择污染/失效槽位，宿主顺序执行回收请求并保留失败即返回。
- BoardProjection 组合已有投影与 TaskLogsState；加载、调度状态、调度日志、选择裁剪、缓存及查看器同步全部在领域；acknowledge 明确不论结果成功失败均释放持久化保护。
- helpers::execution 接收真实 TaskExecutorState 与 TaskLogsState，执行有界日志轮询、节流和超时回写；只有计划文件写入使用窄 FnMut 回调。finish_planning 复用 apply_planning_result/start_planned_task，完整决定持久化、失败及启动结果；宿主只提供计划写入和工作树分配回调并启动任务。
- helpers::settings 拥有所有期限算术；宿主仅保存 RecentProjectMeta（导航领域类型）和创建 Iced 延迟任务。未增加依赖、配置、trait 或反向加载；领域不引用 crate::app，不 path/include 宿主。

## 行为保持说明

- CardReleased 原验证后的 reasoning_effort 立即被 task.reasoning_effort 覆盖；迁移保留最终值，原模型选项查询为纯读取。OpenTaskInSession 原计算但未使用的 _prompt 移除，仍仅隐藏看板并发送 Pending 状态变更。
- EditTask 同步缓存只影响任务 logs，草稿字段不读取 logs；草稿构造先后调整不改变字段值。无产品功能、UI布局、字体、滚动条或主题变更。
- 网络创建/发送顺序、失败检查、项目隔离、stop_on_error=true、回收失败即返回、日志扫描预算和节流保留。

## 来源统计（pathlib splitlines，含空行注释）

| 来源 | 分类 | 基线 | 当前 | 减量 |
|---|---|---:|---:|---:|
| `crates/vw-desktop/src/app/message/task_board/helpers/bulk.rs` | 生产候选 | 301 | 152 | 149 |
| `crates/vw-desktop/src/app/message/task_board/helpers/bulk_tests.rs` | 测试 | 221 | 65 | 156 |
| `crates/vw-desktop/src/app/message/task_board/helpers/draft.rs` | 生产候选 | 14 | 2 | 12 |
| `crates/vw-desktop/src/app/message/task_board/helpers/draft_adapter_tests.rs` | 测试 | 15 | 0 | 15 |
| `crates/vw-desktop/src/app/message/task_board/helpers/execution.rs` | 生产候选 | 559 | 482 | 77 |
| `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 测试 | 597 | 113 | 484 |
| `crates/vw-desktop/src/app/message/task_board/helpers/logs.rs` | 生产候选 | 55 | 36 | 19 |
| `crates/vw-desktop/src/app/message/task_board/helpers/review.rs` | 生产候选 | 2 | 2 | 0 |
| `crates/vw-desktop/src/app/message/task_board/helpers/settings.rs` | 生产候选 | 186 | 170 | 16 |
| `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 测试 | 146 | 75 | 71 |
| `crates/vw-desktop/src/app/message/task_board/helpers_tests.rs` | 测试 | 74 | 0 | 74 |
| `crates/vw-desktop/src/app/message/task_board/update.rs` | 生产候选 | 1907 | 1093 | 814 |
| `crates/vw-desktop/src/app/message/task_board/update_tests.rs` | 测试 | 78 | 0 | 78 |

来源总量 4155 → 2190，宿主净减量 1965 行；生产候选 3024 → 1937，生产候选减少 1087 行。未新增宿主替代文件。helpers.rs 是来源清单外必要父模块接线，额外删去失效模板包装与测试挂载，单独列入审计，不冒充上述来源减量。

## 逐测试断言映射

表中数量为源码 assert!/assert_eq!/assert_ne! 调用数，不是执行通过数；所有测试未运行。辅助窗口范围断言函数整体迁移。

| 原文件/测试 | 现在归属 | 原/现断言 |
|---|---|---:|
| `bulk_tests.rs::empty_bulk_command_short_circuits_without_gateway_access` | `crates/vw-desktop/src/app/message/task_board/helpers/bulk_tests.rs` | 1/1 |
| `bulk_tests.rs::empty_gateway_bulk_wrappers_short_circuit_without_gateway_access` | `crates/vw-desktop/src/app/message/task_board/helpers/bulk_tests.rs` | 4/4 |
| `bulk_tests.rs::reset_bulk_operation_inputs_copies_draft_and_recent_choices` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 6/6 |
| `bulk_tests.rs::reset_bulk_operation_inputs_uses_priority_model_and_agent_defaults` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 3/3 |
| `bulk_tests.rs::deactivate_bulk_selection_mode_clears_only_active_status_selection` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 3/3 |
| `bulk_tests.rs::deactivate_bulk_selection_mode_without_active_status_preserves_selection` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 1/1 |
| `bulk_tests.rs::manual_merge_running_requires_running_pr_submitted_board_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 2/2 |
| `bulk_tests.rs::pop_next_manual_merge_task_discards_invalid_entries_and_returns_first_eligible_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 3/3 |
| `bulk_tests.rs::pop_next_manual_merge_task_drains_queue_without_eligible_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 2/2 |
| `bulk_tests.rs::start_manual_merge_queue_task_returns_none_when_pr_merge_is_already_running` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 2/2 |
| `bulk_tests.rs::start_manual_merge_queue_task_with_project_and_no_eligible_task_drains_invalid_queue` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 2/1 |
| `bulk_tests.rs::start_manual_merge_queue_task_with_eligible_task_adds_log_and_pops_queue` | `crates/libs/desktop-task-board/src/host_module_10_tests/bulk_tests.rs` | 3/3 |
| `bulk_tests.rs::start_manual_merge_queue_task_without_project_clears_queue_and_notifies` | `crates/vw-desktop/src/app/message/task_board/helpers/bulk_tests.rs` | 3/3 |
| `draft_adapter_tests.rs::reset_create_draft_rebuilds_state_and_synchronizes_prompt_editor` | `crates/libs/desktop-task-board/src/host_module_10_tests/draft_adapter_tests.rs` | 4/4 |
| `execution_tests.rs::test_module_is_wired` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 1/1 |
| `execution_tests.rs::task_board_does_not_touch_legacy_task_database` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 1/1 |
| `execution_tests.rs::merge_stall_timeout_ignores_queued_pr_submitted_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 4/4 |
| `execution_tests.rs::merge_stall_timeout_pauses_running_pr_submitted_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 6/6 |
| `execution_tests.rs::merge_stall_timeout_waits_for_runtime_cleanup_without_unlock_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 3/3 |
| `execution_tests.rs::git_flow_failure_after_success_does_not_auto_retry_task` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 4/4 |
| `execution_tests.rs::failed_task_scan_keeps_failed_status_without_retry_or_rebuild` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 5/5 |
| `execution_tests.rs::running_timeout_first_scan_keeps_task_running` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 5/5 |
| `execution_tests.rs::running_timeout_second_scan_fails_without_recycling` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 5/5 |
| `execution_tests.rs::code_review_model_timeout_retry_waits_before_dispatch` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 2/2 |
| `execution_tests.rs::worktree_action_log_keeps_only_the_newest_bounded_lines` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 3/3 |
| `execution_tests.rs::clear_and_finish_worktree_action_logs_reset_transient_state` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 9/9 |
| `execution_tests.rs::poll_worktree_action_logs_filters_blank_lines_and_clears_disconnected_receiver` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 2/2 |
| `execution_tests.rs::poll_worktree_action_logs_keeps_receiver_when_channel_is_empty` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 2/2 |
| `execution_tests.rs::flushing_without_running_tasks_resets_log_scan_cursor` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 1/1 |
| `execution_tests.rs::throttled_log_flush_respects_interval_unless_forced` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 3/3 |
| `execution_tests.rs::worktree_snapshot_refresh_interval_tracks_panel_visibility` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 2/2 |
| `execution_tests.rs::missing_project_path_clears_stale_worktree_snapshot_state` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 3/3 |
| `execution_tests.rs::non_git_project_schedules_logical_pool_refresh` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 7/7 |
| `execution_tests.rs::git_project_still_schedules_worktree_snapshot_refresh` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 2/2 |
| `execution_tests.rs::worktree_snapshot_refresh_skips_loading_and_interval_but_force_schedules` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 4/4 |
| `execution_tests.rs::worktree_maintenance_respects_in_flight_and_manual_action_guards` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 4/4 |
| `execution_tests.rs::persistence_and_removal_helpers_stay_idle_without_local_ownership` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 2/2 |
| `execution_tests.rs::worktree_snapshot_from_gateway_preserves_counts_locks_and_slot_state` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 8/8 |
| `execution_tests.rs::timeout_helpers_clamp_configuration_and_recognize_only_timeout_errors` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 7/7 |
| `execution_tests.rs::code_review_selection_honors_exclusions_and_priority_order` | `crates/libs/desktop-task-board/src/host_module_10_tests/execution_tests.rs` | 2/2 |
| `execution_tests.rs::task_pool_desktop_scheduler_prepares_logical_capacity_without_worktrees` | `crates/vw-desktop/src/app/message/task_board/helpers/execution_tests.rs` | 5/5 |
| `settings_tests.rs::save_settings_without_project_leaves_recent_projects_unchanged` | `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 1/1 |
| `settings_tests.rs::save_settings_updates_existing_project_meta` | `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 2/2 |
| `settings_tests.rs::save_settings_creates_project_meta_for_new_project_path` | `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 4/4 |
| `settings_tests.rs::scheduler_deadline_is_preserved_while_still_in_the_future` | `crates/libs/desktop-task-board/src/host_module_10_tests/settings_tests.rs` | 1/1 |
| `settings_tests.rs::scheduler_deadline_is_refreshed_after_expiry` | `crates/libs/desktop-task-board/src/host_module_10_tests/settings_tests.rs` | 0/0 |
| `settings_tests.rs::fixed_interval_schedulers_refresh_their_deadlines` | `crates/libs/desktop-task-board/src/host_module_10_tests/settings_tests.rs` | 0/0 |
| `settings_tests.rs::auto_promote_scheduler_uses_sanitized_interval_for_deadline` | `crates/libs/desktop-task-board/src/host_module_10_tests/settings_tests.rs` | 0/0 |
| `settings_tests.rs::standalone_schedulers_produce_deferred_actions` | `crates/vw-desktop/src/app/message/task_board/helpers/settings_tests.rs` | 1/1 |
| `helpers_tests.rs::test_module_is_wired` | `crates/libs/desktop-task-board/src/host_module_10_tests/helpers_tests.rs` | 1/1 |
| `helpers_tests.rs::import_prompt_template_json_includes_selected_values_and_format_guidance` | `crates/libs/desktop-task-board/src/host_module_10_tests/helpers_tests.rs` | 5/5 |
| `helpers_tests.rs::import_prompt_template_csv_uses_defaults_for_blank_model_and_missing_executor` | `crates/libs/desktop-task-board/src/host_module_10_tests/helpers_tests.rs` | 3/3 |
| `helpers_tests.rs::import_prompt_template_tsv_trims_model_and_includes_reasoning_effort_column` | `crates/libs/desktop-task-board/src/host_module_10_tests/helpers_tests.rs` | 4/4 |
| `update_tests.rs::persistence_ack_releases_snapshot_guard` | `crates/libs/desktop-task-board/src/host_module_10_tests/update_tests.rs` | 2/2 |

- 唯一拆分：start_manual_merge_queue_task_with_project_and_no_eligible_task_drains_invalid_queue 的队列断言在领域，通知为空断言保留于宿主 manual_merge_without_eligible_task_does_not_notify，沿用原缺失ID+Pending任务夹具。其余表内原断言全部保留。
- 两组快照/维护用例的 Task.units()/into_stream 存在性改为领域准入 bool；宿主 non_git_project_schedules_logical_pool_refresh/git_project_still_schedules_worktree_snapshot_refresh 保留真实 Iced Task 单位与 stream 契约。没有用空过滤或执行结果替代断言。
- persistence_ack_releases_snapshot_guard 的 App 消息驱动改为真实 BoardProjection.acknowledge/status，成功与失败两种 Result 均传入生产入口；项目隔离保留于真实宿主回调适配。
- 与已有 bulk_tests/helpers execution/settings、desktop_split_* 独立组核对：旧库测试覆盖模型规范化、请求默认值、可见选择、日志流等基础算法；本轮是重置偏好、整条超时与队列转换、缓存保护释放、截止时间调用边界的整组证据。未将不同夹具的交叠断言当作等价删去。test_module_is_wired 分别检查自身真实模块挂载，不能互相替代。
- 新增 local_update_tests 直接调用生产 update_bulk/update_local，记录批量优先级的持久化意图、选择与本地任务不提前回写，以及项目隔离和创建后关闭/清空行为；源码已注册，未执行。

## 宿主残留逐函数归属

- `crates/vw-desktop/src/app/message/task_board/helpers/bulk.rs`：`submit_agent_scheduler_bulk_command`、`persist_updated_tasks_via_gateway`、`delete_tasks_via_gateway`、`send_archive_commands_via_gateway`、`move_task_status_via_gateway`、`move_tasks_to_status_via_gateway`、`bulk_state`、`start_manual_merge_queue_task`。
- `crates/vw-desktop/src/app/message/task_board/helpers/draft.rs`：仅领域重导出。
- `crates/vw-desktop/src/app/message/task_board/helpers/execution.rs`：`flush_running_task_logs_throttled`、`schedule_worktree_action_log_tick`、`resolve_task_board_project_id`、`resolve_task_board_scheduler_scope`、`sync_agent_scheduler_status`、`load_agent_scheduler_logs`、`sync_agent_scheduler_logs_for_project_path`、`sync_agent_scheduler_logs`、`schedule_agent_scheduler_command_in_flight_status_sync`、`sync_task_board_gateway_snapshot`、`create_task_via_agent_scheduler`、`register_pending_agent_scheduler_tasks`、`send_agent_scheduler_command`、`worktree_snapshot_from_gateway`、`cleanup_office_contexts`、`recycle_worktree_slot_via_gateway`、`recycle_task_worktree_via_gateway`、`schedule_worktree_pool_maintenance`、`maybe_schedule_worktree_snapshot_refresh`、`build_persist_tasks`、`recycle_assigned_task_worktree_for_removal`、`recycle_assigned_task_worktrees_for_removal`、`snapshot_state`。
- `crates/vw-desktop/src/app/message/task_board/helpers/logs.rs`：`sync_task_logs_editor`、`refresh_task_log_cache`、`set_viewing_logs`、`sync_viewing_logs`、`close_logs_context_menu`、`logs_state`。
- `crates/vw-desktop/src/app/message/task_board/helpers/review.rs`：仅领域重导出。
- `crates/vw-desktop/src/app/message/task_board/helpers/settings.rs`：`save_settings`、`scheduler_tick_interval_secs`、`auto_promote_tick_interval_secs`、`schedule_scheduler_tick`、`schedule_auto_review_tick`、`schedule_auto_merge_tick`、`schedule_scheduler_tick_with_deadline`、`schedule_auto_review_tick_with_deadline`、`schedule_auto_merge_tick_with_deadline`、`schedule_auto_promote_tick_with_deadline`。
- `crates/vw-desktop/src/app/message/task_board/update.rs`：`task_board_project_matches`、`initial_task_status_for_app`、`sync_agent_scheduler_status_for_current_project`、`agent_scheduler_gateway_unavailable`、`persist_task_metadata_patch_task`、`apply_agent_scheduler_status`、`apply_agent_scheduler_logs`、`schedule_agent_scheduler_status_followup_ticks`、`update`、`apply_local_effect`、`apply_worktree_effects`、`projection_state`。

保留原因按上列函数逐组对应：

- bulk 的 submit/persist/delete/archive/move 只绑定项目scope、真实GatewayClient、DTO传输及拒绝错误；bulk_state 仅借用装配；start_manual_merge_queue_task 只适配领域意图至通知、网关任务和状态同步计时器。
- draft/review 仅重导出；logs 的 sync/refresh/set/close 仅为既有日志状态构造局部借用；logs_state 为宿主字段解包。
- execution 的 resolve/sync/create/register/send/recycle/cleanup 系列均涉及客户端scope、配置或网关网络调用；flush_throttled 只装配借用与计划写入回调；schedule 系列创建 Iced Task；snapshot_state 为借用装配；build_persist_tasks 保持原无本地持久化所有权的空操作，不能恢复旧SQLite路径。
- settings.save_settings 接 RecentProjectMeta 与后台保存，属于跨导航持久化；interval 包装只传 settings，schedule 包装接真实 Iced 生命周期和域内期限回写。
- update 主入口顺序委派领域更新、解包 App 状态、查询模型目录/会话运行时并映射效果；task_board_project_matches/initial_status 等包装只取真实宿主上下文。屏幕开关属于跨导航生命周期；输入清空属于会话运行时；剪贴板、focus/selection任务、文件选择/读取属于原生效果。apply_local_effect/apply_worktree_effects 和 projection_state 都不另实现业务规则。原生/网关结果的错误通知和跨域后续任务仍由适配层绑定。
- 宿主测试残留：bulk 仅真实网关空输入与通知契约；execution 仅源路径禁止SQLite检查、真实Iced任务调度/不建立本地worktree和无本地持久化契约；settings 仅导航元数据保存及真实延迟任务stream。没有遗留纯领域大型App夹具。

## 静态检查与后续验证

- 本任务文件 git diff --check 无输出；独立 rustfmt 解析修改源码成功，skip_children=true，未格式化无关模块。源码核对各新增mod、cfg(test)、全部消息真实映射及调用者，未发现领域反向引用App/宿主include。
- 仓库存在其它并行变更；独立Git索引只暂存本任务差异，共享文件保留其它任务注册与项目路径字段改接。提交前检查本任务暂存差异与换行。
- 后续需用户另行授权后执行：`python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_10_tests -- --test-threads=1`；以及统一宿主 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。当前不能宣称类型检查、通过数、绘制或系统权限结果。
- 风险：未编译的借用结构/回调类型和并行导航字段重命名的集成，需要以后针对最终代码版本验证；不自动启动回归。
