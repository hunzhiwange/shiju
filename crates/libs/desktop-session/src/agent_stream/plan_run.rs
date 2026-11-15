use super::*;

pub(super) fn plan_run_event_type(event: &vw_api_task::TaskPlanRunStreamEventDto) -> &'static str {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { .. } => "planning_started",
        vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { .. } => "session_delta",
        vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => "session_waiting",
        vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { .. } => "plan_created",
        vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { .. } => "batch_started",
        vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { .. } => "batch_finished",
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { .. } => {
            "goal_turn_started"
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { .. } => "goal_evaluated",
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { .. } => "subtask_started",
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { .. } => "subtask_finished",
        vw_api_task::TaskPlanRunStreamEventDto::Blocked { .. } => "blocked",
        vw_api_task::TaskPlanRunStreamEventDto::Finished { .. } => "finished",
        vw_api_task::TaskPlanRunStreamEventDto::Error { .. } => "error",
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. } => "cancelled",
    }
}

pub(super) fn plan_run_event_task_id(event: &vw_api_task::TaskPlanRunStreamEventDto) -> &str {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Blocked { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Finished { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Cancelled { task_id, .. } => task_id,
        vw_api_task::TaskPlanRunStreamEventDto::Error { task_id, .. } => {
            task_id.as_deref().unwrap_or("unknown")
        }
    }
}

pub(super) fn plan_run_event_session_id(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Blocked { session_id, .. } => {
            Some(session_id.as_str())
        }
        vw_api_task::TaskPlanRunStreamEventDto::Finished { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Cancelled { session_id, .. } => {
            session_id.as_deref()
        }
        vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Error { .. } => None,
    }
}

pub(super) fn plan_run_event_session_title(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
) -> Option<String> {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. } => {
            Some(format!("规划 · {task_id}"))
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted {
            subtask_id, title, ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished {
            subtask_id,
            title,
            ..
        } => {
            let title = title.trim();
            Some(if title.is_empty() {
                format!("子任务 · {subtask_id}")
            } else {
                title.to_string()
            })
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, .. } => {
            Some(format!("目标续轮 · 第 {turn} 轮"))
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { turn, .. } => {
            Some(format!("目标验收 · 第 {turn} 轮"))
        }
        _ => None,
    }
}

pub(super) fn plan_run_session_started_message(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
    parent_request_id: u64,
    parent_session_id: &str,
    directory: &str,
    query: &str,
) -> Option<StreamMessage> {
    let session_title = plan_run_event_session_title(event)?;
    let (session_id, initial_query) = match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { session_id, .. } => {
            (session_id, query.to_string())
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. } => {
            (session_id, session_title.clone())
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted {
            session_id,
            reason,
            ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated {
            session_id, reason, ..
        } => (session_id, reason.clone()),
        _ => return None,
    };
    let title_feedback = match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. } => {
            Some(desktop_i18n::input::Feedback::new("规划 · %s", vec![task_id.clone().into()]))
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted {
            subtask_id, title, ..
        } if title.trim().is_empty() => Some(desktop_i18n::input::Feedback::new(
            "子任务 · %s",
            vec![subtask_id.clone().into()],
        )),
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, .. } => Some(
            desktop_i18n::input::Feedback::new("目标续轮 · 第 %u 轮", vec![(*turn).into()]),
        ),
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { turn, .. } => Some(
            desktop_i18n::input::Feedback::new("目标验收 · 第 %u 轮", vec![(*turn).into()]),
        ),
        _ => None,
    };
    Some(StreamMessage::PlanRunSessionStarted {
        title_feedback,
        parent_request_id,
        parent_session_id: parent_session_id.to_string(),
        session_id: session_id.clone(),
        title: session_title,
        directory: directory.to_string(),
        query: initial_query,
    })
}

pub(super) fn plan_run_session_finished_id(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Blocked { session_id, .. } => {
            Some(session_id)
        }
        vw_api_task::TaskPlanRunStreamEventDto::Finished { session_id, .. } => {
            session_id.as_deref()
        }
        _ => None,
    }
}

pub(super) fn plan_run_session_cancelled_id(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
) -> Option<&str> {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { session_id, .. } => {
            session_id.as_deref()
        }
        _ => None,
    }
}

pub(super) fn plan_run_tool_call_id(event: &vw_api_task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { task_id, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { task_id, .. } => {
            format!("plan-run-{task_id}-planning")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SessionDelta {
            task_id, session_id, ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting {
            task_id,
            session_id,
            ..
        } => format!("plan-run-{task_id}-session-{session_id}"),
        vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { task_id, batch, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { task_id, batch, .. } => {
            format!("plan-run-{task_id}-batch-{batch}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted {
            task_id, turn, ..
        } => format!("plan-run-{task_id}-goal-turn-{turn}"),
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { task_id, turn, .. } => {
            format!("plan-run-{task_id}-goal-evaluation-{turn}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id,
            subtask_id,
            ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id,
            subtask_id,
            ..
        } => format!("plan-run-{task_id}-subtask-{subtask_id}"),
        vw_api_task::TaskPlanRunStreamEventDto::Blocked { task_id, subtask_id, .. } => {
            format!("plan-run-{}-blocked-{}", task_id, subtask_id.as_deref().unwrap_or("task"))
        }
        vw_api_task::TaskPlanRunStreamEventDto::Finished { task_id, .. } => {
            format!("plan-run-{task_id}-finished")
        }
        vw_api_task::TaskPlanRunStreamEventDto::Error { task_id, .. } => {
            format!("plan-run-{}-error", task_id.as_deref().unwrap_or("unknown"))
        }
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { task_id, .. } => {
            format!("plan-run-{task_id}-cancelled")
        }
    }
}

pub(super) fn plan_run_event_status(event: &vw_api_task::TaskPlanRunStreamEventDto) -> &'static str {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated {
            achieved: false, ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { .. } => "running",
        vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => "waiting",
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished {
            success: false, ..
        }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { success: false, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Blocked { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Error { .. } => "error",
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. } => "cancelled",
        vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { .. }
        | vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { achieved: true, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::Finished { .. } => "completed",
    }
}

pub(super) fn plan_run_event_summary(event: &vw_api_task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { .. } => {
            "规划任务".to_string()
        }
        vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { .. } => {
            "规划会话".to_string()
        }
        vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting { .. } => {
            "等待后代智能体".to_string()
        }
        vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { subtask_count, .. } => {
            format!("计划已创建 · {subtask_count} 个子任务")
        }
        vw_api_task::TaskPlanRunStreamEventDto::BatchStarted { batch, .. } => {
            format!("批次 {batch}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { batch, .. } => {
            format!("批次 {batch}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, .. } => {
            format!("目标续轮 · 第 {turn} 轮")
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated { turn, achieved, .. } => {
            let status = if *achieved { "通过" } else { "未通过" };
            format!("目标验收 · 第 {turn} 轮 · {status}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { batch, title, .. } => {
            format!("批次 {batch} · {title}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished {
            batch,
            title,
            success,
            ..
        } => {
            let suffix = if *success { "完成" } else { "失败" };
            format!("批次 {batch} · {title} · {suffix}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::Blocked { .. } => "执行受阻".to_string(),
        vw_api_task::TaskPlanRunStreamEventDto::Finished { .. } => {
            "计划执行完成".to_string()
        }
        vw_api_task::TaskPlanRunStreamEventDto::Error { .. } => "计划执行失败".to_string(),
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. } => {
            "计划执行已取消".to_string()
        }
    }
}

pub(super) fn plan_run_event_output_text(event: &vw_api_task::TaskPlanRunStreamEventDto) -> String {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { plan_dir, .. } => {
            format!("正在规划任务，计划目录: {plan_dir}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SessionDelta { delta, .. } => delta.clone(),
        vw_api_task::TaskPlanRunStreamEventDto::SessionWaiting {
            active_descendant_session_ids,
            ..
        } => format!(
            "等待 {} 个后代智能体完成: {}",
            active_descendant_session_ids.len(),
            active_descendant_session_ids.join(", ")
        ),
        vw_api_task::TaskPlanRunStreamEventDto::PlanCreated {
            plan_dir,
            plan_md_path,
            tasks_json_path,
            subtask_count,
            ..
        } => format!(
            "计划已创建，共 {subtask_count} 个子任务。\nplan.md: {plan_md_path}\ntasks.json: {tasks_json_path}\n目录: {plan_dir}"
        ),
        vw_api_task::TaskPlanRunStreamEventDto::BatchStarted {
            batch, subtask_ids, ..
        } => format!("批次 {batch} 开始执行: {}", subtask_ids.join(", ")),
        vw_api_task::TaskPlanRunStreamEventDto::BatchFinished {
            batch,
            subtask_ids,
            success,
            ..
        } => {
            let status = if *success { "执行完毕" } else { "执行失败" };
            format!("批次 {batch} {status}: {}", subtask_ids.join(", "))
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalTurnStarted { turn, reason, .. } => {
            format!("目标第 {turn} 轮续行：{reason}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::GoalEvaluated {
            turn,
            achieved,
            reason,
            ..
        } => {
            let status = if *achieved { "通过" } else { "未通过，继续执行" };
            format!("目标第 {turn} 轮验收{status}：{reason}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted {
            task_file, title, ..
        } => format!("子任务开始: {title}\n任务文件: {task_file}"),
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished {
            success,
            output,
            error,
            task_file,
            ..
        } => {
            if *success {
                format!(
                    "子任务完成，任务文件已处理: {task_file}\n{}",
                    output.as_deref().unwrap_or_default()
                )
            } else if output.is_some() {
                format!(
                    "子任务验收未通过，任务文件待修正: {task_file}\n{}",
                    error.as_deref().unwrap_or("验收未通过")
                )
            } else {
                format!(
                    "子任务失败，任务文件保留: {task_file}\n{}",
                    error.as_deref().unwrap_or("执行失败")
                )
            }
        }
        vw_api_task::TaskPlanRunStreamEventDto::Blocked { reason, .. } => {
            format!("执行受阻: {reason}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::Finished {
            status,
            output,
            plan_dir,
            ..
        } => format!("状态: {status}\n计划目录: {plan_dir}\n{output}"),
        vw_api_task::TaskPlanRunStreamEventDto::Error { message, .. } => {
            format!("计划执行失败: {message}")
        }
        vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. } => {
            "计划执行已取消".to_string()
        }
    }
}

pub(super) fn plan_run_event_task_path(event: &vw_api_task::TaskPlanRunStreamEventDto) -> Option<&str> {
    match event {
        vw_api_task::TaskPlanRunStreamEventDto::PlanningStarted { plan_dir, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { plan_dir, .. } => {
            Some(plan_dir)
        }
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskStarted { task_file, .. }
        | vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { task_file, .. } => {
            Some(task_file)
        }
        _ => None,
    }
    .map(String::as_str)
    .map(str::trim)
    .filter(|path| !path.is_empty())
}

pub(super) fn plan_run_event_is_terminal(event: &vw_api_task::TaskPlanRunStreamEventDto) -> bool {
    matches!(
        event,
        vw_api_task::TaskPlanRunStreamEventDto::Finished { .. }
            | vw_api_task::TaskPlanRunStreamEventDto::Blocked { .. }
            | vw_api_task::TaskPlanRunStreamEventDto::Error { .. }
            | vw_api_task::TaskPlanRunStreamEventDto::Cancelled { .. }
    )
}

pub(super) fn plan_run_tool_block(
    event: &vw_api_task::TaskPlanRunStreamEventDto,
    session_directory: &str,
) -> String {
    use serde_json::json;

    let summary = plan_run_event_summary(event);
    let output_value = workflow_preview_outputs(&plan_run_event_output_text(event), None, false);
    let output =
        serde_json::to_string(&output_value).unwrap_or_else(|_| "计划执行事件".to_string());
    let error = matches!(
        event,
        vw_api_task::TaskPlanRunStreamEventDto::SubtaskFinished { success: false, .. }
            | vw_api_task::TaskPlanRunStreamEventDto::BatchFinished { success: false, .. }
            | vw_api_task::TaskPlanRunStreamEventDto::Blocked { .. }
            | vw_api_task::TaskPlanRunStreamEventDto::Error { .. }
    )
    .then(|| plan_run_event_output_text(event));
    let mut metadata = json!({
        "canonical_tool_id": "workflow_node",
        "plan_run": true,
        "event_type": plan_run_event_type(event),
        "task_id": plan_run_event_task_id(event),
    });
    if let Some(session_id) = plan_run_event_session_id(event) {
        metadata["session_id"] = json!(session_id);
        metadata["session_directory"] = json!(session_directory);
    }
    if let Some(title) = plan_run_event_session_title(event) {
        metadata["title"] = json!(title);
    }
    if let Some(task_path) = plan_run_event_task_path(event) {
        metadata["task_path"] = json!(task_path);
    }
    if let vw_api_task::TaskPlanRunStreamEventDto::PlanCreated { subtask_count, .. } = event
    {
        metadata["subtask_count"] = json!(subtask_count);
    }
    let payload = json!({
        "tool_call_id": plan_run_tool_call_id(event),
        "status": plan_run_event_status(event),
        "input": summary,
        "summary": summary,
        "output": output,
        "error": error,
        "metadata": metadata,
    });
    let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!("tool workflow_node\n{payload}\n")
}
