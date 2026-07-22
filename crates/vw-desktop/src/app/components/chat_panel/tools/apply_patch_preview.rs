//! 解析 apply_patch 输出并生成变更预览数据。
//! 模块只提取文件级差异信息，不直接触碰文件系统，避免预览扩大写入能力。

use std::collections::HashMap;
use std::path::Path;

use vw_shared::patch::{Hunk, parse_patch};

use super::changes::parse_changes_files;
use super::diff_utils::extract_diff_block;
use super::types::ChangeFile;

/// 执行 collect_apply_patch_changes 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn collect_apply_patch_changes(output: &str, input: &str) -> Vec<ChangeFile> {
    let mut parsed = parse_changes_files(output);
    if let Some(fallback_source) = extract_diff_block(output).filter(|d| !d.trim().is_empty()) {
        parsed =
            merge_apply_patch_changes(parsed, parse_unified_diff_change_files(&fallback_source));
    }

    if parsed.is_empty() && looks_like_unified_diff(input) {
        parsed = merge_apply_patch_changes(parsed, parse_unified_diff_change_files(input));
    }

    if parsed.is_empty() && input.contains("*** Begin Patch") {
        parsed = merge_apply_patch_changes(parsed, parse_apply_patch_change_files(input));
    }

    parsed
}

/// 执行 find_apply_patch_change 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn find_apply_patch_change<'a>(
    changes: &'a [ChangeFile],
    path: &str,
) -> Option<&'a ChangeFile> {
    changes.iter().filter(|change| apply_patch_paths_match(&change.path, path)).max_by_key(
        |change| {
            (
                usize::from(!change.before.is_empty() || !change.after.is_empty()),
                change.before.len() + change.after.len(),
                change.additions + change.deletions,
            )
        },
    )
}

/// 执行 apply_patch_paths_match 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn apply_patch_paths_match(candidate: &str, target: &str) -> bool {
    let candidate = normalize_apply_patch_change_path(candidate);
    let target = normalize_apply_patch_change_path(target);
    candidate == target
        || candidate.ends_with(&format!("/{target}"))
        || target.ends_with(&format!("/{candidate}"))
        || Path::new(&candidate).file_name() == Path::new(&target).file_name()
}

/// 执行 parse_apply_patch_change_files 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn parse_apply_patch_change_files(patch_text: &str) -> Vec<ChangeFile> {
    let Ok(parsed) = parse_patch(patch_text) else {
        return Vec::new();
    };

    parsed
        .hunks
        .into_iter()
        .map(|hunk| match hunk {
            Hunk::Add { path, contents } => ChangeFile {
                additions: contents.lines().count(),
                deletions: 0,
                before: String::new(),
                after: contents,
                path,
            },
            Hunk::Delete { path } => ChangeFile {
                additions: 0,
                deletions: 0,
                before: String::new(),
                after: String::new(),
                path,
            },
            Hunk::Update { path, move_path, chunks } => {
                let mut before = String::new();
                let mut after = String::new();
                let mut additions = 0usize;
                let mut deletions = 0usize;

                for (idx, chunk) in chunks.into_iter().enumerate() {
                    if idx > 0 {
                        append_preview_gap(&mut before);
                        append_preview_gap(&mut after);
                    }

                    if let Some(context) = chunk.change_context.filter(|s| !s.is_empty()) {
                        append_preview_line(&mut before, &context);
                        append_preview_line(&mut after, &context);
                    }

                    for line in chunk.old_lines {
                        append_preview_line(&mut before, &line);
                        deletions = deletions.saturating_add(1);
                    }

                    for line in chunk.new_lines {
                        append_preview_line(&mut after, &line);
                        additions = additions.saturating_add(1);
                    }
                }

                ChangeFile { additions, deletions, before, after, path: move_path.unwrap_or(path) }
            }
        })
        .collect()
}

/// 执行 parse_unified_diff_change_files 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn parse_unified_diff_change_files(diff: &str) -> Vec<ChangeFile> {
    let mut out = Vec::new();
    let mut current: Option<ChangeFile> = None;
    let mut old_path: Option<String> = None;
    let mut _new_path: Option<String> = None;

    for raw in diff.lines() {
        if raw.starts_with("diff --git ") {
            if let Some(change) = current.take() {
                out.push(change);
            }
            old_path = None;
            _new_path = None;
            continue;
        }

        if let Some(path) = raw.strip_prefix("--- ") {
            old_path = normalize_unified_diff_path(path);
            continue;
        }

        if let Some(path) = raw.strip_prefix("+++ ") {
            _new_path = normalize_unified_diff_path(path);
            let resolved = _new_path.clone().or_else(|| old_path.clone());
            if let Some(path) = resolved {
                if let Some(change) = current.take() {
                    out.push(change);
                }
                current = Some(ChangeFile {
                    path,
                    additions: 0,
                    deletions: 0,
                    before: String::new(),
                    after: String::new(),
                });
            }
            continue;
        }

        if raw.starts_with("@@") {
            append_preview_gap_to_current(&mut current);
            continue;
        }

        if raw.starts_with("index ")
            || raw.starts_with("new file mode ")
            || raw.starts_with("deleted file mode ")
            || raw.starts_with("similarity index ")
            || raw.starts_with("rename from ")
            || raw.starts_with("rename to ")
            || raw.starts_with("Binary files ")
            || raw.starts_with("\\ No newline at end of file")
        {
            continue;
        }

        let Some(change) = current.as_mut() else {
            continue;
        };

        if let Some(stripped) = raw.strip_prefix('+') {
            if !raw.starts_with("+++") {
                append_preview_line(&mut change.after, stripped);
                change.additions = change.additions.saturating_add(1);
            }
            continue;
        }

        if let Some(stripped) = raw.strip_prefix('-') {
            if !raw.starts_with("---") {
                append_preview_line(&mut change.before, stripped);
                change.deletions = change.deletions.saturating_add(1);
            }
            continue;
        }

        if let Some(stripped) = raw.strip_prefix(' ') {
            append_preview_line(&mut change.before, stripped);
            append_preview_line(&mut change.after, stripped);
        }
    }

    if let Some(change) = current {
        out.push(change);
    }

    out
}

fn normalize_apply_patch_change_path(path: &str) -> String {
    let path = path.trim().replace('\\', "/");
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path.as_str())
        .trim_start_matches("./")
        .to_string()
}

fn normalize_unified_diff_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path == "/dev/null" {
        return None;
    }

    if let Some(stripped) = path.strip_prefix("a/") {
        return Some(stripped.to_string());
    }
    if let Some(stripped) = path.strip_prefix("b/") {
        return Some(stripped.to_string());
    }

    Some(path.to_string())
}

fn append_preview_gap_to_current(current: &mut Option<ChangeFile>) {
    let Some(change) = current.as_mut() else {
        return;
    };
    append_preview_gap(&mut change.before);
    append_preview_gap(&mut change.after);
}

fn append_preview_gap(buf: &mut String) {
    if !buf.is_empty() && !buf.ends_with("\n\n") {
        buf.push('\n');
    }
}

fn append_preview_line(buf: &mut String, line: &str) {
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf.push_str(line);
}

fn looks_like_unified_diff(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.starts_with("--- ") && t.contains("\n+++ ") && t.contains("\n@@")
}

fn merge_apply_patch_changes(
    primary: Vec<ChangeFile>,
    fallback: Vec<ChangeFile>,
) -> Vec<ChangeFile> {
    let mut by_path = HashMap::<String, ChangeFile>::new();

    for change in primary.into_iter().chain(fallback) {
        let key = normalize_apply_patch_change_path(&change.path);
        match by_path.get_mut(&key) {
            Some(current) => {
                if apply_patch_change_score(&change) > apply_patch_change_score(current) {
                    *current = change;
                } else {
                    fill_missing_change_fields(current, &change);
                }
            }
            None => {
                by_path.insert(key, change);
            }
        }
    }

    by_path.into_values().collect()
}

fn apply_patch_change_score(change: &ChangeFile) -> (usize, usize, usize) {
    (
        usize::from(!change.before.is_empty()) + usize::from(!change.after.is_empty()),
        change.before.len() + change.after.len(),
        change.additions + change.deletions,
    )
}

fn fill_missing_change_fields(target: &mut ChangeFile, source: &ChangeFile) {
    if target.before.is_empty() && !source.before.is_empty() {
        target.before = source.before.clone();
    }
    if target.after.is_empty() && !source.after.is_empty() {
        target.after = source.after.clone();
    }
    if target.additions == 0 && source.additions > 0 {
        target.additions = source.additions;
    }
    if target.deletions == 0 && source.deletions > 0 {
        target.deletions = source.deletions;
    }
}

#[cfg(test)]
#[path = "tests/apply_patch_preview.rs"]
mod tests;

#[cfg(test)]
#[path = "apply_patch_preview_tests.rs"]
mod coverage_tests;
