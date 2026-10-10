//! 处理任务看板状态更新分支，将 UI 消息转换为应用状态变更和异步任务。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

fn is_late_success_after_running_timeout(task: &Task, result: &Result<String, String>) -> bool {
    task.status == TaskStatus::Failed
        && task
            .last_error
            .as_deref()
            .is_some_and(|reason| reason.contains("仍未完成") && reason.contains("自动标记失败"))
        && result.is_ok()
}

fn persist_result_task_via_gateway(
    project_path: String,
    workflow_id: String,
    task: Task,
    action_label: &'static str,
) -> iced::Task<crate::app::Message> {
    let task_id = task.id.clone();
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            persist_updated_tasks_via_gateway(&project_path, &[task], action_label, &workflow_id)
                .await
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::TaskResultPersistenceCompleted {
                project_path: result_project_path.clone(),
                task_id: task_id.clone(),
                result,
            })
        },
    )
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn update(
    app: &mut crate::app::App,
    message: TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    dispatch_task_board_messages!(message,
    TaskBoardMessage::TaskExecutionCompleted { project_path, task_id, result } => {
        if !task_board_project_matches(app, &project_path) {
            return iced::Task::none();
        }
        let final_logs = app.task_board_executor.poll_task_logs_all(&task_id);
        app.task_board_executor.finish_task(&task_id);
        let mut task_to_persist: Option<Task> = None;
        let mut should_recycle_worktree = false;
        let mut recycle_reason: Option<String> = None;
        let staff_handoff_task = iced::Task::none();
        let mut should_schedule_auto_review_tick = false;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                for log in final_logs {
                    append_task_log_stream(task, &log);
                }
            let late_success_after_timeout = is_late_success_after_running_timeout(task, &result);
            if task.status != TaskStatus::Running && !late_success_after_timeout {
                task.add_log("任务状态已变化，忽略本次执行结果".to_string());
            } else {
                match result {
                    Ok(output) => {
                        if late_success_after_timeout {
                            task.add_log("执行结果晚于超时返回，按成功结果继续处理".to_string());
                        }
                        let (body, git_summary, source_branch, target_branch, worktree_path) =
                            split_output_and_git_metadata(&output);
                        let mut message = if body.is_empty() {
                            "执行完成".to_string()
                        } else {
                            format!("执行完成: {}", truncate_for_ui(&body, TASK_LOG_UI_MAX_DETAIL_CHARS))
                        };
                        if let Some(summary) = git_summary {
                            message.push_str(" | ");
                            message.push_str(&summary);
                        }
                        if source_branch.is_some() {
                            task.merge_source_branch = source_branch;
                        }
                        if target_branch.is_some() {
                            task.merge_target_branch = target_branch;
                        }
                        if worktree_path.is_some() {
                            task.selected_worktree_path = worktree_path;
                        }
                        task.add_log(message);
                        task.mark_execution_succeeded();
                        // 正常来源 Running 走 ExecutionSucceeded 边；晚到成功来源为超时标记的
                        // Failed，按恢复语义用 ManualMove 放行，保留既有“晚到成功继续处理”行为。
                        let succeed_intent = if late_success_after_timeout {
                            crate::app::task::TaskTransitionIntent::ManualMove
                        } else {
                            crate::app::task::TaskTransitionIntent::ExecutionSucceeded
                        };
                        if task.apply_task_transition(TaskStatus::CodeComplete, succeed_intent).is_err()
                        {
                            task.mark_paused("执行成功但状态机拒绝进入代码完成".to_string());
                            should_recycle_worktree = true;
                            recycle_reason = task.pause_reason.clone();
                        } else {
                            task.add_log("等待代码完成调度".to_string());
                            should_schedule_auto_review_tick = true;
                        }
                        if matches!(task.status, TaskStatus::CodeReview | TaskStatus::PrSubmitted) {
                        }
                    }
                    Err(error) => {
                        let recycle_detail = format!("任务执行失败: {}", error);
                        task.add_log(format!(
                            "执行失败: {}",
                            truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                        ));
                        task.mark_execution_failed(error);
                        should_recycle_worktree = true;
                        recycle_reason = Some(recycle_detail);
                    }
                }
            }
            task_to_persist = Some(task.clone());
        }
        if should_recycle_worktree {
            if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id) {
                let reason = recycle_reason
                    .as_deref()
                    .map(|reason| truncate_for_ui(reason, TASK_LOG_UI_MAX_DETAIL_CHARS))
                    .unwrap_or_else(|| "任务执行结束".to_string());
                task.add_log(format!("{reason}，等待网关运行时清理 worktree 槽位"));
                task_to_persist = Some(task.clone());
            }
        }
        sync_viewing_logs(app, &task_id);
        if let (Some(project_path), Some(task_clone)) = (&app.project_path, task_to_persist.clone()) {
            app.task_board_tasks_pending_persistence.insert(task_id.clone());
            let persist_task = persist_result_task_via_gateway(
                project_path.clone(),
                app.task_board_settings.workflow_id.clone(),
                task_clone,
                "保存执行结果",
            );
            let mut tasks = vec![
                persist_task,
                staff_handoff_task,
                schedule_scheduler_tick_with_deadline(app),
            ];
            if should_schedule_auto_review_tick {
                tasks.push(schedule_auto_review_tick_with_deadline(app));
            }
            return iced::Task::batch(tasks);
        }
        let mut tasks = vec![staff_handoff_task, schedule_scheduler_tick_with_deadline(app)];
        if should_schedule_auto_review_tick {
            tasks.push(schedule_auto_review_tick_with_deadline(app));
        }
        iced::Task::batch(tasks)
        }
    TaskBoardMessage::TaskCodeReviewCompleted { project_path, task_id, result } => {
        if !task_board_project_matches(app, &project_path) {
            return iced::Task::none();
        }
        let final_logs = app.task_board_executor.poll_task_logs_all(&task_id);
        app.task_board_executor.finish_task(&task_id);
        let mut task_to_persist: Option<Task> = None;
        let mut should_recycle_worktree = false;
        let mut recycle_reason: Option<String> = None;
        let staff_handoff_task = iced::Task::none();
        if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
            for log in final_logs {
                append_task_log_stream(task, &log);
            }
            if task.status != TaskStatus::CodeReview {
                task.add_log("任务状态已变化，忽略本次审核结果".to_string());
            } else {
                match result {
                    Ok(output) => {
                        let review_output = output.trim().to_string();
                        if !review_output.is_empty() {
                            task.add_log(format!(
                                "审核结果: {}",
                                truncate_for_ui(&review_output, TASK_LOG_UI_MAX_DETAIL_CHARS)
                            ));
                        } else {
                            task.add_log("审核结果: 无输出".to_string());
                        }
                        match parse_review_decision(&output) {
                            Ok(()) => {
                                task.add_log(format!(
                                    "合并前校验: source={} target={} worktree={} project_path={}",
                                    task.merge_source_branch.as_deref().unwrap_or("none"),
                                    task.merge_target_branch.as_deref().unwrap_or("none"),
                                    task.selected_worktree_path.as_deref().unwrap_or("none"),
                                    app.project_path.as_deref().unwrap_or("none")
                                ));
                                match validate_ready_for_merge(task, app.project_path.as_deref()) {
                                    Ok(()) => {
                                        task.add_log("代码审核通过，进入合并阶段".to_string());
                                        let _ = task.apply_task_transition(
                                            TaskStatus::PrSubmitted,
                                            crate::app::task::TaskTransitionIntent::ReviewApproved,
                                        );
                                    }
                                    Err(reason) => {
                                        task.mark_paused(reason);
                                        should_recycle_worktree = true;
                                        recycle_reason = task.pause_reason.clone();
                                    }
                                }
                            }
                            Err(reason) => {
                                task.mark_paused(reason);
                                should_recycle_worktree = true;
                                recycle_reason = task.pause_reason.clone();
                            }
                        }
                    }
                    Err(error) => {
                        let detail = truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS);
                        let reason = if is_model_response_timeout_error(&error) {
                            format!("代码审核模型响应超时: {detail}")
                        } else {
                            format!("代码审核执行失败: {detail}")
                        };
                        task.mark_paused(reason);
                        should_recycle_worktree = true;
                        recycle_reason = task.pause_reason.clone();
                    }
                }
            }
            task_to_persist = Some(task.clone());
        }
        if should_recycle_worktree {
            if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id) {
                let reason = recycle_reason
                    .as_deref()
                    .map(|reason| truncate_for_ui(reason, TASK_LOG_UI_MAX_DETAIL_CHARS))
                    .unwrap_or_else(|| "代码审核结束".to_string());
                task.add_log(format!("{reason}，等待网关运行时清理 worktree 槽位"));
                task_to_persist = Some(task.clone());
            }
            sync_viewing_logs(app, &task_id);
            if let (Some(project_path), Some(task_clone)) = (&app.project_path, task_to_persist.clone()) {
                app.task_board_tasks_pending_persistence.insert(task_id.clone());
                let persist_task = persist_result_task_via_gateway(
                    project_path.clone(),
                    app.task_board_settings.workflow_id.clone(),
                    task_clone,
                    "保存审核结果",
                );
                return iced::Task::batch(vec![persist_task, staff_handoff_task]);
            }
            return staff_handoff_task;
        }
        sync_viewing_logs(app, &task_id);
        if let (Some(project_path), Some(task_clone)) = (&app.project_path, task_to_persist.clone()) {
            app.task_board_tasks_pending_persistence.insert(task_id.clone());
            let persist_task = persist_result_task_via_gateway(
                project_path.clone(),
                app.task_board_settings.workflow_id.clone(),
                task_clone,
                "保存审核结果",
            );
            return iced::Task::batch(vec![persist_task, staff_handoff_task]);
        }
        staff_handoff_task
    }
    TaskBoardMessage::TaskMergeCompleted { project_path, task_id, result } => {
        if !task_board_project_matches(app, &project_path) {
            return iced::Task::none();
        }
        let final_logs = app.task_board_executor.poll_task_logs_all(&task_id);
        app.task_board_executor.finish_task(&task_id);
        let project_path_snapshot = app.project_path.clone();
        let mut merge_succeeded = false;
        let mut recycle_reason: Option<String> = None;
        let mut task_to_persist: Option<Task> = None;
        let staff_handoff_task = iced::Task::none();
        if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
            for log in final_logs {
                append_task_log_stream(task, &log);
            }
            let late_success_after_timeout = task.status == TaskStatus::Paused
                && task.pause_reason.as_deref().is_some_and(|reason| {
                    reason.contains("合并阶段") && reason.contains("自动暂停")
                })
                && result.is_ok();
            if task.status != TaskStatus::PrSubmitted && !late_success_after_timeout {
                task.add_log("任务状态已变化，忽略本次合并结果".to_string());
            } else {
                match result {
                    Ok(output) => {
                        if late_success_after_timeout {
                            task.add_log("合并结果晚于超时暂停返回，按成功结果完成任务".to_string());
                        }
                        if output.trim().is_empty() {
                            task.add_log("合并完成".to_string());
                        } else {
                            task.add_log(format!(
                                "合并完成: {}",
                                truncate_for_ui(&output, TASK_LOG_UI_MAX_DETAIL_CHARS)
                            ));
                        }
                        task.mark_execution_succeeded();
                        // 正常来源 PrSubmitted 走 MergeSucceeded 边；晚到成功来源为超时暂停的
                        // Paused，按恢复语义用 ManualMove 放行，保留既有“晚到成功完成任务”行为。
                        let merge_intent = if late_success_after_timeout {
                            crate::app::task::TaskTransitionIntent::ManualMove
                        } else {
                            crate::app::task::TaskTransitionIntent::MergeSucceeded
                        };
                        if task.apply_task_transition(TaskStatus::Completed, merge_intent).is_err() {
                            task.mark_paused("合并成功但状态机拒绝进入任务完成".to_string());
                        } else {
                            merge_succeeded = true;
                        }
                    }
                    Err(error) => {
                        let detail = truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS);
                        task.mark_paused(format!("合并失败: {}", detail));
                        recycle_reason = task.pause_reason.clone();
                        task.add_log(format!(
                            "合并失败详情: source={} target={} worktree={} project_path={} error={}",
                            task.merge_source_branch.as_deref().unwrap_or("none"),
                            task.merge_target_branch.as_deref().unwrap_or("none"),
                            task.selected_worktree_path.as_deref().unwrap_or("none"),
                            project_path_snapshot.as_deref().unwrap_or("none"),
                            detail
                        ));
                        record_task_worktree_recycle_after_failure(task, "合并失败");
                    }
                }
            }
            task_to_persist = Some(task.clone());
        }
        sync_viewing_logs(app, &task_id);
        if !merge_succeeded && !app.task_board_manual_merge_queue.is_empty() {
            app.task_board_manual_merge_queue.clear();
            app.push_notification("批量合并已停止，请先处理当前合并失败".to_string());
        }
        let continue_manual_merge = if merge_succeeded && !app.task_board_manual_merge_queue.is_empty() {
            iced::Task::done(Message::TaskBoard(TaskBoardMessage::ManualMergeQueueTick))
        } else {
            iced::Task::none()
        };
        if !merge_succeeded && let Some(reason) = recycle_reason.as_deref()
            && let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id)
        {
            task.add_log(format!(
                "{}，等待网关运行时清理 worktree 槽位",
                truncate_for_ui(reason, TASK_LOG_UI_MAX_DETAIL_CHARS)
            ));
            task_to_persist = Some(task.clone());
        }
        if let (Some(project_path), Some(task_clone)) = (&app.project_path, task_to_persist) {
            app.task_board_tasks_pending_persistence.insert(task_id.clone());
            let persist_task = persist_result_task_via_gateway(
                project_path.clone(),
                app.task_board_settings.workflow_id.clone(),
                task_clone,
                "保存合并结果",
            );
            return iced::Task::batch(vec![
                persist_task,
                staff_handoff_task,
                continue_manual_merge,
            ]);
        }
        iced::Task::batch(vec![staff_handoff_task, continue_manual_merge])
    }
        )
}
#[cfg(test)]
#[path = "execution_results_tests.rs"]
mod execution_results_tests;
