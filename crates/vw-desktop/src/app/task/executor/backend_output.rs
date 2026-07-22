//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// 执行 extract_opencode_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_opencode_message(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let candidates = [
        value.pointer("/message/content").and_then(|v| v.as_str()),
        value.pointer("/message").and_then(|v| v.as_str()),
        value.pointer("/content").and_then(|v| v.as_str()),
        value.pointer("/text").and_then(|v| v.as_str()),
        value.pointer("/delta").and_then(|v| v.as_str()),
        value.pointer("/result/content").and_then(|v| v.as_str()),
        value.pointer("/output").and_then(|v| v.as_str()),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|text| !text.trim().is_empty())
        .map(|text| text.trim().to_string())
}

/// 执行 extract_opencode_error_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_opencode_error_message(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(|v| v.as_str()) != Some("error") {
        return None;
    }

    let name = value
        .pointer("/error/name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let message = [
        value.pointer("/error/data/message").and_then(|v| v.as_str()),
        value.pointer("/error/message").and_then(|v| v.as_str()),
        value.pointer("/message").and_then(|v| v.as_str()),
        value.pointer("/error/data/error/message").and_then(|v| v.as_str()),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|v| !v.is_empty());

    match (name, message) {
        (Some(n), Some(m)) => Some(format!("{n}: {m}")),
        (Some(n), None) => Some(n.to_string()),
        (None, Some(m)) => Some(m.to_string()),
        (None, None) => Some("未知错误".to_string()),
    }
}

/// 执行 extract_opencode_terminal_error 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_opencode_terminal_error(stdout: &str, stderr: &str) -> Option<String> {
    let stdout_last = stdout.lines().rev().find(|line| !line.trim().is_empty());
    if let Some(message) = stdout_last.and_then(extract_opencode_error_message) {
        return Some(message);
    }

    let stderr_last = stderr.lines().rev().find(|line| !line.trim().is_empty());
    stderr_last.and_then(extract_opencode_error_message)
}

/// 执行 extract_claude_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_claude_message(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;

    if value.get("type").and_then(|v| v.as_str()) == Some("assistant")
        && let Some(content) = value.pointer("/message/content").and_then(|v| v.as_array())
    {
        let chunks = content.iter().filter_map(extract_claude_text_block).collect::<Vec<_>>();
        if !chunks.is_empty() {
            return Some(chunks.join("\n"));
        }
    }

    if let Some(content) = value.pointer("/content").and_then(|v| v.as_array()) {
        let chunks = content.iter().filter_map(extract_claude_text_block).collect::<Vec<_>>();
        if !chunks.is_empty() {
            return Some(chunks.join("\n"));
        }
    }

    let candidates = [
        value.pointer("/delta/text").and_then(|v| v.as_str()),
        value.pointer("/content_block/text").and_then(|v| v.as_str()),
        value.pointer("/message/text").and_then(|v| v.as_str()),
        value.pointer("/result").and_then(|v| v.as_str()),
        value.pointer("/message").and_then(|v| v.as_str()),
        value.pointer("/content").and_then(|v| v.as_str()),
        value.pointer("/text").and_then(|v| v.as_str()),
        value.pointer("/delta").and_then(|v| v.as_str()),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|text| !text.is_empty())
        .map(|text| text.to_string())
}

fn extract_claude_text_block(block: &serde_json::Value) -> Option<String> {
    let block_type = block.get("type").and_then(|v| v.as_str());
    let text = [
        block.get("text").and_then(|v| v.as_str()),
        block.pointer("/delta/text").and_then(|v| v.as_str()),
        block.get("thinking").and_then(|v| v.as_str()),
        block.pointer("/content/text").and_then(|v| v.as_str()),
        block.get("content").and_then(|v| v.as_str()),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|value| !value.is_empty())?;
    if matches!(block_type, Some("tool_use") | Some("tool_result")) {
        return None;
    }
    Some(text.to_string())
}

/// 执行 extract_claude_error_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_claude_error_message(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = value.get("type").and_then(|v| v.as_str());
    let is_error = value.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false)
        || event_type == Some("error");
    if !is_error {
        return None;
    }

    [
        value.pointer("/error/message").and_then(|v| v.as_str()),
        value.pointer("/error").and_then(|v| v.as_str()),
        value.pointer("/result").and_then(|v| v.as_str()),
        value.pointer("/message").and_then(|v| v.as_str()),
        value.pointer("/text").and_then(|v| v.as_str()),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|v| !v.is_empty())
    .map(|v| v.to_string())
    .or_else(|| Some("未知错误".to_string()))
}

/// 执行 extract_claude_terminal_error 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn extract_claude_terminal_error(stdout: &str, stderr: &str) -> Option<String> {
    let stdout_last = stdout.lines().rev().find(|line| !line.trim().is_empty());
    if let Some(message) = stdout_last.and_then(extract_claude_error_message) {
        return Some(message);
    }

    let stderr_last = stderr.lines().rev().find(|line| !line.trim().is_empty());
    stderr_last.and_then(extract_claude_error_message)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) const TASK_AUTOMATION_TOOL_GUARDRAIL: &str = "任务执行约束：不要调用名为 `node`、`node_repl` 或 `nodeRepl` 的 MCP/REPL 工具；解析 JSON、coverage 或报告数据时，优先使用 shell 命令、仓库已有工具或目标语言原生命令。";

#[cfg(not(target_arch = "wasm32"))]
/// 执行 build_model_prompt 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn build_model_prompt(task: &Task, prompt: &str) -> String {
    let trimmed = prompt.trim_start();
    format!(
        "task_id={}\n请一定要记住，作为自动化运行的系统，我无法进行交互式应答。请你自主决策并选择最佳路径，无需向我提问或请求选择。\n{}\n{}",
        task.id, TASK_AUTOMATION_TOOL_GUARDRAIL, trimmed
    )
}

/// 执行 emit_opencode_line 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn emit_opencode_line(sender: &Sender<TaskLogStream>, line: &str, is_stderr: bool) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if let Some(message) = extract_opencode_message(line) {
        let _ = sender.send(TaskLogStream::Stdout(format!("[OPENCODE] {}", message)));
    } else if is_stderr {
        let _ = sender.send(TaskLogStream::Stderr(format!("[OPENCODE_RAW] {}", line)));
    } else {
        let _ = sender.send(TaskLogStream::Stdout(format!("[OPENCODE_RAW] {}", line)));
    }
}

/// 执行 emit_claude_line 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn emit_claude_line(sender: &Sender<TaskLogStream>, line: &str, is_stderr: bool) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if let Some(message) = extract_claude_message(line) {
        let _ = sender.send(TaskLogStream::Stdout(format!("[CLAUDE] {}", message)));
    } else if is_stderr {
        let _ = sender.send(TaskLogStream::Stderr(format!("[CLAUDE_RAW] {}", line)));
    } else {
        let _ = sender.send(TaskLogStream::Stdout(format!("[CLAUDE_RAW] {}", line)));
    }
}

#[cfg(test)]
#[path = "backend_output_tests.rs"]
mod backend_output_tests;
