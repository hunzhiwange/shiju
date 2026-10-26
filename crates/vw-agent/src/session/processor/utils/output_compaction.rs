//! 工具输入输出压缩逻辑，负责在日志和 UI 展示前收窄大块或敏感的工具载荷。

use super::file_links::compact_file_link;
use crate::app::agent::tools::{is_todo_read_tool_id, is_todo_write_tool_id};
use serde_json::Value;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolInputSanitizeMode {
    Log,
    Ui,
}

/// 执行 tool_fingerprint 操作，并返回调用方需要的结果。
pub(crate) fn tool_fingerprint(name: &str, input_sanitized: &str, session_message: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut h);
    input_sanitized.hash(&mut h);
    session_message.len().hash(&mut h);
    if let Some(head) = session_message.get(..std::cmp::min(session_message.len(), 128)) {
        head.hash(&mut h);
    }
    h.finish()
}

/// 执行 is_streaming_tool 操作，并返回调用方需要的结果。
pub(crate) fn is_streaming_tool(name: &str) -> bool {
    matches!(name, "bash" | "shell" | "grep" | "read" | "file_read" | "pdf_read" | "glob" | "ls")
}

/// 执行 sanitize_tool_input 操作，并返回调用方需要的结果。
pub(crate) fn sanitize_tool_input(name: &str, input: &str) -> String {
    sanitize_tool_input_with_mode(name, input, ToolInputSanitizeMode::Log)
}

/// 执行 sanitize_tool_input_for_ui 操作，并返回调用方需要的结果。
pub(crate) fn sanitize_tool_input_for_ui(name: &str, input: &str) -> String {
    sanitize_tool_input_with_mode(name, input, ToolInputSanitizeMode::Ui)
}

fn sanitize_tool_input_with_mode(name: &str, input: &str, mode: ToolInputSanitizeMode) -> String {
    const MAX_INPUT_BYTES: usize = 2 * 1024;
    const MAX_STRING_BYTES: usize = 400;

    let raw = input.trim();
    if raw.is_empty() {
        return String::new();
    }
    if !raw.starts_with('{') {
        if mode == ToolInputSanitizeMode::Ui && preserves_full_ui_input(name, None) {
            return raw.to_string();
        }
        return truncate_string(raw, MAX_INPUT_BYTES);
    }

    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return truncate_string(raw, MAX_INPUT_BYTES);
    };
    let v = sanitize_json_value(name, None, &v, MAX_STRING_BYTES, mode);
    let s = serde_json::to_string(&v).unwrap_or_else(|_| raw.to_string());
    if mode == ToolInputSanitizeMode::Ui { s } else { truncate_string(&s, MAX_INPUT_BYTES) }
}

fn preserves_full_ui_input(tool: &str, key: Option<&str>) -> bool {
    if let Some(key) = key {
        return matches!(
            key,
            "oldString"
                | "newString"
                | "old_string"
                | "new_string"
                | "cell"
                | "source"
                | "content"
                | "patch"
                | "diff"
        );
    }

    matches!(tool, "apply_patch")
}

fn sanitize_json_value(
    tool: &str,
    key: Option<&str>,
    v: &Value,
    max_string_bytes: usize,
    mode: ToolInputSanitizeMode,
) -> Value {
    fn omit_string(s: &str) -> Value {
        Value::String(format!("<omitted {} chars>", s.chars().count()))
    }

    let key = key.unwrap_or("");
    let mut omit_keys = matches!(
        key,
        "oldString"
            | "newString"
            | "old_string"
            | "new_string"
            | "cell"
            | "source"
            | "content"
            | "patch"
            | "diff"
            | "body"
            | "prompt"
            | "messages"
            | "answer"
            | "history"
    );
    if (is_todo_write_tool_id(tool) || is_todo_read_tool_id(tool)) && key == "content" {
        omit_keys = false;
    }
    let omit_by_tool = matches!(tool, "write" | "file_write" | "file_edit" | "apply_patch")
        && matches!(
            key,
            "content"
                | "patch"
                | "oldString"
                | "newString"
                | "old_string"
                | "new_string"
                | "cell"
                | "source"
        );

    match v {
        Value::String(s) => {
            if mode == ToolInputSanitizeMode::Log && (omit_keys || omit_by_tool) {
                return omit_string(s);
            }
            if mode == ToolInputSanitizeMode::Ui && preserves_full_ui_input(tool, Some(key)) {
                return Value::String(s.clone());
            }
            if s.as_bytes().len() <= max_string_bytes {
                return Value::String(s.clone());
            }
            let mut cut = max_string_bytes;
            while cut > 0 && !s.is_char_boundary(cut) {
                cut -= 1;
            }
            Value::String(format!("{}…<truncated {} chars>", &s[..cut], s.chars().count()))
        }
        Value::Array(arr) => {
            const MAX_ITEMS: usize = 20;
            let mut out = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                if i >= MAX_ITEMS {
                    out.push(serde_json::json!({ "_remaining_items": arr.len() - MAX_ITEMS }));
                    break;
                }
                out.push(sanitize_json_value(tool, None, item, max_string_bytes, mode));
            }
            Value::Array(out)
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, vv) in map {
                out.insert(
                    k.clone(),
                    sanitize_json_value(tool, Some(k.as_str()), vv, max_string_bytes, mode),
                );
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}

/// 执行 truncate_string 操作，并返回调用方需要的结果。
pub(crate) fn truncate_string(s: &str, limit: usize) -> String {
    if s.as_bytes().len() <= limit {
        return s.to_string();
    }
    let mut cut = limit;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push_str("…<truncated>");
    out
}

fn truncate_string_tail(s: &str, limit: usize) -> String {
    if s.as_bytes().len() <= limit {
        return s.to_string();
    }
    let total = s.len();
    let mut start = total.saturating_sub(limit);
    while start < total && !s.is_char_boundary(start) {
        start = start.saturating_add(1);
    }
    let mut out = "…<truncated>".to_string();
    out.push_str(&s[start..]);
    out
}

fn truncate_string_middle(s: &str, limit: usize) -> String {
    const MARKER: &str = "\n…<middle truncated>…\n";
    if s.len() <= limit {
        return s.to_string();
    }
    let content_budget = limit.saturating_sub(MARKER.len());
    let head_budget = content_budget / 2;
    let tail_budget = content_budget.saturating_sub(head_budget);

    let mut head_end = head_budget.min(s.len());
    while head_end > 0 && !s.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = s.len().saturating_sub(tail_budget);
    while tail_start < s.len() && !s.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    format!("{}{}{}", &s[..head_end], MARKER, &s[tail_start..])
}

fn is_agent_tool_name(name: &str) -> bool {
    matches!(name.trim().to_ascii_lowercase().as_str(), "agenttool" | "agent_tool" | "agent")
}

fn compact_agent_tool_result(output: &str) -> Option<String> {
    const SINGLE_REPORT_BYTES: usize = 16 * 1024;
    const TOTAL_REPORT_BYTES: usize = 24 * 1024;
    const MAX_MULTI_REPORT_BYTES: usize = 12 * 1024;
    const MIN_REPORT_BYTES: usize = 768;

    let mut value = serde_json::from_str::<Value>(output.trim()).ok()?;
    let report_count = usize::from(value.get("output").and_then(Value::as_str).is_some())
        + value
            .get("tasks")
            .and_then(Value::as_array)
            .map(|tasks| {
                tasks
                    .iter()
                    .filter(|task| task.get("output").and_then(Value::as_str).is_some())
                    .count()
            })
            .unwrap_or_default();
    if report_count == 0 {
        return Some(output.to_string());
    }
    let report_budget = if report_count == 1 {
        SINGLE_REPORT_BYTES
    } else {
        (TOTAL_REPORT_BYTES / report_count).clamp(MIN_REPORT_BYTES, MAX_MULTI_REPORT_BYTES)
    };

    if let Some(report) = value.get_mut("output").filter(|report| report.is_string()) {
        let compacted = truncate_string_middle(report.as_str().unwrap_or_default(), report_budget);
        *report = Value::String(compacted);
    }
    if let Some(tasks) = value.get_mut("tasks").and_then(Value::as_array_mut) {
        for task in tasks {
            if let Some(report) = task.get_mut("output").filter(|report| report.is_string()) {
                let compacted =
                    truncate_string_middle(report.as_str().unwrap_or_default(), report_budget);
                *report = Value::String(compacted);
            }
        }
    }
    serde_json::to_string(&value).ok()
}

/// 执行 compact_tool_output 操作，并返回调用方需要的结果。
pub(crate) fn compact_tool_output(name: &str, output: &str) -> String {
    const MAX_BYTES: usize = 3 * 1024;
    const MAX_BYTES_BASH: usize = 50 * 1024;

    if matches!(name, "bash" | "bash_wait" | "bash_cancel")
        && let Ok(mut value) = serde_json::from_str::<serde_json::Value>(output)
        && value.get("process_id").and_then(serde_json::Value::as_str).is_some()
    {
        let limit = if name == "bash" { MAX_BYTES_BASH } else { MAX_BYTES };
        if let Some(text) = value.get("output").and_then(serde_json::Value::as_str) {
            let compact = truncate_string_middle(text, limit);
            if compact != text {
                value["output"] = serde_json::Value::String(compact);
                value["output_truncated"] = serde_json::Value::Bool(true);
            }
        }
        return value.to_string();
    }
    let mut s = output.to_string();
    if s.contains("<file_link>") {
        s = compact_file_link(&s);
    }
    if name == "bash" {
        return truncate_string_tail(&s, MAX_BYTES_BASH);
    }
    if is_agent_tool_name(name) {
        return compact_agent_tool_result(&s).unwrap_or_else(|| truncate_string(&s, MAX_BYTES));
    }
    truncate_string(&s, MAX_BYTES)
}

fn strip_hidden_file_message(s: &str) -> String {
    s.lines()
        .filter(|line| line.trim() != "内容已隐藏，点击文件名打开")
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// 执行 compact_tool_output_for_ui 操作，并返回调用方需要的结果。
pub(crate) fn compact_tool_output_for_ui(name: &str, output: &str) -> String {
    if matches!(name, "read" | "file_read" | "pdf_read") {
        if output.contains("<file>") {
            return truncate_string(output, 16 * 1024);
        }
        if output.contains("<file_link>") {
            return strip_hidden_file_message(output);
        }
        return truncate_string(output, 512);
    }
    output.to_string()
}

/// 执行 rewrite_todowrite_completed_when_no_work 操作，并返回调用方需要的结果。
pub(crate) fn rewrite_todowrite_completed_when_no_work(input: &str) -> String {
    let raw = input.trim();
    if raw.is_empty() || !raw.starts_with('{') {
        return input.to_string();
    }
    let Ok(mut v) = serde_json::from_str::<Value>(raw) else {
        return input.to_string();
    };
    let merge = v.get("merge").and_then(|m| m.as_bool()).unwrap_or(false);
    let Some(arr) = v.get_mut("todos").and_then(|t| t.as_array_mut()) else {
        return input.to_string();
    };
    let mut changed = false;
    for item in arr {
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        let Some(status) = obj.get("status").and_then(|s| s.as_str()) else {
            continue;
        };
        if status == "completed" {
            obj.insert(
                "status".to_string(),
                Value::String(if merge {
                    "in_progress".to_string()
                } else {
                    "pending".to_string()
                }),
            );
            changed = true;
        }
    }
    if !changed {
        return input.to_string();
    }
    serde_json::to_string(&v).unwrap_or_else(|_| input.to_string())
}
#[cfg(test)]
#[path = "output_compaction_tests.rs"]
mod output_compaction_tests;
