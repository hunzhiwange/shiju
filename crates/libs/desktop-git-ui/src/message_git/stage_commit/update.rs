//! 提交表单、选择和提交流程的领域入口。
use super::*;
use crate::git_panel::types::ConventionalCommitType;
use crate::message_git::messages::GitMessage;
use crate::message_git::shared::{
    SelectedCommitRequest, build_commit_request, reset_commit_form_state,
};
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet};

pub struct UpdateInput<'a> {
    pub context_expansions: &'a mut HashMap<(String, usize), (usize, usize)>,
    pub expanded_files: &'a mut Vec<String>,
    pub expanded_files_set: &'a mut HashSet<String>,
    pub expanded_hunks: &'a mut Vec<(String, usize)>,
    pub git_commit_description: &'a mut String,
    pub git_commit_description_editor: &'a mut text_editor::Content,
    pub git_commit_in_progress: &'a mut bool,
    pub git_commit_message: &'a mut String,
    pub git_commit_scope: &'a mut String,
    pub git_commit_type: &'a mut Option<ConventionalCommitType>,
    pub git_filter_deleted: &'a mut bool,
    pub git_filter_excluded: &'a mut bool,
    pub git_filter_included: &'a mut bool,
    pub git_filter_modified: &'a mut bool,
    pub git_filter_new: &'a mut bool,
    pub git_filter_query: &'a mut String,
    pub git_hovered_file_header: &'a mut Option<String>,
    pub git_panel_header_hovered: &'a mut bool,
    pub show_git_commit_help_modal: &'a mut bool,
    pub show_git_filter_help_modal: &'a mut bool,
    pub show_git_filter_options: &'a mut bool,
    pub staged_files_selected: &'a mut Vec<String>,
    pub staged_hunks_selected: &'a mut Vec<(String, usize)>,
    pub staged_lines_selected: &'a mut Vec<(String, usize)>,
    pub staged_old_lines_selected: &'a mut Vec<(String, usize)>,
    pub remote_busy: bool,
    pub context_path: Option<String>,
    pub repo_path: Option<String>,
    pub animate: bool,
}

#[derive(Debug, Clone)]
pub enum Effect {
    StageAll(String),
    Commit { path: String, request: SelectedCommitRequest, animate: bool },
    InputError(&'static str),
    Error(String),
    NotifyRaw(String),
    Notify { message: &'static str, error: Option<String> },
    Toast { success: bool, message: &'static str, error: Option<String> },
    RefreshAfterMutation,
    RefreshBranches,
}

pub fn update(app: &mut UpdateInput<'_>, message: GitMessage) -> Vec<Effect> {
    let mut effects = Vec::new();
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::CommitMessageChanged(message) => {
            commit_message_changed(&mut *app.git_commit_message, message);
        }
        GitMessage::CommitTypeSelected(commit_type) => {
            commit_type_selected(&mut *app.git_commit_type, commit_type);
        }
        GitMessage::CommitScopeChanged(scope) => {
            commit_scope_changed(&mut *app.git_commit_scope, scope);
        }
        GitMessage::CommitDescriptionChanged(description) => {
            commit_description_changed(&mut *app.git_commit_description, description);
        }
        GitMessage::CommitDescriptionEditorAction(action) => {
            commit_description_editor_action(
                &mut *app.git_commit_description_editor,
                &mut *app.git_commit_description,
                action,
            );
        }
        GitMessage::CommitHelpOpen => {
            commit_help_open(&mut *app.show_git_commit_help_modal);
        }
        GitMessage::CommitHelpClose => {
            commit_help_close(&mut *app.show_git_commit_help_modal);
        }
        GitMessage::FilterHelpOpen => {
            filter_help_open(&mut *app.show_git_filter_help_modal);
        }
        GitMessage::FilterHelpClose => {
            filter_help_close(&mut *app.show_git_filter_help_modal);
        }
        GitMessage::ToggleFilterOptions(enabled) => {
            toggle_filter_options(&mut *app.show_git_filter_options, enabled);
        }
        GitMessage::FilterQueryChanged(query) => {
            filter_query_changed(&mut *app.git_filter_query, query);
        }
        GitMessage::FilterToggleIncluded(enabled) => {
            filter_toggle_included(&mut *app.git_filter_included, enabled);
        }
        GitMessage::FilterToggleExcluded(enabled) => {
            filter_toggle_excluded(&mut *app.git_filter_excluded, enabled);
        }
        GitMessage::FilterToggleNew(enabled) => {
            filter_toggle_new(&mut *app.git_filter_new, enabled);
        }
        GitMessage::FilterToggleModified(enabled) => {
            filter_toggle_modified(&mut *app.git_filter_modified, enabled);
        }
        GitMessage::FilterToggleDeleted(enabled) => {
            filter_toggle_deleted(&mut *app.git_filter_deleted, enabled);
        }
        GitMessage::ClearFilters => {
            clear_filters(
                &mut *app.git_filter_query,
                &mut *app.git_filter_included,
                &mut *app.git_filter_excluded,
                &mut *app.git_filter_new,
                &mut *app.git_filter_modified,
                &mut *app.git_filter_deleted,
            );
        }
        GitMessage::ToggleStageFile(file, checked) => {
            toggle_stage_file(&mut *app.staged_files_selected, file, checked);
        }
        GitMessage::ToggleStageHunk(file, idx, checked) => {
            toggle_stage_hunk(&mut *app.staged_hunks_selected, file, idx, checked);
        }
        GitMessage::ToggleStageLine(file, new_idx, checked) => {
            toggle_stage_line(&mut *app.staged_lines_selected, file, new_idx, checked);
        }
        GitMessage::ToggleStageOldLine(file, old_idx, checked) => {
            toggle_stage_old_line(&mut *app.staged_old_lines_selected, file, old_idx, checked);
        }
        GitMessage::SelectAllFileLines(_file) =>
        {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = app.repo_path.clone() {
                select_all_file_lines(
                    &mut *app.staged_lines_selected,
                    &mut *app.staged_old_lines_selected,
                    &path,
                    &_file,
                );
            }
        }
        GitMessage::SelectAllVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = app.repo_path.clone();
                select_visible_file_lines(
                    &mut *app.staged_lines_selected,
                    &mut *app.staged_old_lines_selected,
                    path.as_deref(),
                    _files,
                    false,
                );
            }
        }
        GitMessage::InvertVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = app.repo_path.clone();
                select_visible_file_lines(
                    &mut *app.staged_lines_selected,
                    &mut *app.staged_old_lines_selected,
                    path.as_deref(),
                    _files,
                    true,
                );
            }
        }
        GitMessage::ClearAllFileLines(file) => {
            clear_all_file_lines(
                &mut *app.staged_lines_selected,
                &mut *app.staged_old_lines_selected,
                file,
            );
        }
        GitMessage::HoverFileHeaderEnter(file) => {
            hover_file_header_enter(&mut *app.git_hovered_file_header, file);
        }
        GitMessage::HoverFileHeaderExit(file) => {
            hover_file_header_exit(&mut *app.git_hovered_file_header, file);
        }
        GitMessage::HoverGitPanelHeaderEnter => {
            hover_git_panel_header_enter(&mut *app.git_panel_header_hovered);
        }
        GitMessage::HoverGitPanelHeaderExit => {
            hover_git_panel_header_exit(&mut *app.git_panel_header_hovered);
        }
        GitMessage::StageAll => {
            if *app.git_commit_in_progress || app.remote_busy {
                return effects;
            }
            let Some(path) = app.context_path.clone() else {
                effects.push(Effect::InputError("未找到项目路径，无法通过网关暂存"));
                effects.push(Effect::Notify {
                    message: "未找到项目路径，无法通过网关暂存",
                    error: None,
                });
                return effects;
            };

            *app.git_commit_in_progress = true;
            effects.push(Effect::StageAll(path));
        }
        GitMessage::StageAllFinished(result) => {
            *app.git_commit_in_progress = false;
            match result {
                Ok(()) => {
                    effects.push(Effect::Notify { message: "暂存完成", error: None });
                    {
                        effects.push(Effect::RefreshAfterMutation);
                        effects.push(Effect::Toast {
                            success: true,
                            message: "Git 变更已暂存",
                            error: None,
                        });
                    }
                }
                Err(error) => {
                    effects.push(Effect::Error(error.clone()));
                    effects.push(Effect::Notify {
                        message: "暂存失败: %s",
                        error: Some(error.clone()),
                    });
                    effects.push(Effect::Toast {
                        success: false,
                        message: "暂存 Git 变更失败：%s",
                        error: Some(error),
                    })
                }
            }
        }
        message @ (GitMessage::CommitSelected | GitMessage::CommitAll) => {
            if *app.git_commit_in_progress || app.remote_busy {
                return effects;
            }

            let request = match message {
                GitMessage::CommitAll => build_commit_request(
                    app.git_commit_message,
                    app.git_commit_description,
                    app.git_commit_type,
                    app.git_commit_scope,
                    app.staged_files_selected,
                    app.staged_hunks_selected,
                    app.staged_lines_selected,
                    app.staged_old_lines_selected,
                    true,
                ),
                GitMessage::CommitSelected => build_commit_request(
                    app.git_commit_message,
                    app.git_commit_description,
                    app.git_commit_type,
                    app.git_commit_scope,
                    app.staged_files_selected,
                    app.staged_hunks_selected,
                    app.staged_lines_selected,
                    app.staged_old_lines_selected,
                    false,
                ),
                _ => unreachable!("commit action pattern only accepts commit variants"),
            };
            let request = match request {
                Ok(request) => request,
                Err(error) => {
                    if error == "提交消息不能为空" {
                        effects.push(Effect::InputError("提交消息不能为空"));
                        effects.push(Effect::Notify {
                            message: "提交消息不能为空", error: None
                        });
                    } else {
                        effects.push(Effect::Error(error.clone()));
                        effects.push(Effect::NotifyRaw(error));
                    }
                    return effects;
                }
            };

            *app.git_commit_in_progress = true;
            let Some(path) = app.context_path.clone() else {
                *app.git_commit_in_progress = false;
                effects.push(Effect::InputError("未找到项目路径，无法通过网关提交"));
                effects.push(Effect::Notify {
                    message: "未找到项目路径，无法通过网关提交",
                    error: None,
                });
                return effects;
            };
            effects.push(Effect::Commit { path, request, animate: app.animate });
        }
        GitMessage::CommitSelectedFinished(result) => {
            *app.git_commit_in_progress = false;
            match result {
                Ok(()) => {
                    reset_commit_form_state(
                        app.staged_files_selected,
                        app.staged_hunks_selected,
                        app.staged_lines_selected,
                        app.staged_old_lines_selected,
                        app.expanded_files,
                        app.expanded_files_set,
                        app.expanded_hunks,
                        app.context_expansions,
                        app.git_commit_message,
                        app.git_commit_type,
                        app.git_commit_scope,
                        app.git_commit_description,
                        app.git_commit_description_editor,
                    );
                    effects.push(Effect::RefreshBranches);
                    effects.push(Effect::Notify { message: "提交完成", error: None });
                    {
                        effects.push(Effect::RefreshAfterMutation);
                        effects.push(Effect::Toast {
                            success: true,
                            message: "提交成功",
                            error: None,
                        });
                    }
                }
                Err(error) => {
                    effects.push(Effect::Error(error.clone()));
                    effects.push(Effect::Notify {
                        message: "提交失败: %s",
                        error: Some(error.clone()),
                    });
                }
            }
        }
        _ => unreachable!("unexpected stage/commit git message"),
    }
    effects
}
