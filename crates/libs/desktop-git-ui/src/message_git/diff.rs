use crate::DiffTheme;
use crate::git_panel::diff_view::{GitDiffLineRange, GitDiffCommentDraft, GitDiffContextMenuState};
use desktop_workspace::workspace_state::GitDiffSelectedLine;
use iced::widget::text_editor;
use desktop_workspace::workspace_state::GitDiffFileMenuState;

pub fn toggle_diff_line_selection(
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    file: String,
    line: usize,
    is_old: bool,
    text: String,
) {
    (*git_diff_selected_range) = None;
    if let Some(pos) = (*git_diff_selected_lines).iter().position(|selected| {
        selected.file == file && selected.line == line && selected.is_old == is_old
    }) {
        (*git_diff_selected_lines).remove(pos);
    } else {
        (*git_diff_selected_lines).push(GitDiffSelectedLine { file, line, is_old, text });
    }
}

pub fn diff_drag_select_start(
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_dragging: &mut bool,
    git_diff_drag_start_text: &mut Option<String>,
    git_diff_drag_range: &mut Option<GitDiffLineRange>,
    file: String,
    line: usize,
    is_old: bool,
    text: String,
) {
    (*git_diff_comment_draft) = None;
    (*git_diff_context_menu) = None;
    (*git_diff_selected_range) = None;
    (*git_diff_dragging) = true;
    (*git_diff_drag_start_text) = Some(text);
    (*git_diff_drag_range) =
        Some(GitDiffLineRange { file, start: line, end: line, is_old });
}

pub fn diff_drag_select_hover(
    git_diff_dragging: &mut bool,
    git_diff_drag_range: &mut Option<GitDiffLineRange>,
    file: String,
    line: usize,
    is_old: bool,
) {
    if (*git_diff_dragging)
        && let Some(range) = (*git_diff_drag_range).as_mut()
        && range.file == file
        && range.is_old == is_old
    {
        range.end = line;
    }
}

pub fn close_diff_context_menu(git_diff_context_menu: &mut Option<GitDiffContextMenuState>) {
    (*git_diff_context_menu) = None;
}

pub fn close_diff_file_menu(git_diff_file_menu: &mut Option<GitDiffFileMenuState>) {
    (*git_diff_file_menu) = None;
}

pub fn revert_diff_file(
    git_diff_file_menu: &mut Option<GitDiffFileMenuState>,
    file_to_discard: &mut Option<String>,
    file: String,
) {
    (*git_diff_file_menu) = None;
    (*file_to_discard) = Some(file);
}

pub fn diff_hover_enter(
    git_diff_hovered_line: &mut Option<(String, usize, bool)>,
    file: String,
    line: usize,
    is_old: bool,
) {
    (*git_diff_hovered_line) = Some((file, line, is_old));
}

pub fn diff_hover_exit(
    git_diff_hovered_line: &mut Option<(String, usize, bool)>,
    file: String,
    line: usize,
    is_old: bool,
) {
    if let Some((hover_file, hover_line, hover_old)) = (*git_diff_hovered_line).as_ref()
        && hover_file == &file
        && *hover_line == line
        && *hover_old == is_old
    {
        (*git_diff_hovered_line) = None;
    }
}

pub fn diff_comment_editor_action(
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    action: text_editor::Action,
) {
    if let Some(draft) = (*git_diff_comment_draft).as_mut() {
        draft.editor.perform(action);
    }
}

pub fn diff_comment_cancel(
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
) {
    (*git_diff_comment_draft) = None;
    (*git_diff_selected_range) = None;
}

pub fn confirm_discard_file(
    git_diff_file_menu: &mut Option<GitDiffFileMenuState>,
    file_to_discard: &mut Option<String>,
    file: String,
) {
    (*git_diff_file_menu) = None;
    (*file_to_discard) = Some(file);
}

pub fn cancel_discard_file(
    git_diff_file_menu: &mut Option<GitDiffFileMenuState>,
    file_to_discard: &mut Option<String>,
) {
    (*git_diff_file_menu) = None;
    (*file_to_discard) = None;
}

pub fn toggle_expand_hunk(expanded_hunks: &mut Vec<(String, usize)>, file: String, idx: usize) {
    if let Some(pos) =
        (*expanded_hunks).iter().position(|(expanded_file, expanded_idx)| {
            expanded_file == &file && *expanded_idx == idx
        })
    {
        (*expanded_hunks).remove(pos);
    } else {
        (*expanded_hunks).push((file, idx));
    }
}

pub fn diff_theme_selected(diff_theme: &mut DiffTheme, theme: DiffTheme) {
    (*diff_theme) = theme;
}

pub enum Effect { None, CopyCode(String), AppendChat(String), Refresh }
use super::shared::{diff_context_target_range, diff_context_target_stage_lines, extend_stage_selection_for_lines, clear_stage_selection_for_lines, selected_lines_from_range};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use desktop_git::{git_revert_line_restore, git_revert_line_delete};
#[cfg(not(target_arch = "wasm32"))]
use super::shared::{changed_diff_line_sets, new_position_for_old_line};

pub fn diff_drag_select_end(
    git_diff_dragging: &mut bool,
    git_diff_drag_start_text: &mut Option<String>,
    git_diff_drag_range: &mut Option<GitDiffLineRange>,
    git_diff_last_click: &mut Option<(String, usize, bool, web_time::Instant)>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    repo_path: Option<String>,
) -> Effect {
    (*git_diff_dragging) = false;
    let start_text = (*git_diff_drag_start_text).take();
    let Some(range) = (*git_diff_drag_range).take() else {
        return Effect::None;
    };
    let range = normalize_range(range);

    let now = web_time::Instant::now();
    if range.start == range.end {
        let is_double_click =
            (*git_diff_last_click).as_ref().is_some_and(|(file, line, is_old, when)| {
                file == &range.file
                    && *line == range.start
                    && *is_old == range.is_old
                    && now.duration_since(*when) <= Duration::from_millis(350)
            });

        if is_double_click {
            (*git_diff_last_click) = None;
            (*git_diff_selected_range) = Some(range.clone());
            (*git_diff_comment_draft) =
                Some(GitDiffCommentDraft { range, editor: text_editor::Content::new() });
        } else {
            (*git_diff_last_click) =
                Some((range.file.clone(), range.start, range.is_old, now));
            (*git_diff_selected_range) = None;
            let text = start_text.unwrap_or_default();
            if let Some(pos) = (*git_diff_selected_lines).iter().position(|selected| {
                selected.file == range.file
                    && selected.line == range.start
                    && selected.is_old == range.is_old
            }) {
                (*git_diff_selected_lines).remove(pos);
            } else {
                (*git_diff_selected_lines).push(GitDiffSelectedLine {
                    file: range.file.clone(),
                    line: range.start,
                    is_old: range.is_old,
                    text,
                });
            }
        }
        return Effect::None;
    }

    (*git_diff_last_click) = None;
    (*git_diff_selected_range) = Some(range.clone());
    (*git_diff_selected_lines) = selected_lines_from_range(repo_path.as_deref(), &range);
    (*git_diff_comment_draft) = None;
    Effect::None
}

pub fn open_diff_context_menu(
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    file: String,
    line: usize,
    is_old: bool,
    text: String,
    x: f32,
    y: f32,
) -> Effect {

    let in_selected_range = (*git_diff_selected_range).as_ref().is_some_and(|range| {
        range.file == file
            && range.is_old == is_old
            && line >= range.start.min(range.end)
            && line <= range.start.max(range.end)
    });
    let in_selected_lines = (*git_diff_selected_lines).iter().any(|selected| {
        selected.file == file && selected.line == line && selected.is_old == is_old
    });

    if !in_selected_range
        && !in_selected_lines
        && (*git_diff_selected_lines).is_empty()
        && (*git_diff_selected_range).is_none()
    {
        (*git_diff_selected_lines).clear();
        (*git_diff_selected_lines).push(GitDiffSelectedLine {
            file: file.clone(),
            line,
            is_old,
            text,
        });
        (*git_diff_selected_range) =
            Some(GitDiffLineRange { file: file.clone(), start: line, end: line, is_old });
    }

    (*git_diff_context_menu) = Some(GitDiffContextMenuState { file, line, is_old, x, y });
    Effect::None
}

pub fn open_diff_file_menu(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_file_menu: &mut Option<GitDiffFileMenuState>,
    file: String,
) -> Effect {
    (*git_diff_context_menu) = None;
    (*git_diff_file_menu) = Some(GitDiffFileMenuState { file });
    Effect::None
}

pub fn open_diff_comment_draft(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
) -> Effect {
    let Some(range) = diff_context_target_range(&(*git_diff_context_menu), &(*git_diff_selected_range), &(*git_diff_selected_lines)) else {
        return Effect::None;
    };
    (*git_diff_context_menu) = None;
    (*git_diff_selected_range) = Some(range.clone());
    (*git_diff_comment_draft) =
        Some(GitDiffCommentDraft { range, editor: text_editor::Content::new() });
    Effect::None
}

pub fn select_diff_context_stage_lines(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    staged_lines_selected: &mut Vec<(String, usize)>,
) -> Effect {
    let lines = diff_context_target_stage_lines(&(*git_diff_context_menu), &(*git_diff_selected_range), &(*git_diff_selected_lines));
    if lines.is_empty() {
        return Effect::None;
    }
    extend_stage_selection_for_lines(&mut (*staged_old_lines_selected), &mut (*staged_lines_selected), &lines);
    (*git_diff_context_menu) = None;
    Effect::None
}

pub fn clear_diff_context_stage_lines(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    staged_lines_selected: &mut Vec<(String, usize)>,
) -> Effect {
    let lines = diff_context_target_stage_lines(&(*git_diff_context_menu), &(*git_diff_selected_range), &(*git_diff_selected_lines));
    if lines.is_empty() {
        return Effect::None;
    }
    clear_stage_selection_for_lines(&mut (*staged_old_lines_selected), &mut (*staged_lines_selected), &lines);
    (*git_diff_context_menu) = None;
    Effect::None
}

pub fn diff_comment_submit(
    git_diff_comment_draft: &mut Option<GitDiffCommentDraft>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
) -> Effect {
    let Some(draft) = (*git_diff_comment_draft).take() else {
        return Effect::None;
    };
    let file = draft.range.file;
    let start = draft.range.start + 1;
    let end = draft.range.end + 1;
    let comment = draft.editor.text().trim().to_string();
    let line_ref =
        if start == end { start.to_string() } else { format!("{}-{}", start, end) };
    let out = if comment.is_empty() {
        format!("@{}:{} ", file, line_ref)
    } else {
        format!("@{}:{} #{} ", file, line_ref, comment)
    };
    (*git_diff_selected_range) = None;
    Effect::AppendChat(out)
}

pub fn copy_diff_selection(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
) -> Effect {
    (*git_diff_context_menu) = None;
    let mut selected = (*git_diff_selected_lines).clone();
    selected.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.is_old.cmp(&right.is_old))
            .then(left.line.cmp(&right.line))
    });
    let content =
        selected.into_iter().map(|entry| entry.text).collect::<Vec<_>>().join("\n");
    if content.trim().is_empty() {
        Effect::None
    } else {
        Effect::CopyCode(content)
    }
}

pub fn discard_diff_selection(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
    git_diff_selected_range: &mut Option<GitDiffLineRange>,
    repo_path: Option<String>,
) -> Effect {
    #[cfg(target_arch = "wasm32")]
    let _ = (&repo_path, &git_diff_selected_lines, &git_diff_selected_range);
    (*git_diff_context_menu) = None;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(repo_path) = repo_path.clone() else {
            return Effect::None;
        };

        let mut selected = (*git_diff_selected_lines).clone();
        selected.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.is_old.cmp(&right.is_old))
                .then(left.line.cmp(&right.line))
        });
        if selected.is_empty() {
            return Effect::None;
        }

        let mut refreshed = false;
        let mut changed_line_cache = std::collections::HashMap::new();
        for entry in selected.into_iter().rev() {
            let (old_changed, new_changed) =
                changed_line_cache.entry(entry.file.clone()).or_insert_with(|| {
                    changed_diff_line_sets(&repo_path, &entry.file).unwrap_or_default()
                });

            let result = if entry.is_old && old_changed.contains(&entry.line) {
                let insert_idx = new_position_for_old_line(&repo_path, &entry.file, entry.line)
                    .unwrap_or(usize::MAX);
                git_revert_line_restore(&repo_path, &entry.file, insert_idx, entry.line)
            } else if !entry.is_old && new_changed.contains(&entry.line) {
                git_revert_line_delete(&repo_path, &entry.file, entry.line)
            } else {
                continue;
            };
            if result.is_ok() {
                refreshed = true;
            }
        }

        (*git_diff_selected_lines).clear();
        (*git_diff_selected_range) = None;
        if refreshed {
            return Effect::Refresh;
        }
    }
    Effect::None
}

pub fn insert_diff_selection_to_chat(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
) -> Effect {
    (*git_diff_context_menu) = None;
    let mut selected = (*git_diff_selected_lines).clone();
    selected.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.is_old.cmp(&right.is_old))
            .then(left.line.cmp(&right.line))
    });
    let text = selected.into_iter().map(|entry| entry.text).collect::<Vec<_>>().join("\n");
    if text.trim().is_empty() {
        Effect::None
    } else {
        Effect::AppendChat(text)
    }
}

pub fn insert_diff_selection_comment(
    git_diff_context_menu: &mut Option<GitDiffContextMenuState>,
    git_diff_selected_lines: &mut Vec<GitDiffSelectedLine>,
) -> Effect {
    (*git_diff_context_menu) = None;
    let mut selected = (*git_diff_selected_lines).clone();
    selected.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.is_old.cmp(&right.is_old))
            .then(left.line.cmp(&right.line))
    });
    if selected.is_empty() {
        return Effect::None;
    }

    let mut out = String::from("Git Diff 评论\n");
    for file in selected
        .iter()
        .map(|entry| entry.file.as_str())
        .collect::<std::collections::BTreeSet<_>>()
    {
        out.push_str(&format!("文件: {}\n", file));
        let old_lines = selected
            .iter()
            .filter(|entry| entry.file == file && entry.is_old)
            .map(|entry| entry.line)
            .collect::<Vec<_>>();
        let new_lines = selected
            .iter()
            .filter(|entry| entry.file == file && !entry.is_old)
            .map(|entry| entry.line)
            .collect::<Vec<_>>();
        if !old_lines.is_empty() {
            out.push_str(&format!("旧行: {:?}\n", old_lines));
        }
        if !new_lines.is_empty() {
            out.push_str(&format!("新行: {:?}\n", new_lines));
        }
        out.push_str("```text\n");
        for entry in selected.iter().filter(|entry| entry.file == file) {
            out.push_str(&entry.text);
            out.push('\n');
        }
        out.push_str("```\n");
    }
    out.push_str("评论: \n");

    Effect::AppendChat(out)
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;





pub use super::diff_actions::{ExpandDirection, DiscardAction, is_diff_file_expanded, ensure_diff_file_expanded, clear_expanded_files, set_single_expanded_file, toggle_expand_file, expand_context, scroll_changed, toggle_fullscreen, discard};
