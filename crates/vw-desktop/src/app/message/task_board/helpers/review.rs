//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// 执行 split_output_and_git_metadata 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn split_output_and_git_metadata(
    output: &str,
) -> (String, Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut summary: Option<String> = None;
    let mut source_branch: Option<String> = None;
    let mut target_branch: Option<String> = None;
    let mut worktree_path: Option<String> = None;
    let mut body_lines: Vec<String> = Vec::new();

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("__VW_GIT_SUMMARY__:") {
            let value = value.trim();
            if !value.is_empty() {
                summary = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("__VW_GIT_SOURCE_BRANCH__:") {
            let value = value.trim();
            if !value.is_empty() {
                source_branch = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("__VW_GIT_TARGET_BRANCH__:") {
            let value = value.trim();
            if !value.is_empty() {
                target_branch = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("__VW_GIT_WORKTREE_PATH__:") {
            let value = value.trim();
            if !value.is_empty() {
                worktree_path = Some(value.to_string());
            }
        } else {
            body_lines.push(line.to_string());
        }
    }

    let body = body_lines.join("\n").trim().to_string();
    (body, summary, source_branch, target_branch, worktree_path)
}

/// 执行 validate_ready_for_merge 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn validate_ready_for_merge(
    task: &Task,
    project_path: Option<&str>,
) -> Result<(), String> {
    if project_path.is_none() {
        return Err("缺少项目路径，无法执行合并".to_string());
    }

    let source_branch = task.merge_source_branch.as_deref().unwrap_or("").trim();
    if source_branch.is_empty() || source_branch == "HEAD" {
        return Err("缺少有效 source branch，无法自动合并".to_string());
    }

    if let Some(target_branch) =
        task.merge_target_branch.as_deref().map(str::trim).filter(|branch| !branch.is_empty())
    {
        if target_branch == "HEAD" {
            return Err("无效 target branch，无法自动合并: HEAD".to_string());
        }
        if source_branch == target_branch {
            return Err(format!(
                "source branch 与 target branch 相同，无法自动合并: {}",
                source_branch
            ));
        }
    }

    let selected_worktree_path = task.selected_worktree_path.as_deref().unwrap_or("").trim();
    if selected_worktree_path.is_empty() {
        return Err("缺少任务 worktree，无法自动合并".to_string());
    }

    Ok(())
}

/// 执行 build_task_prompt_from_input 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn build_task_prompt_from_input(
    raw: &str,
    _priority: Option<u32>,
    _subtasks: &[String],
) -> String {
    raw.trim().to_string()
}

/// 执行 parse_review_decision 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn parse_review_decision(output: &str) -> Result<(), String> {
    fn from_json(value: &serde_json::Value) -> Option<Result<(), String>> {
        let status = value.get("status").and_then(|v| v.as_str())?;
        if status == "error" {
            let reason = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .unwrap_or("风险太高")
                .to_string();
            return Some(Err(reason));
        }
        if status == "ok" {
            return Some(Ok(()));
        }
        None
    }

    fn collect_json_objects(text: &str) -> Vec<String> {
        let mut objects = Vec::new();
        let mut start: Option<usize> = None;
        let mut depth = 0usize;
        for (idx, ch) in text.char_indices() {
            if ch == '{' {
                if depth == 0 {
                    start = Some(idx);
                }
                depth = depth.saturating_add(1);
                continue;
            }
            if ch == '}' {
                if depth == 0 {
                    continue;
                }
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(begin) = start.take()
                {
                    objects.push(text[begin..=idx].to_string());
                }
            }
        }
        objects
    }

    fn parse_candidate_json(text: &str) -> Option<serde_json::Value> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            return Some(value);
        }
        let unescaped = text.replace("\\\"", "\"");
        if unescaped != text
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&unescaped)
        {
            return Some(value);
        }
        None
    }

    fn find_decision_in_value(value: &serde_json::Value) -> Option<Result<(), String>> {
        if let Some(decision) = from_json(value) {
            return Some(decision);
        }
        match value {
            serde_json::Value::Array(items) => items.iter().rev().find_map(find_decision_in_value),
            serde_json::Value::Object(map) => map.values().find_map(find_decision_in_value),
            serde_json::Value::String(text) => find_decision_in_text(text),
            _ => None,
        }
    }

    fn find_decision_in_text(text: &str) -> Option<Result<(), String>> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(value) = parse_candidate_json(trimmed)
            && let Some(decision) = find_decision_in_value(&value)
        {
            return Some(decision);
        }
        for line in trimmed.lines().rev() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(value) = parse_candidate_json(line)
                && let Some(decision) = find_decision_in_value(&value)
            {
                return Some(decision);
            }
        }
        for json_text in collect_json_objects(trimmed).into_iter().rev() {
            if let Some(value) = parse_candidate_json(&json_text)
                && let Some(decision) = find_decision_in_value(&value)
            {
                return Some(decision);
            }
        }
        None
    }

    if let Some(decision) = find_decision_in_text(output) {
        return decision;
    }

    if output.contains("\"status\":\"error\"") {
        return Err("风险太高".to_string());
    }

    Err("审核结果缺少可解析结论".to_string())
}
#[cfg(test)]
#[path = "review_tests.rs"]
mod review_tests;
