//! 处理 Git 暂存与提交流程，连接界面操作和后台 Git 命令任务。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::GitMessage;
use super::shared::{
    build_all_commit_request, build_selected_commit_request, execute_selected_commit_via_gateway,
    execute_stage_all_via_gateway, git_context_path_for_app,
    refresh_git_panel_data_after_repo_mutation_task, reset_commit_form_state,
    schedule_commit_button_animation_tick,
};
#[cfg(not(target_arch = "wasm32"))]
use super::shared::{
    changed_diff_line_sets, invert_stage_selection_for_file_lines, normalize_stage_line_selections,
    replace_stage_selection_for_file_lines,
};
use crate::app::{App, Message};
use iced::Task;

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::CommitMessageChanged(message) => {
            app.git_commit_message = message;
            Task::none()
        }
        GitMessage::CommitTypeSelected(commit_type) => {
            app.git_commit_type = Some(commit_type);
            Task::none()
        }
        GitMessage::CommitScopeChanged(scope) => {
            app.git_commit_scope = scope;
            Task::none()
        }
        GitMessage::CommitDescriptionChanged(description) => {
            app.git_commit_description = description;
            Task::none()
        }
        GitMessage::CommitDescriptionEditorAction(action) => {
            app.git_commit_description_editor.perform(action);
            app.git_commit_description = app.git_commit_description_editor.text().to_string();
            Task::none()
        }
        GitMessage::CommitHelpOpen => {
            app.show_git_commit_help_modal = true;
            Task::none()
        }
        GitMessage::CommitHelpClose => {
            app.show_git_commit_help_modal = false;
            Task::none()
        }
        GitMessage::FilterHelpOpen => {
            app.show_git_filter_help_modal = true;
            Task::none()
        }
        GitMessage::FilterHelpClose => {
            app.show_git_filter_help_modal = false;
            Task::none()
        }
        GitMessage::ToggleFilterOptions(enabled) => {
            app.show_git_filter_options = enabled;
            Task::none()
        }
        GitMessage::FilterQueryChanged(query) => {
            app.git_filter_query = query;
            Task::none()
        }
        GitMessage::FilterToggleIncluded(enabled) => {
            app.git_filter_included = enabled;
            Task::none()
        }
        GitMessage::FilterToggleExcluded(enabled) => {
            app.git_filter_excluded = enabled;
            Task::none()
        }
        GitMessage::FilterToggleNew(enabled) => {
            app.git_filter_new = enabled;
            Task::none()
        }
        GitMessage::FilterToggleModified(enabled) => {
            app.git_filter_modified = enabled;
            Task::none()
        }
        GitMessage::FilterToggleDeleted(enabled) => {
            app.git_filter_deleted = enabled;
            Task::none()
        }
        GitMessage::ClearFilters => {
            app.git_filter_query.clear();
            app.git_filter_included = false;
            app.git_filter_excluded = false;
            app.git_filter_new = false;
            app.git_filter_modified = false;
            app.git_filter_deleted = false;
            Task::none()
        }
        GitMessage::ToggleStageFile(file, checked) => {
            if checked {
                if !app.staged_files_selected.contains(&file) {
                    app.staged_files_selected.push(file);
                }
            } else if let Some(pos) =
                app.staged_files_selected.iter().position(|selected| selected == &file)
            {
                app.staged_files_selected.remove(pos);
            }
            Task::none()
        }
        GitMessage::ToggleStageHunk(file, idx, checked) => {
            if checked {
                if !app.staged_hunks_selected.iter().any(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == idx
                }) {
                    app.staged_hunks_selected.push((file, idx));
                }
            } else if let Some(pos) =
                app.staged_hunks_selected.iter().position(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == idx
                })
            {
                app.staged_hunks_selected.remove(pos);
            }
            Task::none()
        }
        GitMessage::ToggleStageLine(file, new_idx, checked) => {
            if checked {
                if !app.staged_lines_selected.iter().any(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == new_idx
                }) {
                    app.staged_lines_selected.push((file, new_idx));
                }
            } else if let Some(pos) =
                app.staged_lines_selected.iter().position(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == new_idx
                })
            {
                app.staged_lines_selected.remove(pos);
            }
            Task::none()
        }
        GitMessage::ToggleStageOldLine(file, old_idx, checked) => {
            if checked {
                if !app.staged_old_lines_selected.iter().any(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == old_idx
                }) {
                    app.staged_old_lines_selected.push((file, old_idx));
                }
            } else if let Some(pos) =
                app.staged_old_lines_selected.iter().position(|(selected_file, selected_idx)| {
                    selected_file == &file && *selected_idx == old_idx
                })
            {
                app.staged_old_lines_selected.remove(pos);
            }
            Task::none()
        }
        GitMessage::SelectAllFileLines(_file) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let Some((old_changed, new_changed)) = changed_diff_line_sets(app, &_file) else {
                    return Task::none();
                };
                replace_stage_selection_for_file_lines(app, &_file, &old_changed, &new_changed);
                normalize_stage_line_selections(app);
            }
            Task::none()
        }
        GitMessage::SelectAllVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                for file in _files {
                    let Some((old_changed, new_changed)) = changed_diff_line_sets(app, &file)
                    else {
                        continue;
                    };
                    replace_stage_selection_for_file_lines(app, &file, &old_changed, &new_changed);
                }
                normalize_stage_line_selections(app);
            }
            Task::none()
        }
        GitMessage::InvertVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                for file in _files {
                    let Some((old_changed, new_changed)) = changed_diff_line_sets(app, &file)
                    else {
                        continue;
                    };
                    invert_stage_selection_for_file_lines(app, &file, &old_changed, &new_changed);
                }
                normalize_stage_line_selections(app);
            }
            Task::none()
        }
        GitMessage::ClearAllFileLines(file) => {
            app.staged_lines_selected.retain(|(selected_file, _)| selected_file != &file);
            app.staged_old_lines_selected.retain(|(selected_file, _)| selected_file != &file);
            Task::none()
        }
        GitMessage::HoverFileHeaderEnter(file) => {
            app.git_hovered_file_header = Some(file);
            Task::none()
        }
        GitMessage::HoverFileHeaderExit(file) => {
            if app.git_hovered_file_header.as_deref() == Some(file.as_str()) {
                app.git_hovered_file_header = None;
            }
            Task::none()
        }
        GitMessage::HoverGitPanelHeaderEnter => {
            app.git_panel_header_hovered = true;
            Task::none()
        }
        GitMessage::HoverGitPanelHeaderExit => {
            app.git_panel_header_hovered = false;
            Task::none()
        }
        GitMessage::StageAll => {
            if app.git_commit_in_progress {
                return Task::none();
            }
            let Some(path) = git_context_path_for_app(app) else {
                let error = "未找到项目路径，无法通过网关暂存".to_string();
                app.error_message = Some(error.clone());
                app.push_notification(error);
                return Task::none();
            };

            app.git_commit_in_progress = true;
            Task::perform(async move { execute_stage_all_via_gateway(path).await }, |result| {
                Message::Git(GitMessage::StageAllFinished(result))
            })
        }
        GitMessage::StageAllFinished(result) => {
            app.git_commit_in_progress = false;
            match result {
                Ok(()) => {
                    app.push_notification("暂存完成".to_string());
                    Task::batch(vec![
                        refresh_git_panel_data_after_repo_mutation_task(app),
                        app.show_success_toast("Git 变更已暂存"),
                    ])
                }
                Err(error) => {
                    app.error_message = Some(error.clone());
                    app.push_notification(format!("暂存失败: {error}"));
                    app.show_error_toast(format!("暂存 Git 变更失败：{error}"))
                }
            }
        }
        message @ (GitMessage::CommitSelected | GitMessage::CommitAll) => {
            if app.git_commit_in_progress {
                return Task::none();
            }

            let request = match message {
                GitMessage::CommitAll => build_all_commit_request(app),
                GitMessage::CommitSelected => build_selected_commit_request(app),
                _ => unreachable!("commit action pattern only accepts commit variants"),
            };
            let request = match request {
                Ok(request) => request,
                Err(error) => {
                    app.error_message = Some(error.clone());
                    app.push_notification(error);
                    return Task::none();
                }
            };

            app.git_commit_in_progress = true;
            let animation_tick = (!app.file_manager_changes_refreshing
                && !app.file_manager_file_tree_refreshing)
                .then(schedule_commit_button_animation_tick);
            let Some(path) = git_context_path_for_app(app) else {
                app.git_commit_in_progress = false;
                let error = "未找到项目路径，无法通过网关提交".to_string();
                app.error_message = Some(error.clone());
                app.push_notification(error);
                return Task::none();
            };
            let commit_task = Task::perform(
                async move { execute_selected_commit_via_gateway(path, request).await },
                |result| Message::Git(GitMessage::CommitSelectedFinished(result)),
            );
            if let Some(animation_tick) = animation_tick {
                Task::batch(vec![commit_task, animation_tick])
            } else {
                commit_task
            }
        }
        GitMessage::CommitSelectedFinished(result) => {
            app.git_commit_in_progress = false;
            match result {
                Ok(()) => {
                    reset_commit_form_state(app);
                    app.refresh_branches();
                    app.push_notification("提交完成".to_string());
                    Task::batch(vec![
                        refresh_git_panel_data_after_repo_mutation_task(app),
                        app.show_success_toast("提交成功"),
                    ])
                }
                Err(error) => {
                    app.error_message = Some(error.clone());
                    app.push_notification(format!("提交失败: {}", error));
                    Task::none()
                }
            }
        }
        _ => unreachable!("unexpected stage/commit git message"),
    }
}
