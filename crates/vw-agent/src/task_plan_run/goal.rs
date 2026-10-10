//! 目标模式的任务清单检查与续轮提示。

use super::PlanTaskEntry;
use crate::agent::prompt::multi_agent::recursive_orchestration_policy;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalTaskFilesState {
    Complete,
    Pending(String),
    Blocked(String),
}

pub fn inspect_goal_task_files(
    plan_abs: &Path,
    tasks: &[PlanTaskEntry],
) -> Result<GoalTaskFilesState, String> {
    let mut pending = Vec::new();
    for task in tasks {
        match inspect_goal_task_file(plan_abs, task)? {
            GoalTaskFilesState::Complete => {}
            GoalTaskFilesState::Pending(reason) => pending.push(reason),
            GoalTaskFilesState::Blocked(reason) => {
                return Ok(GoalTaskFilesState::Blocked(reason));
            }
        }
    }

    if pending.is_empty() {
        Ok(GoalTaskFilesState::Complete)
    } else {
        Ok(GoalTaskFilesState::Pending(pending.join("\n\n")))
    }
}

pub fn inspect_goal_task_file(
    plan_abs: &Path,
    task: &PlanTaskEntry,
) -> Result<GoalTaskFilesState, String> {
    let task_path = task_file_path(plan_abs, task);
    if !task_path.exists() {
        return Ok(GoalTaskFilesState::Pending(format!(
            "## {} ({})\n任务文件不存在",
            task.title, task.file
        )));
    }
    let content = std::fs::read_to_string(&task_path)
        .map_err(|error| format!("无法读取目标任务文件 {}: {error}", task_path.display()))?;
    if has_blocked_status(&content) {
        return Ok(GoalTaskFilesState::Blocked(blocked_reason(&task.id, &content)));
    }
    if let Some(reason) = incomplete_task_reason(&content) {
        return Ok(GoalTaskFilesState::Pending(format!(
            "## {} ({})\n{reason}",
            task.title, task.file
        )));
    }
    Ok(GoalTaskFilesState::Complete)
}

fn build_goal_continuation_prompt_base(
    original_goal: &str,
    plan_markdown: &str,
    reason: &str,
    task_files_state: &GoalTaskFilesState,
) -> String {
    let task_state = match task_files_state {
        GoalTaskFilesState::Complete => "所有编号任务文件已满足完成契约。".to_string(),
        GoalTaskFilesState::Pending(summary) => summary.clone(),
        GoalTaskFilesState::Blocked(reason) => format!("明确阻塞：{reason}"),
    };
    format!(
        "这是目标续轮。目标尚未通过独立验收，请继续在当前仓库自主排障、实现并验证，直到满足原始目标。\
\n不要询问用户；不要重复规划。必须运行相关验证并在回复中展示确切命令、退出码和结果。\
\n不要删除编号任务文件。完成任务时保留文件，并精确填写 `状态：已完成`、非空 `完成条件：`、`验证命令：`，以及包含成功退出码的 `验证结果：`；外部凭据、权限或安全风险确实无法自行解决时，将任务文件状态精确标记为 `状态：已阻塞` 并记录原因。\
\n外层编号任务、批次依赖和任务文件所有权保持不变；可在当前目标续轮边界内自主拆分内部工作流。\
\n\n{}\
\n\n# 本轮未通过原因\n{}\
\n\n# 原始目标\n{}\
\n\n# plan.md\n{}\
\n\n# 当前任务文件\n{}",
        recursive_orchestration_policy(),
        reason.trim(),
        original_goal.trim(),
        plan_markdown.trim(),
        task_state
    )
}

pub fn build_goal_continuation_prompt(
    original_goal: &str,
    plan_abs: &Path,
    tasks: &[PlanTaskEntry],
    plan_markdown: &str,
    reason: &str,
    task_files_state: &GoalTaskFilesState,
    execution_evidence: &str,
) -> String {
    let canonical_task_files = tasks
        .iter()
        .map(|task| {
            format!("- [{}] {}：{}", task.id, task.title, task_file_path(plan_abs, task).display())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let evidence = if execution_evidence.trim().is_empty() {
        "（暂无已有执行证据）"
    } else {
        execution_evidence.trim()
    };
    format!(
        "{}\
\n\n# 规范计划目录\n{}\
\n\n# 规范任务文件\n{}\
\n\n# 已有执行证据\n{}\
\n\n先复用上述已有执行证据修正未通过清单，不要重复实现已经达成的产物；只更新列出的规范任务文件。",
        build_goal_continuation_prompt_base(original_goal, plan_markdown, reason, task_files_state,),
        plan_abs.display(),
        canonical_task_files,
        evidence,
    )
}

pub fn goal_turn_session_id(task_id: &str, turn: u32) -> String {
    format!("{}-goal-{turn}", super::prompts::sanitize_task_plan_id_part(task_id))
}

fn has_blocked_status(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        line == "状态：已阻塞" || line == "状态: 已阻塞"
    })
}

fn task_file_path(plan_abs: &Path, task: &PlanTaskEntry) -> std::path::PathBuf {
    let task_path = Path::new(&task.file);
    if task_path.is_absolute() { task_path.to_path_buf() } else { plan_abs.join(task_path) }
}

fn incomplete_task_reason(content: &str) -> Option<String> {
    if !has_completed_status(content) {
        return Some("状态不是 `状态：已完成`".to_string());
    }
    if required_field(content, "完成条件").is_none() {
        return Some("缺少非空完成条件".to_string());
    }
    if required_field(content, "验证命令").is_none() {
        return Some("缺少非空验证命令".to_string());
    }
    let Some(validation_result) = required_field(content, "验证结果") else {
        return Some("缺少非空验证结果".to_string());
    };
    if !has_success_exit_code(validation_result) {
        return Some("验证结果未包含成功退出码 0".to_string());
    }
    None
}

fn has_completed_status(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        line == "状态：已完成" || line == "状态: 已完成"
    })
}

fn required_field<'a>(content: &'a str, field: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix(&format!("{field}："))
            .or_else(|| line.strip_prefix(&format!("{field}:")))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn has_success_exit_code(validation_result: &str) -> bool {
    let compact = validation_result
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    compact.contains("退出码：0")
        || compact.contains("退出码:0")
        || compact.contains("退出代码：0")
        || compact.contains("退出代码:0")
        || compact.contains("exitcode:0")
}

fn blocked_reason(task_id: &str, content: &str) -> String {
    let reason = content
        .lines()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("阻塞原因：")
                .or_else(|| line.strip_prefix("阻塞原因:"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("任务文件明确标记为已阻塞");
    format!("子任务 {task_id} 已阻塞：{reason}")
}
