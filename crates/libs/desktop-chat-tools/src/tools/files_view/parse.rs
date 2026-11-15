//! 文件视图的数据解析与列表裁剪。

use std::collections::HashMap;
use std::path::Path;

use crate::path::{normalize_file_url_to_path, resolve_path};

use super::super::changes::parse_changes_files;
use super::super::tool_parse::{tool_change_files, tool_input_path, tool_output_path};
use super::super::types::ChangeFile;
use super::FileListState;

pub fn is_git_diff_tool(tool_name: &str, input: &str) -> bool {
    if tool_name == "git_diff" {
        return true;
    }

    if tool_name != "git_operations" {
        return false;
    }

    serde_json::from_str::<serde_json::Value>(input.trim())
        .ok()
        .is_some_and(|args| args.get("operation").and_then(|v| v.as_str()) == Some("diff"))
}

pub fn should_skip_files_view(tool_name: &str, input: &str) -> bool {
    tool_name == "apply_patch" || is_git_diff_tool(tool_name, input)
}

pub fn is_edit_like_tool(tool_name: &str) -> bool {
    matches!(tool_name, "write" | "file_write" | "file_edit" | "notebook_edit" | "apply_patch")
}

pub fn is_search_tool(tool_name: &str) -> bool {
    matches!(tool_name, "glob" | "glob_search" | "grep" | "content_search" | "lsp" | "codesearch")
}

pub fn parse_output_files(
    project_root: Option<&str>,
    tool_name: &str,
    input: &str,
    output: &str,
    value: &serde_json::Value,
) -> (HashMap<String, ChangeFile>, Vec<(String, String)>) {
    let structured_changes = tool_change_files(value);
    let changes_by_path = if structured_changes.is_empty() {
        parse_changes_files(output)
            .into_iter()
            .map(|change| (change.path.clone(), change))
            .collect::<HashMap<String, ChangeFile>>()
    } else {
        structured_changes
            .into_iter()
            .map(|change| (change.path.clone(), change))
            .collect::<HashMap<String, ChangeFile>>()
    };

    if !changes_by_path.is_empty() {
        let mut items = changes_by_path
            .keys()
            .map(|path| {
                let resolved = resolve_path(project_root, path).unwrap_or_else(|| path.clone());
                (path.clone(), resolved)
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.0.cmp(&right.0));
        return (changes_by_path, items);
    }

    let mut items: Vec<(String, String)> = Vec::new();
    let mut file_link_open: Option<&str> = None;
    let mut file_link_path: Option<&str> = None;
    let mut in_file_link = false;

    for line in output.lines() {
        let line = line.trim();

        if line == "<file_link>" {
            in_file_link = true;
            continue;
        }

        if line == "</file_link>" {
            in_file_link = false;
            continue;
        }

        if in_file_link {
            if let Some(value) = line.strip_prefix("open: ") {
                file_link_open = Some(value.trim());
            } else if let Some(value) = line.strip_prefix("path: ") {
                file_link_path = Some(value.trim());
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("path: ") {
            let path = path.trim();
            if let Some(abs) = resolve_path(project_root, path) {
                items.push((path.to_string(), abs));
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("- ") {
            let display = path.trim().to_string();
            if let Some(abs) = resolve_path(project_root, path) {
                items.push((display, abs));
            }
        }
    }

    if items.is_empty()
        && let Some(open) = file_link_open
    {
        let path = normalize_file_url_to_path(open).to_string();
        let display = file_link_path.map(str::to_string).unwrap_or_else(|| path.clone());
        if Path::new(&path).is_absolute() {
            items.push((display, path));
        }
    }

    if items.is_empty()
        && let Some(path) = tool_output_path(value).and_then(|path| resolve_path(project_root, &path))
    {
        items.push((path.clone(), path));
    }

    if items.is_empty()
        && matches!(tool_name, "read" | "file_read")
        && let Some(path) = tool_input_path(input).and_then(|path| resolve_path(project_root, &path))
    {
        items.push((path.clone(), path));
    }

    if items.is_empty()
        && is_edit_like_tool(tool_name)
        && input.trim_start().starts_with('{')
        && let Some(path) = tool_input_path(input).and_then(|path| resolve_path(project_root, &path))
    {
        items.push((path.clone(), path));
    }

    items.sort_by(|left, right| left.0.cmp(&right.0));
    items.dedup_by(|left, right| left.1 == right.1);

    (changes_by_path, items)
}

pub fn parse_read_range(tool_name: &str, input: &str) -> Option<String> {
    if !matches!(tool_name, "read" | "file_read") || !input.trim_start().starts_with('{') {
        return None;
    }

    serde_json::from_str::<serde_json::Value>(input.trim()).ok().and_then(|value| {
        let mut parts = Vec::new();
        if let Some(offset) = value.get("offset").and_then(|item| item.as_u64()) {
            parts.push(format!("offset={}", offset.max(1)));
        }
        if let Some(limit) = value.get("limit").and_then(|item| item.as_u64()) {
            parts.push(format!("limit={limit}"));
        }
        if parts.is_empty() { None } else { Some(parts.join(", ")) }
    })
}

pub fn build_file_list_state(
    items: Vec<(String, String)>,
    is_search: bool,
    filter_query: &str,
    max_items: usize,
) -> FileListState {
    let filter_query = filter_query.trim().to_lowercase();
    let filtered_items = if filter_query.is_empty() {
        items.clone()
    } else {
        items
            .iter()
            .filter(|(display, _)| display.to_lowercase().contains(&filter_query))
            .cloned()
            .collect::<Vec<_>>()
    };

    let total_items = items.len();
    let display_items = if is_search { filtered_items.clone() } else { items };

    let mut truncated_middle = false;
    let mut middle_omitted = 0usize;
    let mut tail_omitted = 0usize;
    let mut items_for_display = display_items;

    if items_for_display.len() > max_items {
        if is_search {
            tail_omitted = items_for_display.len().saturating_sub(max_items);
            items_for_display.truncate(max_items);
        } else {
            let head = max_items / 2;
            let tail = max_items - head;
            let mut head_items: Vec<(String, String)> = items_for_display.drain(..head).collect();
            let tail_items: Vec<(String, String)> =
                items_for_display.drain(items_for_display.len().saturating_sub(tail)..).collect();
            middle_omitted = items_for_display.len().saturating_sub(head + tail);
            head_items.extend(tail_items);
            items_for_display = head_items;
            truncated_middle = true;
        }
    }

    let display_count = items_for_display.len();
    let is_empty_filtered = items_for_display.is_empty();

    FileListState {
        items_for_display,
        total_items,
        display_count,
        truncated_middle,
        middle_omitted,
        tail_omitted,
        filter_query,
        is_empty_filtered,
        max_items,
        is_search,
    }
}
