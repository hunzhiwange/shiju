//! 目标模式的任务文件检查、独立验收契约与续轮提示。

use super::PlanTaskEntry;
use crate::agent::prompt::multi_agent::{
    descendant_acceptance_policy, recursive_orchestration_policy,
};
use serde::Deserialize;
use std::path::Path;

const MAX_EVIDENCE_CHARS: usize = 24_000;
const MAX_TASK_FILE_CHARS: usize = 4_000;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalEvaluation {
    pub achieved: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalTaskFilesState {
    Complete,
    Pending(String),
    Blocked(String),
}

pub fn parse_goal_evaluation(output: &str) -> Result<GoalEvaluation, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("目标评估器返回空响应".to_string());
    }
    let evaluation: GoalEvaluation = serde_json::from_str(trimmed)
        .map_err(|error| format!("目标评估器必须返回严格 JSON: {error}"))?;
    if evaluation.reason.trim().is_empty() {
        return Err("目标评估器 reason 不能为空".to_string());
    }
    Ok(evaluation)
}

pub fn inspect_goal_task_files(
    plan_abs: &Path,
    tasks: &[PlanTaskEntry],
) -> Result<GoalTaskFilesState, String> {
    let mut pending = Vec::new();
    for task in tasks {
        let task_path = if Path::new(&task.file).is_absolute() {
            Path::new(&task.file).to_path_buf()
        } else {
            plan_abs.join(&task.file)
        };
        if !task_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&task_path)
            .map_err(|error| format!("无法读取目标任务文件 {}: {error}", task_path.display()))?;
        if has_blocked_status(&content) {
            return Ok(GoalTaskFilesState::Blocked(blocked_reason(
                &task.id, &content,
            )));
        }
        pending.push(format!(
            "## {} ({})\n{}",
            task.title,
            task.file,
            truncate_chars(&content, MAX_TASK_FILE_CHARS)
        ));
    }

    if pending.is_empty() {
        Ok(GoalTaskFilesState::Complete)
    } else {
        Ok(GoalTaskFilesState::Pending(pending.join("\n\n")))
    }
}

pub fn build_goal_evaluation_prompt(
    original_goal: &str,
    plan_markdown: &str,
    execution_evidence: &str,
    task_files_state: &GoalTaskFilesState,
) -> String {
    let task_state = match task_files_state {
        GoalTaskFilesState::Complete => "所有编号任务文件均已删除。".to_string(),
        GoalTaskFilesState::Pending(summary) => {
            format!("以下编号任务文件仍存在：\n{summary}")
        }
        GoalTaskFilesState::Blocked(reason) => format!("任务已明确阻塞：{reason}"),
    };
    format!(
        "你是独立目标验收器，只判断已有证据，不执行工具或命令。\
\n请根据原始目标、计划验收条件、执行会话展示的确切命令/退出码/结果，以及编号任务文件状态判断目标是否真正完成。\
\n没有新鲜验证证据、验证失败、约束未满足或编号任务文件仍存在时，achieved 必须为 false。\
\n\n{}\
\n只返回一个严格 JSON 对象，不要 Markdown、代码围栏或额外文字：\
\n{{\"achieved\":true或false,\"reason\":\"简洁且可执行的中文原因\"}}\
\n\n# 原始目标\n{}\
\n\n# plan.md\n{}\
\n\n# 执行证据\n{}\
\n\n# 编号任务文件状态\n{}",
        descendant_acceptance_policy(),
        original_goal.trim(),
        plan_markdown.trim(),
        truncate_chars(execution_evidence, MAX_EVIDENCE_CHARS),
        task_state
    )
}

pub fn build_goal_continuation_prompt(
    original_goal: &str,
    plan_markdown: &str,
    reason: &str,
    task_files_state: &GoalTaskFilesState,
) -> String {
    let task_state = match task_files_state {
        GoalTaskFilesState::Complete => "编号任务文件已全部删除，但独立验收仍未通过。".to_string(),
        GoalTaskFilesState::Pending(summary) => summary.clone(),
        GoalTaskFilesState::Blocked(reason) => format!("明确阻塞：{reason}"),
    };
    format!(
        "这是目标续轮。目标尚未通过独立验收，请继续在当前仓库自主排障、实现并验证，直到满足原始目标。\
\n不要询问用户；不要重复规划。必须运行相关验证并在回复中展示确切命令、退出码和结果。\
\n只有获得新鲜成功证据后才能删除对应编号任务文件；外部凭据、权限或安全风险确实无法自行解决时，将任务文件状态精确标记为 `状态：已阻塞` 并记录原因。\
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

pub fn goal_turn_session_id(task_id: &str, turn: u32) -> String {
    format!(
        "{}-goal-{turn}",
        super::prompts::sanitize_task_plan_id_part(task_id)
    )
}

pub fn goal_evaluation_session_id(task_id: &str, turn: u32) -> String {
    format!(
        "{}-goal-eval-{turn}",
        super::prompts::sanitize_task_plan_id_part(task_id)
    )
}

fn has_blocked_status(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        line == "状态：已阻塞" || line == "状态: 已阻塞"
    })
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

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{truncated}…")
    } else if value.is_empty() {
        "(空)".to_string()
    } else {
        truncated
    }
}
