//! Git 消息的领域辅助逻辑；桌面负责跨域消息映射。
use crate::git_panel::diff_view::GitDiffContextMenuState;
use desktop_workspace::workspace_state::GitDiffSelectedLine;
use crate::git_panel::diff_view::GitDiffLineRange;
use iced::Font;
use iced_code_editor::theme;
pub fn text_too_large_for_code_editor(s: &str) -> bool {
    const MAX_BYTES: usize = 1_000_000;
    const MAX_LINES: usize = 20_000;
    const MAX_LINE_BYTES: usize = 20_000;

    if s.len() > MAX_BYTES {
        return true;
    }

    for (i, line) in s.lines().enumerate() {
        if i > MAX_LINES || line.len() > MAX_LINE_BYTES || line.contains('\0') {
            return true;
        }
    }
    false
}

pub fn normalize_range(mut range: GitDiffLineRange) -> GitDiffLineRange {
    if range.start > range.end {
        std::mem::swap(&mut range.start, &mut range.end);
    }
    range
}

pub fn configure_git_code_editor(
    theme: iced::Theme,
    font_size: f32,
    line_height: f32,
    language: iced_code_editor::i18n::Language,
    editor: &mut iced_code_editor::CodeEditor,
) {
    editor.set_font(Font::with_name("JetBrains Mono"));
    editor.set_theme(theme::from_iced_theme(&theme));
    editor.set_font_size(font_size, true);
    editor.set_line_height(line_height);
    editor.set_language(language);
    editor.set_line_numbers_enabled(false);
    editor.set_search_replace_enabled(true);
    editor.set_wrap_enabled(false);
}

pub fn normalize_path(value: &str) -> String {
    value.replace('\\', "/").trim_end_matches('/').to_string()
}

pub fn worktree_label(name: &str, branch: &str, directory: &str) -> String {
    let base = if name.trim().is_empty() {
        std::path::Path::new(directory)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("工作树")
            .to_string()
    } else {
        name.to_string()
    };

    if branch.trim().is_empty() { base } else { format!("{base} · {branch}") }
}

pub fn range_contains_line(range: &GitDiffLineRange, file: &str, line: usize, is_old: bool) -> bool {
    range.file == file && range.is_old == is_old && line >= range.start && line <= range.end
}

#[cfg(not(target_arch = "wasm32"))]
pub fn diff_contents_for_file(repo_path: &str, file: &str) -> Option<(String, String)> {
    let repo = git2::Repository::open(&repo_path).ok()?;
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;

    let old_content = if let Ok(entry) = tree.get_path(std::path::Path::new(file)) {
        let obj = entry.to_object(&repo).ok()?;
        let blob = obj.as_blob()?;
        String::from_utf8_lossy(blob.content()).to_string()
    } else {
        String::new()
    };

    let new_content =
        std::fs::read_to_string(std::path::Path::new(&repo_path).join(file)).unwrap_or_default();

    Some((old_content, new_content))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn changed_diff_line_sets(
    repo_path: &str,
    file: &str,
) -> Option<(std::collections::HashSet<usize>, std::collections::HashSet<usize>)> {
    let (old_content, new_content) = diff_contents_for_file(repo_path, file)?;
    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    let mut old_lines = std::collections::HashSet::new();
    let mut new_lines = std::collections::HashSet::new();

    for group in diff.grouped_ops(desktop_git::DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Delete { old_index, old_len, .. } => {
                    for k in 0..old_len {
                        old_lines.insert(old_index + k);
                    }
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    for k in 0..new_len {
                        new_lines.insert(new_index + k);
                    }
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    for k in 0..old_len {
                        old_lines.insert(old_index + k);
                    }
                    for k in 0..new_len {
                        new_lines.insert(new_index + k);
                    }
                }
                similar::DiffOp::Equal { .. } => {}
            }
        }
    }

    Some((old_lines, new_lines))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn new_position_for_old_line(repo_path: &str, file: &str, old_idx: usize) -> Option<usize> {
    let (old_content, new_content) = diff_contents_for_file(repo_path, file)?;
    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    let mut last_new_end = 0usize;
    let mut target_new_pos = None::<usize>;

    for group in diff.grouped_ops(desktop_git::DIFF_CONTEXT) {
        for op in group {
            match op {
                similar::DiffOp::Equal { old_index, new_index, len } => {
                    if old_index <= old_idx && old_idx < old_index + len {
                        target_new_pos = Some(new_index + (old_idx - old_index));
                    }
                    last_new_end = last_new_end.max(new_index + len);
                }
                similar::DiffOp::Delete { old_index, old_len, new_index } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index);
                }
                similar::DiffOp::Insert { new_index, new_len, .. } => {
                    last_new_end = last_new_end.max(new_index + new_len);
                }
                similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    if old_index <= old_idx && old_idx < old_index + old_len {
                        target_new_pos = Some(new_index);
                    }
                    last_new_end = last_new_end.max(new_index + new_len);
                }
            }
        }
    }

    Some(target_new_pos.unwrap_or(last_new_end))
}

#[cfg(test)]
#[path = "shared_tests.rs"]
mod tests;

pub fn diff_context_target_range(
    git_diff_context_menu: &Option<GitDiffContextMenuState>,
    git_diff_selected_range: &Option<GitDiffLineRange>,
    git_diff_selected_lines: &Vec<GitDiffSelectedLine>,
) -> Option<GitDiffLineRange> {
    let range = git_diff_context_menu.as_ref().map(|menu| {
        if let Some(selected_range) = (*git_diff_selected_range).clone().map(normalize_range)
            && range_contains_line(&selected_range, &menu.file, menu.line, menu.is_old)
        {
            selected_range
        } else {
            GitDiffLineRange {
                file: menu.file.clone(),
                start: menu.line,
                end: menu.line,
                is_old: menu.is_old,
            }
        }
    });

    let range = range.or_else(|| (*git_diff_selected_range).clone()).or_else(|| {
        git_diff_selected_lines.first().map(|line| GitDiffLineRange {
            file: line.file.clone(),
            start: line.line,
            end: line.line,
            is_old: line.is_old,
        })
    })?;

    Some(normalize_range(range))
}

pub fn diff_context_target_stage_lines(
    git_diff_context_menu: &Option<GitDiffContextMenuState>,
    git_diff_selected_range: &Option<GitDiffLineRange>,
    git_diff_selected_lines: &Vec<GitDiffSelectedLine>,
) -> Vec<(String, usize, bool)> {
    if let Some(menu) = git_diff_context_menu.as_ref() {
        if let Some(selected_range) = (*git_diff_selected_range).clone().map(normalize_range)
            && range_contains_line(&selected_range, &menu.file, menu.line, menu.is_old)
        {
            return (selected_range.start..=selected_range.end)
                .map(|line| (selected_range.file.clone(), line, selected_range.is_old))
                .collect();
        }

        let mut selected_lines = git_diff_selected_lines
            .iter()
            .filter(|selected| selected.file == menu.file && selected.is_old == menu.is_old)
            .map(|selected| (selected.file.clone(), selected.line, selected.is_old))
            .collect::<Vec<_>>();

        if selected_lines.iter().any(|(_, line, _)| *line == menu.line) {
            selected_lines.sort();
            selected_lines.dedup();
            return selected_lines;
        }

        return vec![(menu.file.clone(), menu.line, menu.is_old)];
    }

    if let Some(selected_range) = (*git_diff_selected_range).clone().map(normalize_range) {
        return (selected_range.start..=selected_range.end)
            .map(|line| (selected_range.file.clone(), line, selected_range.is_old))
            .collect();
    }

    let mut selected_lines = git_diff_selected_lines
        .iter()
        .map(|selected| (selected.file.clone(), selected.line, selected.is_old))
        .collect::<Vec<_>>();
    selected_lines.sort();
    selected_lines.dedup();
    selected_lines
}

pub fn extend_stage_selection_for_lines(
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    staged_lines_selected: &mut Vec<(String, usize)>,
    lines: &[(String, usize, bool)],
) {
    for (file, line, is_old) in lines {
        let target = if *is_old {
            &mut (*staged_old_lines_selected)
        } else {
            &mut (*staged_lines_selected)
        };
        let entry = (file.clone(), *line);
        if !target.iter().any(|(selected_file, selected_line)| {
            *selected_file == entry.0 && *selected_line == entry.1
        }) {
            target.push(entry);
        }
    }

    (*staged_lines_selected).sort();
    (*staged_lines_selected).dedup();
    (*staged_old_lines_selected).sort();
    (*staged_old_lines_selected).dedup();
}

pub fn clear_stage_selection_for_lines(
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    staged_lines_selected: &mut Vec<(String, usize)>,
    lines: &[(String, usize, bool)],
) {
    for (file, line, is_old) in lines {
        if *is_old {
            (*staged_old_lines_selected).retain(|(selected_file, selected_line)| {
                selected_file != file || *selected_line != *line
            });
        } else {
            (*staged_lines_selected).retain(|(selected_file, selected_line)| {
                selected_file != file || *selected_line != *line
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn normalize_stage_line_selections(
    staged_lines_selected: &mut Vec<(String, usize)>,
    staged_old_lines_selected: &mut Vec<(String, usize)>,
) {
    (*staged_lines_selected).sort();
    (*staged_lines_selected).dedup();
    (*staged_old_lines_selected).sort();
    (*staged_old_lines_selected).dedup();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn replace_stage_selection_for_file_lines(
    staged_lines_selected: &mut Vec<(String, usize)>, staged_old_lines_selected: &mut Vec<(String, usize)>,
    file: &str,
    old_changed: &std::collections::HashSet<usize>,
    new_changed: &std::collections::HashSet<usize>,
) {
    (*staged_lines_selected).retain(|(selected_file, _)| selected_file != file);
    (*staged_old_lines_selected).retain(|(selected_file, _)| selected_file != file);

    (*staged_lines_selected)
        .extend(new_changed.iter().copied().map(|line| (file.to_string(), line)));
    (*staged_old_lines_selected)
        .extend(old_changed.iter().copied().map(|line| (file.to_string(), line)));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn invert_stage_selection_for_file_lines(
    staged_lines_selected: &mut Vec<(String, usize)>, staged_old_lines_selected: &mut Vec<(String, usize)>,
    file: &str,
    old_changed: &std::collections::HashSet<usize>,
    new_changed: &std::collections::HashSet<usize>,
) {
    for line in new_changed.iter().copied() {
        if let Some(pos) =
            (*staged_lines_selected).iter().position(|(selected_file, selected_line)| {
                selected_file == file && *selected_line == line
            })
        {
            (*staged_lines_selected).remove(pos);
        } else {
            (*staged_lines_selected).push((file.to_string(), line));
        }
    }

    for line in old_changed.iter().copied() {
        if let Some(pos) =
            (*staged_old_lines_selected).iter().position(|(selected_file, selected_line)| {
                selected_file == file && *selected_line == line
            })
        {
            (*staged_old_lines_selected).remove(pos);
        } else {
            (*staged_old_lines_selected).push((file.to_string(), line));
        }
    }
}

pub fn git_context_path(
    worktree: Option<&str>,
    active_directory: Option<&str>,
    project: Option<&str>,
) -> Option<String> {
    for value in [worktree, active_directory].into_iter().flatten() {
        let value = value.trim();
        if !value.is_empty() { return Some(value.to_owned()); }
    }
    project.map(str::to_owned)
}

pub fn selected_lines_from_range(repo_path: Option<&str>, range: &GitDiffLineRange) -> Vec<GitDiffSelectedLine> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some((old_content, new_content)) = repo_path.and_then(|path| diff_contents_for_file(path, &range.file)) {
        let content = if range.is_old { old_content } else { new_content };
        let lines: Vec<_> = content.lines().collect();
        return (range.start.min(range.end)..=range.start.max(range.end)).filter_map(|line| {
            lines.get(line).map(|text| GitDiffSelectedLine { file: range.file.clone(), line, is_old: range.is_old, text: (*text).to_owned() })
        }).collect();
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (repo_path, range);
    Vec::new()
}

pub use super::commit::{SelectedCommitRequest, build_commit_request, reset_commit_form_state};

pub use super::gateway::{execute_selected_commit_via_gateway, execute_stage_all_via_gateway, load_git_worktree_options};
