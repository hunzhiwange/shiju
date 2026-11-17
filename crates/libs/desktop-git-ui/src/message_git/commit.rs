//! Git 提交表单与提交请求。
use crate::git_panel::types::ConventionalCommitType;
use std::collections::{HashMap, HashSet};
use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub struct SelectedCommitRequest {
    pub message: String,
    pub stage_all: bool,
    pub selected_files: Vec<String>,
    pub selected_hunks: Vec<(String, usize)>,
    pub selected_lines: Vec<(String, usize)>,
    pub selected_old_lines: Vec<(String, usize)>,
}

pub fn build_commit_request(
    git_commit_message: &String,
    git_commit_description: &String,
    git_commit_type: &Option<ConventionalCommitType>,
    git_commit_scope: &String,
    staged_files_selected: &Vec<String>,
    staged_hunks_selected: &Vec<(String, usize)>,
    staged_lines_selected: &Vec<(String, usize)>,
    staged_old_lines_selected: &Vec<(String, usize)>,
    stage_all: bool,
) -> Result<SelectedCommitRequest, String> {
    let summary = (*git_commit_message).trim().to_string();
    let body = (*git_commit_description).trim().to_string();
    let subject = if let Some(commit_type) = *git_commit_type {
        let scope = (*git_commit_scope).trim();
        if scope.is_empty() {
            format!("{}: {}", commit_type.as_str(), summary)
        } else {
            format!("{}({}): {}", commit_type.as_str(), scope, summary)
        }
    } else {
        summary.clone()
    };

    let message = if body.is_empty() {
        subject.trim().to_string()
    } else {
        format!("{}\n\n{}", subject.trim(), body)
    };
    if message.trim().is_empty() {
        return Err("提交消息不能为空".to_string());
    }

    Ok(SelectedCommitRequest {
        message,
        stage_all,
        selected_files: (*staged_files_selected).clone(),
        selected_hunks: (*staged_hunks_selected).clone(),
        selected_lines: (*staged_lines_selected).clone(),
        selected_old_lines: (*staged_old_lines_selected).clone(),
    })
}

pub fn reset_commit_form_state(
    staged_files_selected: &mut Vec<String>,
    staged_hunks_selected: &mut Vec<(String, usize)>,
    staged_lines_selected: &mut Vec<(String, usize)>,
    staged_old_lines_selected: &mut Vec<(String, usize)>,
    expanded_files: &mut Vec<String>,
    expanded_files_set: &mut HashSet<String>,
    expanded_hunks: &mut Vec<(String, usize)>,
    context_expansions: &mut HashMap<(String, usize), (usize, usize)>,
    git_commit_message: &mut String,
    git_commit_type: &mut Option<ConventionalCommitType>,
    git_commit_scope: &mut String,
    git_commit_description: &mut String,
    git_commit_description_editor: &mut text_editor::Content,
) {
    (*staged_files_selected).clear();
    (*staged_hunks_selected).clear();
    (*staged_lines_selected).clear();
    (*staged_old_lines_selected).clear();
    super::diff::clear_expanded_files(expanded_files, expanded_files_set);
    (*expanded_hunks).clear();
    (*context_expansions).clear();
    (*git_commit_message).clear();
    (*git_commit_type) = Some(ConventionalCommitType::Feat);
    (*git_commit_scope).clear();
    (*git_commit_description).clear();
    (*git_commit_description_editor) = iced::widget::text_editor::Content::new();
}
