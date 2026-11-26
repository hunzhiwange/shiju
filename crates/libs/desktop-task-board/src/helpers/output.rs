//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// 执行 split_output_and_git_metadata 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn split_output_and_git_metadata(
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

/// 执行 build_task_prompt_from_input 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn build_task_prompt_from_input(
    raw: &str,
    _priority: Option<u32>,
) -> String {
    raw.trim().to_string()
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
