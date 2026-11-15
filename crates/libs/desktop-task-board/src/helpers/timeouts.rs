//! 有界超时扫描与评审候选选择。
use desktop_task_model::{Task, TaskStatus, TaskBoardSettings};
use runtime_time::now_ms;
use super::execution::{
    TASK_EXECUTION_SCAN_BUDGET_MS, TASK_TIMEOUT_SCAN_BATCH,
    TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER, minutes_to_ms, failed_retry_max_attempts,
    running_timeout_anchor_ms, task_has_current_timeout_grace,
    record_task_worktree_recycle_after_failure, model_timeout_retry_wait_elapsed,
    retry_backoff_minutes,
};

pub fn apply_execution_timeouts(
    tasks: &mut [Task],
    settings: &TaskBoardSettings,
    running_tasks: &[String],
    merge_target_locks: &[(String, String)],
    project_path: Option<&str>,
    cursor: &mut usize,
    tick_started_at: web_time::Instant,
) -> (Vec<Task>, Vec<String>) {
    let now = now_ms();
    let running_timeout_ms = minutes_to_ms(settings.running_timeout_minutes);
    let pr_stall_timeout_secs = super::settings::pr_submitted_stall_timeout_secs(settings);
    let pr_stall_timeout_ms = pr_stall_timeout_secs.saturating_mul(1000);
    let mut changed_tasks = Vec::new();
    let mut finished_task_ids = Vec::new();

    let mut timed_out_running_ids = Vec::new();
    let mut stalled_merge_ids = Vec::new();

    if tasks.is_empty() {
        *cursor = 0;
        return (changed_tasks, finished_task_ids);
    }
    if *cursor >= tasks.len() {
        *cursor = 0;
    }
    let max_batch = TASK_TIMEOUT_SCAN_BATCH.min(tasks.len());
    let mut scanned = 0usize;

    while scanned < max_batch {
        if scanned > 0
            && tick_started_at.elapsed()
                >= std::time::Duration::from_millis(TASK_EXECUTION_SCAN_BUDGET_MS)
        {
            break;
        }
        let idx = (*cursor + scanned) % tasks.len();
        let task = &mut tasks[idx];
        if task.deleted || task.archived {
            scanned += 1;
            continue;
        }
        match task.status {
            TaskStatus::Failed => {}
            TaskStatus::Planning | TaskStatus::Running => {
                let timeout_anchor = running_timeout_anchor_ms(task);
                if now.saturating_sub(timeout_anchor) >= running_timeout_ms {
                    if !task_has_current_timeout_grace(task) {
                        task.execution_started_at_ms = Some(now);
                        task.add_localized_log(
                            "%s: %s超过 %s 分钟未完成，本次不标记失败，继续等待一个周期",
                            vec![
                                (TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER).to_string().into(),
                                (task.status.label()).to_string().into(),
                                (settings.running_timeout_minutes.max(1))
                                    .to_string()
                                    .into(),
                            ],
                        );
                    } else {
                        let timeout_reason = format!(
                            "{}连续超过 {} 分钟仍未完成，已自动标记失败",
                            task.status.label(),
                            settings.running_timeout_minutes.max(1)
                        );
                        task.mark_execution_failed(timeout_reason.clone());
                        record_task_worktree_recycle_after_failure(task, "任务执行连续超时");
                        timed_out_running_ids.push(task.id.clone());
                    }
                    changed_tasks.push(task.clone());
                }
            }
            TaskStatus::PrSubmitted => {
                if !running_tasks.contains(&task.id) {
                    scanned += 1;
                    continue;
                }
                let inactive_for_ms = now.saturating_sub(task.last_active_at_ms);
                if inactive_for_ms >= pr_stall_timeout_ms {
                    if let Some(project_path) = project_path {
                        let holder = task
                            .merge_target_branch
                            .as_deref()
                            .and_then(|target_branch| {
                                merge_target_locks
                                    .iter()
                                    .find(|(target, _)| target == target_branch.trim())
                                    .map(|(_, task_id)| task_id.clone())
                            })
                            .unwrap_or_else(|| "none".to_string());
                        let selected_worktree = task
                            .selected_worktree_path
                            .clone()
                            .unwrap_or_else(|| "none".to_string());
                        let target_branch =
                            task.merge_target_branch.as_deref().unwrap_or("none").to_string();
                        task.add_localized_log(
                            "合并阶段超时检测: inactive_ms=%s worktree=%s target=%s lock_holder=%s",
                            vec![
                                (inactive_for_ms).to_string().into(),
                                (selected_worktree).to_string().into(),
                                (target_branch).to_string().into(),
                                (holder).to_string().into(),
                            ],
                        );
                        task.add_localized_log(
                            "合并阶段超时暂停: target=%s project_path=%s，gateway 锁由执行守卫释放",
                            vec![
                                (target_branch).to_string().into(),
                                (project_path).to_string().into(),
                            ],
                        );
                    }
                    let pause_reason =
                        format!("合并阶段静默超过 {} 秒未完成，已自动暂停", pr_stall_timeout_secs);
                    task.mark_paused(pause_reason.clone());
                    record_task_worktree_recycle_after_failure(task, "合并阶段超时暂停");
                    stalled_merge_ids.push(task.id.clone());
                    changed_tasks.push(task.clone());
                }
            }
            _ => {}
        }
        scanned += 1;
    }
    if scanned > 0 {
        *cursor =
            (*cursor + scanned) % tasks.len();
    }

    for task_id in timed_out_running_ids {
        finished_task_ids.push(task_id);
    }
    for task_id in stalled_merge_ids {
        finished_task_ids.push(task_id);
    }

    if !tasks.is_empty()
        && *cursor >= tasks.len()
    {
        *cursor = 0;
    }

    (changed_tasks, finished_task_ids)
}

pub fn pick_next_code_review_task(
    tasks: &[Task], settings: &TaskBoardSettings,
    exclude_ids: &[String],
) -> Option<String> {
    let now = now_ms();
    let max_attempts = failed_retry_max_attempts(settings);

    tasks
        .iter()
        .filter(|task| {
            !task.deleted
                && !task.archived
                && task.status == TaskStatus::CodeReview
                && !exclude_ids.contains(&task.id)
                && model_timeout_retry_wait_elapsed(
                    task,
                    now,
                    minutes_to_ms(retry_backoff_minutes(
                        settings.failed_retry_minutes,
                        task.retry_count,
                    )),
                    max_attempts,
                )
        })
        .min_by_key(|task| (task.priority, task.order))
        .map(|task| task.id.clone())
}
