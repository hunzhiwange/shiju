use super::types::FileDescriptor;
use crate::app::agent::tools::context::{current_read_state_for_path, current_tool_use_context};
use crate::app::agent::tools::{FileReadStateEntry, FileSnapshot};
use anyhow::Context;
use serde_json::{Value, json};
use std::ops::Range;
use std::path::{Path, PathBuf};
use vw_api_tools::StructuredPatchHunkDto;

/// patch 汇总信息。
#[derive(Debug, Clone)]
pub(crate) struct PatchSummary {
    pub hunks: Vec<StructuredPatchHunkDto>,
    pub additions: usize,
    pub deletions: usize,
}

/// 字符串替换规划结果。
#[derive(Debug, Clone)]
pub(crate) struct ReplacementPlan {
    pub updated_content: String,
    pub replacements: usize,
    pub quote_normalized_match: bool,
}

pub(crate) fn normalize_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn workspace_root(workspace_dir: &Path) -> PathBuf {
    workspace_dir.canonicalize().unwrap_or_else(|_| workspace_dir.to_path_buf())
}

pub(crate) fn display_path(workspace_dir: &Path, full: &Path) -> String {
    if let Ok(rel) = full.strip_prefix(workspace_dir) {
        return normalize_slashes(rel);
    }
    let root = workspace_root(workspace_dir);
    if let Ok(rel) = full.strip_prefix(&root) {
        return normalize_slashes(rel);
    }
    normalize_slashes(full)
}

pub(crate) fn build_file_descriptor(
    workspace_dir: &Path,
    full: &Path,
    size_bytes: u64,
) -> FileDescriptor {
    FileDescriptor {
        path: display_path(workspace_dir, full),
        absolute_path: normalize_slashes(full),
        open: format!("file:///{}", full.to_string_lossy()),
        size_bytes,
    }
}

pub(crate) fn read_state_metadata_for_path(
    workspace_dir: &Path,
    full: &Path,
    action_label: &str,
) -> Option<Value> {
    if current_tool_use_context().is_none() {
        return None;
    }

    let path = display_path(workspace_dir, full);
    Some(match current_read_state_for_path(full) {
        Some(entry) => {
            let status = if entry.partial_view { "partial" } else { "full" };
            let message = if entry.partial_view {
                format!(
                    "File was last read via a partial view in the current tool context before {action_label}."
                )
            } else {
                format!("File was read in the current tool context before {action_label}.")
            };
            json!({
                "status": status,
                "path": path,
                "message": message,
                "bytesRead": entry.bytes_read,
                "partialView": entry.partial_view,
                "offset": entry.offset,
                "limit": entry.limit,
                "snapshot": entry.snapshot.as_ref().map(|snapshot| json!({
                    "sizeBytes": snapshot.size_bytes,
                    "contentDigest": snapshot.content_digest,
                })),
            })
        }
        None => json!({
            "status": "unread",
            "path": path,
            "message": format!(
                "No file_read state was recorded for this path in the current tool context before {action_label}."
            ),
        }),
    })
}

pub(crate) fn require_read_state_for_existing_file(
    full: &Path,
    display_path: &str,
    action_label: &str,
) -> anyhow::Result<FileReadStateEntry> {
    current_read_state_for_path(full).ok_or_else(|| {
        anyhow::anyhow!(
            "Refusing to {action_label} existing file without a prior file_read in the current tool context: {display_path}"
        )
    })
}

pub(crate) fn ensure_read_state_is_fresh(
    entry: &FileReadStateEntry,
    current_text: &str,
    display_path: &str,
    action_label: &str,
) -> anyhow::Result<()> {
    let snapshot = entry.snapshot.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Refusing to {action_label} {display_path}: the prior file_read snapshot is missing; read the file again first"
        )
    })?;

    if snapshot.matches_text(current_text) {
        return Ok(());
    }

    anyhow::bail!(
        "Refusing to {action_label} {display_path}: file changed since the last file_read in the current tool context"
    )
}

pub(crate) fn snapshot_from_text(text: &str) -> FileSnapshot {
    FileSnapshot::from_text(text)
}

pub(crate) fn build_patch_summary(path: &str, old: &str, new: &str) -> PatchSummary {
    let diff = similar::TextDiff::from_lines(old, new);
    let unified = diff.unified_diff().header(path, path).to_string();

    let mut additions = 0usize;
    let mut deletions = 0usize;
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Insert => additions += 1,
            similar::ChangeTag::Delete => deletions += 1,
            similar::ChangeTag::Equal => {}
        }
    }

    PatchSummary { hunks: parse_hunks(path, &unified), additions, deletions }
}

pub(crate) fn build_replacement_plan(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> anyhow::Result<ReplacementPlan> {
    if old_string.is_empty() {
        anyhow::bail!("old_string must not be empty")
    }

    let exact_ranges = literal_ranges(content, old_string);
    if !exact_ranges.is_empty() {
        return apply_replacements(
            content,
            old_string,
            new_string,
            exact_ranges,
            replace_all,
            false,
        );
    }

    let normalized_ranges = quote_normalized_ranges(content, old_string);
    if normalized_ranges.is_empty() {
        anyhow::bail!("old_string not found in file")
    }

    apply_replacements(content, old_string, new_string, normalized_ranges, replace_all, true)
}

fn apply_replacements(
    content: &str,
    old_string: &str,
    new_string: &str,
    ranges: Vec<Range<usize>>,
    replace_all: bool,
    quote_normalized_match: bool,
) -> anyhow::Result<ReplacementPlan> {
    if !replace_all && ranges.len() != 1 {
        let detail = if quote_normalized_match {
            format!(
                "old_string matched {} locations after quote normalization; set replace_all=true or provide a more specific old_string",
                ranges.len()
            )
        } else {
            format!(
                "old_string matched {} locations; set replace_all=true or provide a more specific old_string",
                ranges.len()
            )
        };
        anyhow::bail!(detail)
    }

    let selected_ranges = if replace_all { ranges } else { vec![ranges[0].clone()] };
    let mut updated = String::with_capacity(content.len().saturating_add(new_string.len()));
    let mut cursor = 0usize;

    for range in &selected_ranges {
        let actual = &content[range.clone()];
        let replacement = if quote_normalized_match {
            preserve_quote_style(old_string, new_string, actual)
        } else {
            new_string.to_string()
        };
        updated.push_str(&content[cursor..range.start]);
        updated.push_str(&replacement);
        cursor = range.end;
    }
    updated.push_str(&content[cursor..]);

    Ok(ReplacementPlan {
        updated_content: updated,
        replacements: selected_ranges.len(),
        quote_normalized_match,
    })
}

fn literal_ranges(content: &str, needle: &str) -> Vec<Range<usize>> {
    content.match_indices(needle).map(|(start, matched)| start..start + matched.len()).collect()
}

fn quote_normalized_ranges(content: &str, needle: &str) -> Vec<Range<usize>> {
    let needle_chars: Vec<char> = needle.chars().map(normalize_quote_char).collect();
    if needle_chars.is_empty() {
        return Vec::new();
    }

    let content_chars: Vec<(usize, char)> = content.char_indices().collect();
    let mut ranges = Vec::new();
    let mut index = 0usize;

    while index + needle_chars.len() <= content_chars.len() {
        let mut matched = true;
        for (offset, expected) in needle_chars.iter().enumerate() {
            if normalize_quote_char(content_chars[index + offset].1) != *expected {
                matched = false;
                break;
            }
        }

        if matched {
            let start = content_chars[index].0;
            let end = if index + needle_chars.len() < content_chars.len() {
                content_chars[index + needle_chars.len()].0
            } else {
                content.len()
            };
            ranges.push(start..end);
            index += needle_chars.len().max(1);
        } else {
            index += 1;
        }
    }

    ranges
}

fn normalize_quote_char(ch: char) -> char {
    if is_quote_char(ch) { '"' } else { ch }
}

fn is_quote_char(ch: char) -> bool {
    matches!(ch, '\'' | '"' | '‘' | '’' | '“' | '”')
}

fn preserve_quote_style(old_string: &str, new_string: &str, actual_match: &str) -> String {
    let old_quotes: Vec<char> = old_string.chars().filter(|ch| is_quote_char(*ch)).collect();
    let actual_quotes: Vec<char> = actual_match.chars().filter(|ch| is_quote_char(*ch)).collect();
    let new_quotes_count = new_string.chars().filter(|ch| is_quote_char(*ch)).count();

    if old_quotes.is_empty()
        || old_quotes.len() != actual_quotes.len()
        || old_quotes.len() != new_quotes_count
    {
        return new_string.to_string();
    }

    let mut actual_quotes = actual_quotes.into_iter();
    let mut rewritten = String::with_capacity(new_string.len());
    for ch in new_string.chars() {
        if is_quote_char(ch) {
            rewritten.push(actual_quotes.next().unwrap_or(ch));
        } else {
            rewritten.push(ch);
        }
    }
    rewritten
}

fn parse_hunks(path: &str, unified: &str) -> Vec<StructuredPatchHunkDto> {
    let mut hunks = Vec::new();
    let mut current: Option<StructuredPatchHunkDto> = None;

    for line in unified.lines() {
        if line.starts_with("@@") {
            if let Some(hunk) = current.take() {
                hunks.push(hunk);
            }
            let (old_start, old_lines, new_start, new_lines) = parse_hunk_header(line);
            current = Some(StructuredPatchHunkDto {
                header: line.to_string(),
                path: Some(path.to_string()),
                old_start,
                old_lines,
                new_start,
                new_lines,
                lines: Vec::new(),
            });
            continue;
        }

        if line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }

        if let Some(hunk) = current.as_mut() {
            hunk.lines.push(line.to_string());
        }
    }

    if let Some(hunk) = current {
        hunks.push(hunk);
    }

    hunks
}

fn parse_hunk_header(header: &str) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
    let mut parts = header.split_whitespace();
    let _ = parts.next();
    let old_part = parts.next().unwrap_or_default();
    let new_part = parts.next().unwrap_or_default();
    let (old_start, old_lines) = parse_hunk_range(old_part, '-');
    let (new_start, new_lines) = parse_hunk_range(new_part, '+');
    (old_start, old_lines, new_start, new_lines)
}

fn parse_hunk_range(part: &str, prefix: char) -> (Option<u32>, Option<u32>) {
    let raw = part.strip_prefix(prefix).unwrap_or_default();
    if raw.is_empty() {
        return (None, None);
    }

    let (start, lines) = raw.split_once(',').unwrap_or((raw, "1"));
    let start = start.parse::<u32>().ok();
    let lines = lines.parse::<u32>().ok();
    (start, lines)
}

pub(crate) fn read_text_file(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file as UTF-8 text: {}", path.display()))
}
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;
