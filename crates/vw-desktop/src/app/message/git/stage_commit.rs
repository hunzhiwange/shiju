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
            desktop_git_ui::message_git::stage_commit::commit_message_changed(&mut app.git_commit_message, message);
            Task::none()
        }
        GitMessage::CommitTypeSelected(commit_type) => {
            desktop_git_ui::message_git::stage_commit::commit_type_selected(&mut app.git_commit_type, commit_type);
            Task::none()
        }
        GitMessage::CommitScopeChanged(scope) => {
            desktop_git_ui::message_git::stage_commit::commit_scope_changed(&mut app.git_commit_scope, scope);
            Task::none()
        }
        GitMessage::CommitDescriptionChanged(description) => {
            desktop_git_ui::message_git::stage_commit::commit_description_changed(&mut app.git_commit_description, description);
            Task::none()
        }
        GitMessage::CommitDescriptionEditorAction(action) => {
            desktop_git_ui::message_git::stage_commit::commit_description_editor_action(&mut app.git_commit_description_editor, &mut app.git_commit_description, action);
            Task::none()
        }
        GitMessage::CommitHelpOpen => {
            desktop_git_ui::message_git::stage_commit::commit_help_open(&mut app.show_git_commit_help_modal);
            Task::none()
        }
        GitMessage::CommitHelpClose => {
            desktop_git_ui::message_git::stage_commit::commit_help_close(&mut app.show_git_commit_help_modal);
            Task::none()
        }
        GitMessage::FilterHelpOpen => {
            desktop_git_ui::message_git::stage_commit::filter_help_open(&mut app.show_git_filter_help_modal);
            Task::none()
        }
        GitMessage::FilterHelpClose => {
            desktop_git_ui::message_git::stage_commit::filter_help_close(&mut app.show_git_filter_help_modal);
            Task::none()
        }
        GitMessage::ToggleFilterOptions(enabled) => {
            desktop_git_ui::message_git::stage_commit::toggle_filter_options(&mut app.show_git_filter_options, enabled);
            Task::none()
        }
        GitMessage::FilterQueryChanged(query) => {
            desktop_git_ui::message_git::stage_commit::filter_query_changed(&mut app.git_filter_query, query);
            Task::none()
        }
        GitMessage::FilterToggleIncluded(enabled) => {
            desktop_git_ui::message_git::stage_commit::filter_toggle_included(&mut app.git_filter_included, enabled);
            Task::none()
        }
        GitMessage::FilterToggleExcluded(enabled) => {
            desktop_git_ui::message_git::stage_commit::filter_toggle_excluded(&mut app.git_filter_excluded, enabled);
            Task::none()
        }
        GitMessage::FilterToggleNew(enabled) => {
            desktop_git_ui::message_git::stage_commit::filter_toggle_new(&mut app.git_filter_new, enabled);
            Task::none()
        }
        GitMessage::FilterToggleModified(enabled) => {
            desktop_git_ui::message_git::stage_commit::filter_toggle_modified(&mut app.git_filter_modified, enabled);
            Task::none()
        }
        GitMessage::FilterToggleDeleted(enabled) => {
            desktop_git_ui::message_git::stage_commit::filter_toggle_deleted(&mut app.git_filter_deleted, enabled);
            Task::none()
        }
        GitMessage::ClearFilters => {
            desktop_git_ui::message_git::stage_commit::clear_filters(&mut app.git_filter_query, &mut app.git_filter_included, &mut app.git_filter_excluded, &mut app.git_filter_new, &mut app.git_filter_modified, &mut app.git_filter_deleted);
            Task::none()
        }
        GitMessage::ToggleStageFile(file, checked) => {
            desktop_git_ui::message_git::stage_commit::toggle_stage_file(&mut app.staged_files_selected, file, checked);
            Task::none()
        }
        GitMessage::ToggleStageHunk(file, idx, checked) => {
            desktop_git_ui::message_git::stage_commit::toggle_stage_hunk(&mut app.staged_hunks_selected, file, idx, checked);
            Task::none()
        }
        GitMessage::ToggleStageLine(file, new_idx, checked) => {
            desktop_git_ui::message_git::stage_commit::toggle_stage_line(&mut app.staged_lines_selected, file, new_idx, checked);
            Task::none()
        }
        GitMessage::ToggleStageOldLine(file, old_idx, checked) => {
            desktop_git_ui::message_git::stage_commit::toggle_stage_old_line(&mut app.staged_old_lines_selected, file, old_idx, checked);
            Task::none()
        }
        GitMessage::SelectAllFileLines(_file) => {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = super::shared::git_repo_path_for_app(app) {
                desktop_git_ui::message_git::stage_commit::select_all_file_lines(
                    &mut app.staged_lines_selected, &mut app.staged_old_lines_selected, &path, &_file,
                );
            }
            Task::none()
        }
        GitMessage::SelectAllVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = super::shared::git_repo_path_for_app(app);
                desktop_git_ui::message_git::stage_commit::select_visible_file_lines(
                    &mut app.staged_lines_selected, &mut app.staged_old_lines_selected,
                    path.as_deref(), _files, false,
                );
            }
            Task::none()
        }
        GitMessage::InvertVisibleFileLines(_files) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = super::shared::git_repo_path_for_app(app);
                desktop_git_ui::message_git::stage_commit::select_visible_file_lines(
                    &mut app.staged_lines_selected, &mut app.staged_old_lines_selected,
                    path.as_deref(), _files, true,
                );
            }
            Task::none()
        }
        GitMessage::ClearAllFileLines(file) => {
            desktop_git_ui::message_git::stage_commit::clear_all_file_lines(&mut app.staged_lines_selected, &mut app.staged_old_lines_selected, file);
            Task::none()
        }
        GitMessage::HoverFileHeaderEnter(file) => {
            desktop_git_ui::message_git::stage_commit::hover_file_header_enter(&mut app.git_hovered_file_header, file);
            Task::none()
        }
        GitMessage::HoverFileHeaderExit(file) => {
            desktop_git_ui::message_git::stage_commit::hover_file_header_exit(&mut app.git_hovered_file_header, file);
            Task::none()
        }
        GitMessage::HoverGitPanelHeaderEnter => {
            desktop_git_ui::message_git::stage_commit::hover_git_panel_header_enter(&mut app.git_panel_header_hovered);
            Task::none()
        }
        GitMessage::HoverGitPanelHeaderExit => {
            desktop_git_ui::message_git::stage_commit::hover_git_panel_header_exit(&mut app.git_panel_header_hovered);
            Task::none()
        }
        GitMessage::StageAll => {
            if app.git_commit_in_progress || app.git_repository.remote_busy {
                return Task::none();
            }
            let Some(path) = git_context_path_for_app(app) else {
                app.set_localized_input_error(
                    crate::app::i18n::input::ErrorField::General,
                    "未找到项目路径，无法通过网关暂存",
                    vec![],
                );
                app.push_localized_notification("未找到项目路径，无法通过网关暂存", vec![]);
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
                    app.push_localized_notification("暂存完成", vec![]);
                    Task::batch(vec![
                        refresh_git_panel_data_after_repo_mutation_task(app),
                        app.show_localized_toast(
                            crate::app::state::ToastKind::Success,
                            "Git 变更已暂存",
                            vec![],
                        ),
                    ])
                }
                Err(error) => {
                    app.error_message = Some(error.clone());
                    app.push_localized_notification("暂存失败: %s", vec![error.clone().into()]);
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "暂存 Git 变更失败：%s",
                        vec![error.into()],
                    )
                }
            }
        }
        message @ (GitMessage::CommitSelected | GitMessage::CommitAll) => {
            if app.git_commit_in_progress || app.git_repository.remote_busy {
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
                    if error == "提交消息不能为空" {
                        app.set_localized_input_error(
                            crate::app::i18n::input::ErrorField::General,
                            "提交消息不能为空",
                            vec![],
                        );
                        app.push_localized_notification("提交消息不能为空", vec![]);
                    } else {
                        app.error_message = Some(error.clone());
                        app.push_notification(error);
                    }
                    return Task::none();
                }
            };

            app.git_commit_in_progress = true;
            let animation_tick = (!app.file_manager.file_manager_changes_refreshing
                && !app.file_manager.file_manager_file_tree_refreshing)
                .then(schedule_commit_button_animation_tick);
            let Some(path) = git_context_path_for_app(app) else {
                app.git_commit_in_progress = false;
                app.set_localized_input_error(
                    crate::app::i18n::input::ErrorField::General,
                    "未找到项目路径，无法通过网关提交",
                    vec![],
                );
                app.push_localized_notification("未找到项目路径，无法通过网关提交", vec![]);
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
                    app.push_localized_notification("提交完成", vec![]);
                    Task::batch(vec![
                        refresh_git_panel_data_after_repo_mutation_task(app),
                        app.show_localized_toast(
                            crate::app::state::ToastKind::Success,
                            "提交成功",
                            vec![],
                        ),
                    ])
                }
                Err(error) => {
                    app.error_message = Some(error.clone());
                    app.push_localized_notification("提交失败: %s", vec![error.clone().into()]);
                    Task::none()
                }
            }
        }
        _ => unreachable!("unexpected stage/commit git message"),
    }
}
