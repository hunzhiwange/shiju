//! Git 提交表单、过滤与暂存选择的局部状态转换。
use crate::git_panel::types::ConventionalCommitType;
use iced::widget::text_editor;

pub fn commit_message_changed(
    git_commit_message: &mut String,
    message: String
) {
    (*git_commit_message) = message;
}

pub fn commit_type_selected(
    git_commit_type: &mut Option<ConventionalCommitType>,
    commit_type: ConventionalCommitType
) {
    (*git_commit_type) = Some(commit_type);
}

pub fn commit_scope_changed(
    git_commit_scope: &mut String,
    scope: String
) {
    (*git_commit_scope) = scope;
}

pub fn commit_description_changed(
    git_commit_description: &mut String,
    description: String
) {
    (*git_commit_description) = description;
}

pub fn commit_description_editor_action(
    git_commit_description_editor: &mut text_editor::Content,
    git_commit_description: &mut String,
    action: text_editor::Action
) {
    (*git_commit_description_editor).perform(action);
    (*git_commit_description) = (*git_commit_description_editor).text().to_string();
}

pub fn commit_help_open(
    show_git_commit_help_modal: &mut bool
) {
    (*show_git_commit_help_modal) = true;
}

pub fn commit_help_close(
    show_git_commit_help_modal: &mut bool
) {
    (*show_git_commit_help_modal) = false;
}

pub fn filter_help_open(
    show_git_filter_help_modal: &mut bool
) {
    (*show_git_filter_help_modal) = true;
}

pub fn filter_help_close(
    show_git_filter_help_modal: &mut bool
) {
    (*show_git_filter_help_modal) = false;
}

pub fn toggle_filter_options(
    show_git_filter_options: &mut bool,
    enabled: bool
) {
    (*show_git_filter_options) = enabled;
}

pub fn filter_query_changed(
    git_filter_query: &mut String,
    query: String
) {
    (*git_filter_query) = query;
}

pub fn filter_toggle_included(
    git_filter_included: &mut bool,
    enabled: bool
) {
    (*git_filter_included) = enabled;
}

pub fn filter_toggle_excluded(
    git_filter_excluded: &mut bool,
    enabled: bool
) {
    (*git_filter_excluded) = enabled;
}

pub fn filter_toggle_new(
    git_filter_new: &mut bool,
    enabled: bool
) {
    (*git_filter_new) = enabled;
}

pub fn filter_toggle_modified(
    git_filter_modified: &mut bool,
    enabled: bool
) {
    (*git_filter_modified) = enabled;
}

pub fn filter_toggle_deleted(
    git_filter_deleted: &mut bool,
    enabled: bool
) {
    (*git_filter_deleted) = enabled;
}

pub fn clear_filters(
    git_filter_query: &mut String,
    git_filter_included: &mut bool,
    git_filter_excluded: &mut bool,
    git_filter_new: &mut bool,
    git_filter_modified: &mut bool,
    git_filter_deleted: &mut bool
) {
    (*git_filter_query).clear();
    (*git_filter_included) = false;
    (*git_filter_excluded) = false;
    (*git_filter_new) = false;
    (*git_filter_modified) = false;
    (*git_filter_deleted) = false;
}

pub fn toggle_stage_file(
    staged_files_selected: &mut Vec<String>,
    file: String,
    checked: bool
) {
    if checked {
        if !(*staged_files_selected).contains(&file) {
            (*staged_files_selected).push(file);
        }
    } else if let Some(pos) =
        (*staged_files_selected).iter().position(|selected| selected == &file)
    {
        (*staged_files_selected).remove(pos);
    }
}

pub fn toggle_stage_hunk(
    staged_hunks_selected: &mut Vec<(String, usize)>,
    file: String,
    idx: usize,
    checked: bool
) {
    if checked {
        if !(*staged_hunks_selected).iter().any(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == idx
        }) {
            (*staged_hunks_selected).push((file, idx));
        }
    } else if let Some(pos) =
        (*staged_hunks_selected).iter().position(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == idx
        })
    {
        (*staged_hunks_selected).remove(pos);
    }
}

pub fn toggle_stage_line(
    staged_lines_selected: &mut Vec<(String, usize)>,
    file: String,
    new_idx: usize,
    checked: bool
) {
    if checked {
        if !(*staged_lines_selected).iter().any(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == new_idx
        }) {
            (*staged_lines_selected).push((file, new_idx));
        }
    } else if let Some(pos) =
        (*staged_lines_selected).iter().position(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == new_idx
        })
    {
        (*staged_lines_selected).remove(pos);
    }
}

pub fn toggle_stage_old_line(
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    file: String,
    old_idx: usize,
    checked: bool
) {
    if checked {
        if !(*staged_old_lines_selected).iter().any(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == old_idx
        }) {
            (*staged_old_lines_selected).push((file, old_idx));
        }
    } else if let Some(pos) =
        (*staged_old_lines_selected).iter().position(|(selected_file, selected_idx)| {
            selected_file == &file && *selected_idx == old_idx
        })
    {
        (*staged_old_lines_selected).remove(pos);
    }
}

pub fn clear_all_file_lines(
    staged_lines_selected: &mut Vec<(String, usize)>,
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    file: String
) {
    (*staged_lines_selected).retain(|(selected_file, _)| selected_file != &file);
    (*staged_old_lines_selected).retain(|(selected_file, _)| selected_file != &file);
}

pub fn hover_file_header_enter(
    git_hovered_file_header: &mut Option<String>,
    file: String
) {
    (*git_hovered_file_header) = Some(file);
}

pub fn hover_file_header_exit(
    git_hovered_file_header: &mut Option<String>,
    file: String
) {
    if (*git_hovered_file_header).as_deref() == Some(file.as_str()) {
        (*git_hovered_file_header) = None;
    }
}

pub fn hover_git_panel_header_enter(
    git_panel_header_hovered: &mut bool
) {
    (*git_panel_header_hovered) = true;
}

pub fn hover_git_panel_header_exit(
    git_panel_header_hovered: &mut bool
) {
    (*git_panel_header_hovered) = false;
}

#[cfg(test)]
#[path = "stage_commit_tests.rs"]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub fn select_all_file_lines(
    lines: &mut Vec<(String, usize)>, old_lines: &mut Vec<(String, usize)>,
    path: &str, file: &str,
) {
    let Some((old_changed, new_changed)) = super::shared::changed_diff_line_sets(path, file) else {
        return;
    };
    super::shared::replace_stage_selection_for_file_lines(lines, old_lines, file, &old_changed, &new_changed);
    super::shared::normalize_stage_line_selections(lines, old_lines);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn select_visible_file_lines(
    lines: &mut Vec<(String, usize)>, old_lines: &mut Vec<(String, usize)>,
    path: Option<&str>, files: Vec<String>, invert: bool,
) {
    for file in files {
        let Some((old_changed, new_changed)) = path.and_then(|path| super::shared::changed_diff_line_sets(path, &file)) else {
            continue;
        };
        if invert {
            super::shared::invert_stage_selection_for_file_lines(lines, old_lines, &file, &old_changed, &new_changed);
        } else {
            super::shared::replace_stage_selection_for_file_lines(lines, old_lines, &file, &old_changed, &new_changed);
        }
    }
    super::shared::normalize_stage_line_selections(lines, old_lines);
}
