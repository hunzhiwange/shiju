//! 工具块与特殊文本摘要工具。
//!
//! 该模块集中处理消息中的工具摘要、探索工具分组汇总和与文件链接相关的
//! 文本规整逻辑，供渲染缓存和消息主体渲染复用。

use std::collections::HashMap;
use std::path::Path;

use crate::app::models::ParsedChatBlock;
use crate::app::state::{AdvancedToolSurfaceState, explicit_advanced_tool_surface_spec};

use super::super::tools::{
    EXPLORE_GROUP_TOOL_IDX, ExploreToolKind, canonical_tool_name, explore_item_dedupe_key,
    explore_tool_kind, is_explore_tool, tool_error_text, tool_inline_summary, tool_input,
    tool_name_from_raw, tool_output_text, tool_permission_error_text, tool_permission_summary,
    tool_status, tool_status_from_raw, tool_structured_diff_text, tool_summary_text,
};
use super::super::utils::{
    normalize_display_text, normalize_file_reference_to_path, single_line_preview,
    strip_internal_tool_trace, truncate_chars,
};
use super::parse::{RenderBlock, borrowed_blocks, owned_blocks_from_raw};

fn bash_command_title(input: &str) -> String {
    let cmd_line = if input.trim_start().starts_with('{') {
        serde_json::from_str::<serde_json::Value>(input)
            .ok()
            .and_then(|value| {
                value.get("command").and_then(|item| item.as_str()).map(ToString::to_string)
            })
            .unwrap_or_default()
    } else {
        input.trim().to_string()
    };

    let title = single_line_preview(&cmd_line, 140);
    if title.is_empty() { "运行命令".to_string() } else { title }
}

fn parse_read_summary_input(input: &str) -> Option<(String, usize, usize)> {
    if input.trim_start().starts_with('{') {
        let value = serde_json::from_str::<serde_json::Value>(input.trim()).ok()?;
        let file_path = value
            .get("filePath")
            .or_else(|| value.get("file_path"))
            .or_else(|| value.get("path"))
            .and_then(|item| item.as_str())
            .and_then(normalize_file_reference_to_path)?;
        let offset = value.get("offset").and_then(|item| item.as_u64()).unwrap_or(0) as usize;
        let limit = value.get("limit").and_then(|item| item.as_u64()).unwrap_or(0) as usize;
        Some((file_path, offset, limit))
    } else {
        normalize_file_reference_to_path(input).map(|file_path| (file_path, 0usize, 0usize))
    }
}

fn read_summary_range_text(offset: usize, limit: usize) -> Option<String> {
    if offset > 0 && limit > 0 {
        let start_line = offset + 1;
        let end_line = offset + limit;
        Some(format!(
            "offset={} limit={} (line {}-{})",
            offset,
            end_line - start_line + 1,
            start_line,
            end_line
        ))
    } else if offset > 0 {
        Some(format!("offset={} (from line {})", offset, offset + 1))
    } else if limit > 0 {
        Some(format!("limit={} (line 1-{})", limit, limit))
    } else {
        None
    }
}

fn read_summary_parts(input: &str) -> Option<(String, Option<String>)> {
    let (file_path, offset, limit) = parse_read_summary_input(input)?;
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|item| item.to_str())
        .unwrap_or(file_path.as_str())
        .to_string();
    Some((file_name, read_summary_range_text(offset, limit)))
}

fn json_string_field(value: &str, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|json| json.get(key).and_then(|item| item.as_str()).map(ToString::to_string))
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn parse_relaxed_json_value(input: &str) -> Option<serde_json::Value> {
    let trimmed = input.trim();
    serde_json::from_str::<serde_json::Value>(trimmed).ok().or_else(|| {
        let mut repaired = String::with_capacity(trimmed.len());
        let mut in_string = false;
        let mut escaped = false;

        for ch in trimmed.chars() {
            if escaped {
                repaired.push(ch);
                escaped = false;
                continue;
            }

            if in_string && ch == '\\' {
                repaired.push(ch);
                escaped = true;
                continue;
            }

            if ch == '"' {
                repaired.push(ch);
                in_string = !in_string;
                continue;
            }

            if in_string {
                match ch {
                    '\n' => repaired.push_str("\\n"),
                    '\r' => repaired.push_str("\\r"),
                    '\t' => repaired.push_str("\\t"),
                    '\u{08}' => repaired.push_str("\\b"),
                    '\u{0C}' => repaired.push_str("\\f"),
                    control if control.is_control() => {
                        repaired.push_str(&format!("\\u{:04x}", control as u32));
                    }
                    _ => repaired.push(ch),
                }
            } else {
                repaired.push(ch);
            }
        }

        serde_json::from_str::<serde_json::Value>(&repaired).ok()
    })
}

fn json_string_or_array_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match value.get(*key) {
        Some(serde_json::Value::String(text)) => Some(text.clone()),
        Some(serde_json::Value::Array(items)) => {
            let lines = items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if lines.is_empty() { None } else { Some(lines.join("\n")) }
        }
        _ => None,
    })
}

fn file_tool_input_preview(tool_name: &str, input: &str) -> Option<String> {
    if !input.trim_start().starts_with('{') {
        return None;
    }

    let value = parse_relaxed_json_value(input)?;
    match canonical_tool_name(tool_name) {
        "write" | "file_write" => {
            value.get("content").and_then(|item| item.as_str()).map(ToString::to_string)
        }
        "file_edit" => json_string_or_array_field(&value, &["new_string", "newString"]),
        "notebook_edit" => json_string_or_array_field(&value, &["new_code", "newCode"]),
        _ => None,
    }
}

fn tool_result_data_object(
    value: &serde_json::Value,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    value
        .get("result")
        .and_then(|result| result.get("data"))
        .or_else(|| value.get("data"))
        .and_then(|data| data.as_object())
}

fn tool_search_preview_texts(value: &serde_json::Value) -> Vec<String> {
    let Some(data) = tool_result_data_object(value) else {
        return Vec::new();
    };
    let Some(items) = data.get("items").and_then(|item| item.as_array()) else {
        return Vec::new();
    };

    let mut lines = Vec::new();
    for item in items.iter().take(3) {
        let Some(object) = item.as_object() else {
            continue;
        };
        let label = object
            .get("display_name")
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .or_else(|| object.get("id").and_then(|item| item.as_str()).map(str::trim))
            .unwrap_or("unknown");
        let reason = object
            .get("reason")
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("matched");
        lines.push(format!("{label}: {reason}"));
    }
    lines
}

fn browser_preview_texts(tool_name: &str, value: &serde_json::Value) -> Vec<String> {
    let Some(data) = tool_result_data_object(value) else {
        return Vec::new();
    };

    if matches!(tool_name, "browser_open" | "open_browser_page") {
        let browser =
            data.get("browser").and_then(|item| item.as_str()).map(str::trim).unwrap_or("");
        let url = data.get("url").and_then(|item| item.as_str()).map(str::trim).unwrap_or("");
        return match (browser.is_empty(), url.is_empty()) {
            (true, true) => Vec::new(),
            (false, true) => vec![format!("已在 {browser} 中打开页面")],
            (true, false) => vec![url.to_string()],
            (false, false) => vec![format!("{browser} · {url}")],
        };
    }

    let Some(result) = data.get("result") else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if let Some(title) = result.get("title").and_then(|item| item.as_str()).map(str::trim)
        && !title.is_empty()
    {
        lines.push(title.to_string());
    }
    if let Some(url) = result.get("url").and_then(|item| item.as_str()).map(str::trim)
        && !url.is_empty()
    {
        lines.push(url.to_string());
    }
    if let Some(text) = result.get("text").and_then(|item| item.as_str()).map(str::trim)
        && !text.is_empty()
    {
        lines.push(truncate_chars(text, 160).to_string());
    }
    if let Some(path) = result.get("path").and_then(|item| item.as_str()).map(str::trim)
        && !path.is_empty()
    {
        lines.push(path.to_string());
    }
    lines
}

pub(super) fn count_code_blocks(text: &str) -> usize {
    text.match_indices("```").count() / 2
}

fn strip_markdown_list_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return rest.trim_start();
    }

    let digit_count = trimmed.bytes().take_while(|byte| byte.is_ascii_digit()).count();
    if digit_count > 0 {
        let rest = &trimmed[digit_count..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return rest.trim_start();
        }
    }

    trimmed
}

fn is_markdown_file_link_line(line: &str) -> bool {
    let trimmed = strip_markdown_list_prefix(line);
    if trimmed.is_empty() || trimmed.ends_with('：') || trimmed.ends_with(':') {
        return true;
    }

    if !trimmed.starts_with('[') {
        return false;
    }

    let Some((_, rest)) = trimmed.split_once("](") else {
        return false;
    };
    let Some(target) = rest.strip_suffix(')') else {
        return false;
    };
    target.starts_with("file:///")
}

fn is_file_list_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.ends_with('：') || trimmed.ends_with(':') {
        return true;
    }
    if is_markdown_file_link_line(trimmed) {
        return true;
    }
    if trimmed == "<file_link>" || trimmed == "</file_link>" {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix("open: ") {
        return rest.trim_start().starts_with("file:///");
    }
    if trimmed.starts_with("path: ") {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix("- ") {
        let rest = rest.trim();
        return !rest.is_empty();
    }
    false
}

fn is_file_list_text(text: &str) -> bool {
    let mut has_file_link = false;
    let mut has_content = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        has_content = true;
        if trimmed.contains("](file:///")
            || trimmed.starts_with("open: file:///")
            || trimmed == "<file_link>"
            || trimmed.strip_prefix("- ").is_some()
        {
            has_file_link = true;
        }
        if !is_file_list_line(trimmed) {
            return false;
        }
    }

    has_content && has_file_link
}

pub(super) fn should_hide_explore_link_box(text: &str) -> bool {
    is_file_list_text(text)
}

pub(super) fn should_hide_post_explore_tool_block(raw: &str) -> bool {
    let Some((_, rest)) = raw.split_once('\n') else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(rest.trim()) else {
        return false;
    };
    let output = tool_output_text(&value).unwrap_or_default();
    let output = output.trim();
    !output.is_empty() && is_file_list_text(output)
}

pub(super) fn normalized_visible_text(content: &str) -> Option<String> {
    let cleaned = strip_internal_tool_trace(content);
    let normalized = normalize_display_text(cleaned.trim()).into_owned();
    if normalized.is_empty() { None } else { Some(normalized) }
}

pub(crate) fn trailing_tool_tail_text_source_block_idx(
    blocks: &[ParsedChatBlock],
) -> Option<usize> {
    let mut trailing_end = blocks.len();
    while trailing_end > 0 {
        match &blocks[trailing_end - 1] {
            ParsedChatBlock::Text { content } if normalized_visible_text(content).is_none() => {
                trailing_end -= 1;
            }
            _ => break,
        }
    }

    let mut first_trailing_tool = trailing_end;
    let mut has_trailing_explore = false;
    while first_trailing_tool > 0 {
        match &blocks[first_trailing_tool - 1] {
            ParsedChatBlock::Tool { raw } => {
                if let Some(name) = tool_name_from_raw(raw)
                    && is_explore_tool(&name)
                {
                    has_trailing_explore = true;
                }
                first_trailing_tool -= 1;
            }
            ParsedChatBlock::Text { content } if normalized_visible_text(content).is_none() => {
                first_trailing_tool -= 1;
            }
            _ => break,
        }
    }

    if !has_trailing_explore || first_trailing_tool == trailing_end {
        return None;
    }

    let mut candidate = first_trailing_tool;
    while candidate > 0 {
        candidate -= 1;
        match &blocks[candidate] {
            ParsedChatBlock::Text { content } if normalized_visible_text(content).is_some() => {
                return Some(candidate);
            }
            ParsedChatBlock::Text { .. } => continue,
            _ => break,
        }
    }

    None
}

fn collect_tool_texts(tool_name: &str, value: &serde_json::Value) -> Vec<String> {
    let tool_name = canonical_tool_name(tool_name);
    let mut tool_texts = Vec::new();
    let status = tool_status(value);
    let is_error = matches!(status, "error" | "denied");
    let output_text = tool_output_text(value).unwrap_or_default();
    let output = output_text.trim();
    let err_owned = tool_error_text(value).unwrap_or_default();
    let err_text = err_owned.trim();
    let permission_error_text = tool_permission_error_text(tool_name, value).unwrap_or_default();
    let permission_error_text = permission_error_text.trim();

    match tool_name {
        "bash" => {
            let input = tool_input(value);
            tool_texts.push(bash_command_title(input));
        }
        "read" | "file_read" | "pdf_read" => {
            if let Some((file_name, range_text)) = read_summary_parts(tool_input(value)) {
                tool_texts.push(file_name);
                if let Some(range_text) = range_text {
                    tool_texts.push(range_text);
                }
            }
        }
        "apply_patch" => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                }
            } else if let Some(summary) = tool_permission_summary(tool_name, value) {
                tool_texts.push(summary.to_string());
            } else if let Some(summary) = tool_summary_text(value) {
                tool_texts.push(summary);
            }
            if let Some(diff) = tool_structured_diff_text(value).or_else(|| {
                value
                    .get("output")
                    .and_then(|item| item.as_str())
                    .and_then(super::super::tools::extract_diff_block)
            }) {
                let diff = diff.trim();
                if !diff.is_empty() {
                    tool_texts.push(diff.to_string());
                }
            }
        }
        "write" | "file_write" | "file_edit" | "notebook_edit" => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                }
            } else if let Some(summary) = tool_permission_summary(tool_name, value) {
                tool_texts.push(summary.to_string());
            } else if let Some(summary) = tool_summary_text(value)
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            }
            let input = tool_input(value);
            if let Some(preview) = super::super::tools::file_preview(tool_name, input, output)
                .or_else(|| file_tool_input_preview(tool_name, input))
            {
                if !preview.trim().is_empty() {
                    tool_texts.push(preview);
                }
            }
        }
        "glob" | "glob_search" | "grep" | "lsp" | "content_search" | "codesearch" => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                } else if !err_text.is_empty() {
                    tool_texts.push(err_text.to_string());
                }
            } else if !output.is_empty() {
                tool_texts.push(output.to_string());
            } else if let Some(summary) = tool_summary_text(value)
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            }
        }
        "web_fetch" | "fetch_webpage" | "http_request" | "web_search" => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                } else if !err_text.is_empty() {
                    tool_texts.push(err_text.to_string());
                }
            } else if !output.is_empty() {
                tool_texts.push(output.to_string());
            } else if let Some(summary) = tool_summary_text(value)
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            }
        }
        "AgentTool" | "Agent" | "browser" | "browser_open" => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                } else if !err_text.is_empty() {
                    tool_texts.push(err_text.to_string());
                }
            } else if let Some(summary) = tool_summary_text(value)
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            } else if let Some(summary) =
                super::super::tools::tool_inline_summary(tool_name, tool_input(value))
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            }

            if !output.is_empty() {
                if tool_name == "AgentTool" || tool_name == "Agent" {
                    if let Some(message) = json_string_field(output, "message") {
                        tool_texts.push(message);
                    } else {
                        tool_texts.push(output.to_string());
                    }
                } else {
                    let previews = browser_preview_texts(tool_name, value);
                    if previews.is_empty() {
                        tool_texts.push(output.to_string());
                    } else {
                        tool_texts.extend(previews);
                    }
                }
            }
        }
        _ if explicit_advanced_tool_surface_spec(tool_name).is_some() => {
            if let Some(spec) = explicit_advanced_tool_surface_spec(tool_name) {
                tool_texts.push(format!("状态: {}", spec.state.label()));
                if is_error {
                    if !permission_error_text.is_empty() {
                        tool_texts.push(permission_error_text.to_string());
                    } else if !err_text.is_empty() {
                        tool_texts.push(err_text.to_string());
                    }
                } else if !output.is_empty() {
                    if tool_name == "tool_search" {
                        let previews = tool_search_preview_texts(value);
                        if previews.is_empty() {
                            tool_texts.push(output.to_string());
                        } else {
                            tool_texts.extend(previews);
                        }
                    } else {
                        tool_texts.push(output.to_string());
                    }
                } else {
                    let state_text = match spec.state {
                        AdvancedToolSurfaceState::Available => {
                            format!("{} 已接入当前会话工具面。", spec.label)
                        }
                        AdvancedToolSurfaceState::Planned => {
                            format!("{} 当前已明确标记为 planned。", spec.label)
                        }
                    };
                    tool_texts.push(state_text);
                }
            }
        }
        "todowrite" => {
            if is_error && !err_text.is_empty() {
                tool_texts.push(err_text.to_string());
            }
            if let Some(input) = value.get("input").and_then(|item| item.as_str())
                && let Ok(input_json) = serde_json::from_str::<serde_json::Value>(input.trim())
                && let Some(todos) = input_json.get("todos").and_then(|item| item.as_array())
            {
                let lines = todos
                    .iter()
                    .map(|todo| {
                        let status =
                            todo.get("status").and_then(|item| item.as_str()).unwrap_or("").trim();
                        let content = todo
                            .get("content")
                            .and_then(|item| item.as_str())
                            .unwrap_or("（无内容）")
                            .trim();
                        let symbol = match status {
                            "completed" => "✓",
                            "in_progress" => "·",
                            _ => "○",
                        };
                        format!("{} {}", symbol, content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !lines.is_empty() {
                    tool_texts.push(lines);
                }
            }
        }
        "todoread" => {
            if let Ok(todos) = serde_json::from_str::<Vec<vw_shared::todo::Todo>>(output) {
                if todos.is_empty() {
                    tool_texts.push("暂无任务".to_string());
                } else {
                    for todo in todos {
                        tool_texts.push(todo.content.trim().to_string());
                    }
                }
            } else if !output.is_empty() {
                tool_texts.push(output.to_string());
            }
        }
        "question" => {
            if let Some(summary) = tool_summary_text(value)
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            } else if let Some(summary) = tool_inline_summary(tool_name, tool_input(value))
                && !summary.trim().is_empty()
            {
                tool_texts.push(summary);
            }

            if is_error && !err_text.is_empty() {
                tool_texts.push(err_text.to_string());
            }
        }
        _ => {
            if is_error {
                if !permission_error_text.is_empty() {
                    tool_texts.push(permission_error_text.to_string());
                } else if !err_text.is_empty() {
                    tool_texts.push(err_text.to_string());
                }
            } else {
                if let Some(summary) = tool_summary_text(value)
                    && !summary.trim().is_empty()
                {
                    tool_texts.push(summary);
                }
                if !output.is_empty() {
                    tool_texts.push(output.to_string());
                }
            }
        }
    }

    tool_texts
}

pub(super) fn collect_tool_card_texts(raw: &str) -> Option<Vec<String>> {
    let (first, rest) = raw.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    Some(collect_tool_texts(tool_name, &value))
}

pub(crate) fn summarize_explore_items<'a>(
    items: impl Iterator<Item = &'a str>,
    group_idx: usize,
    force_running: bool,
) -> Option<(usize, String)> {
    let mut latest_items: HashMap<String, (Option<ExploreToolKind>, Option<String>)> =
        HashMap::new();
    let mut fallback_idx = 0usize;
    let mut has_item = false;

    for raw in items {
        has_item = true;
        if let Some(name) = tool_name_from_raw(raw) {
            let kind = explore_tool_kind(&name);
            let status = tool_status_from_raw(raw);
            let dedupe_key = explore_item_dedupe_key(raw).unwrap_or_else(|| {
                let key = format!("seq:{fallback_idx}");
                fallback_idx = fallback_idx.saturating_add(1);
                key
            });
            latest_items.insert(dedupe_key, (kind, status));
        }
    }

    if !has_item {
        return None;
    }

    let mut read_count = 0usize;
    let mut search_count = 0usize;
    let mut list_count = 0usize;
    let mut other_count = 0usize;
    let mut has_running = false;

    for (kind, status) in latest_items.into_values() {
        match kind {
            Some(ExploreToolKind::Read) => read_count += 1,
            Some(ExploreToolKind::Search) => search_count += 1,
            Some(ExploreToolKind::List) => list_count += 1,
            None => other_count += 1,
        }
        if status.as_deref() == Some("running") {
            has_running = true;
        }
    }

    let mut parts = Vec::new();
    if read_count > 0 {
        parts.push(format!("{} 次读取", read_count));
    }
    if search_count > 0 {
        parts.push(format!("{} 次搜索", search_count));
    }
    if list_count > 0 {
        parts.push(format!("{} 次列出", list_count));
    }
    if other_count > 0 {
        parts.push(format!("{} 次其他", other_count));
    }
    let summary_text = if parts.is_empty() { "暂无".to_string() } else { parts.join("，") };
    let group_base_idx = EXPLORE_GROUP_TOOL_IDX.saturating_sub(group_idx.saturating_mul(2));
    let group_running_idx = group_base_idx.saturating_sub(1);
    let group_tool_idx =
        if has_running || force_running { group_running_idx } else { group_base_idx };
    Some((group_tool_idx, summary_text))
}

#[allow(dead_code)]
pub(crate) fn special_text_blocks(raw: &str) -> Vec<String> {
    let blocks = owned_blocks_from_raw(raw);
    borrowed_blocks(&blocks)
        .filter_map(|block| match block {
            RenderBlock::Text { content } => normalized_visible_text(content),
            _ => None,
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn tool_card_text_blocks(raw: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let blocks = owned_blocks_from_raw(raw);
    for block in borrowed_blocks(&blocks) {
        if let RenderBlock::Tool { raw } = block
            && let Some(tool_texts) = collect_tool_card_texts(raw)
        {
            out.push(tool_texts);
        }
    }
    out
}

fn flush_explore_summary<'a>(
    out: &mut Vec<(usize, String)>,
    items: &mut Vec<&'a str>,
    group_idx: usize,
    force_running: bool,
) {
    if let Some(summary) = summarize_explore_items(items.iter().copied(), group_idx, force_running)
    {
        out.push(summary);
    }
    items.clear();
}

#[allow(dead_code)]
pub(crate) fn explore_summary_text_blocks(
    raw: &str,
    show_reasoning_summary: bool,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut group_idx = 0usize;
    let mut explore_items: Vec<&str> = Vec::new();
    let mut explore_group_force_running = false;
    let blocks = owned_blocks_from_raw(raw);

    for block in borrowed_blocks(&blocks) {
        match block {
            RenderBlock::Tool { raw } => {
                if let Some(name) = tool_name_from_raw(raw)
                    && is_explore_tool(&name)
                {
                    explore_items.push(raw);
                    continue;
                }
                flush_explore_summary(
                    &mut out,
                    &mut explore_items,
                    group_idx,
                    explore_group_force_running,
                );
                explore_group_force_running = false;
                group_idx = group_idx.saturating_add(1);
            }
            RenderBlock::Think { open, .. } => {
                if open || show_reasoning_summary {
                    if open {
                        explore_group_force_running = true;
                    }
                    flush_explore_summary(
                        &mut out,
                        &mut explore_items,
                        group_idx,
                        explore_group_force_running,
                    );
                    explore_group_force_running = open;
                    group_idx = group_idx.saturating_add(1);
                }
            }
            RenderBlock::Text { content } => {
                if normalized_visible_text(content).is_none() {
                    continue;
                }

                flush_explore_summary(
                    &mut out,
                    &mut explore_items,
                    group_idx,
                    explore_group_force_running,
                );
                explore_group_force_running = false;
                group_idx = group_idx.saturating_add(1);
            }
        }
    }

    flush_explore_summary(&mut out, &mut explore_items, group_idx, explore_group_force_running);
    out
}
