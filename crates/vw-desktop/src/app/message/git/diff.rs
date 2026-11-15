//! 处理 Git diff 面板消息，负责差异加载、刷新和选择状态更新。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use super::shared::{dismiss_preview_transient_ui, load_missing_diff_contents, refresh_git_panel_data_task};
use super::{ExpandDirection, GitMessage};

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::ToggleDiffHighlight(enabled) => {
            app.show_git_diff_highlight = enabled;
            app.save_desktop_preference("show_git_diff_highlight", serde_json::Value::Bool(enabled))
        }
        GitMessage::ToggleDiffLineSelection(file, line, is_old, text) => {
            desktop_git_ui::message_git::diff::toggle_diff_line_selection(&mut app.git_diff_selected_range, &mut app.git_diff_selected_lines, file, line, is_old, text);
            Task::none()
        }
        GitMessage::DiffDragSelectStart(file, line, is_old, text) => {
            desktop_git_ui::message_git::diff::diff_drag_select_start(&mut app.git_diff_comment_draft, &mut app.git_diff_context_menu, &mut app.git_diff_selected_range, &mut app.git_diff_dragging, &mut app.git_diff_drag_start_text, &mut app.git_diff_drag_range, file, line, is_old, text);
            Task::none()
        }
        GitMessage::DiffDragSelectHover(file, line, is_old) => {
            desktop_git_ui::message_git::diff::diff_drag_select_hover(&mut app.git_diff_dragging, &mut app.git_diff_drag_range, file, line, is_old);
            Task::none()
        }
        GitMessage::DiffDragSelectEnd => {
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let effect = desktop_git_ui::message_git::diff::diff_drag_select_end(&mut app.git_diff_dragging, &mut app.git_diff_drag_start_text, &mut app.git_diff_drag_range, &mut app.git_diff_last_click, &mut app.git_diff_selected_range, &mut app.git_diff_comment_draft, &mut app.git_diff_selected_lines, repo_path);
            apply_effect(app, effect)
        }
        GitMessage::OpenDiffContextMenu { file, line, is_old, text, x, y } => {
            dismiss_preview_transient_ui(app);
            let effect = desktop_git_ui::message_git::diff::open_diff_context_menu(&mut app.git_diff_selected_range, &mut app.git_diff_selected_lines, &mut app.git_diff_context_menu, file, line, is_old, text, x, y);
            apply_effect(app, effect)
        }
        GitMessage::CloseDiffContextMenu => {
            desktop_git_ui::message_git::diff::close_diff_context_menu(&mut app.git_diff_context_menu);
            Task::none()
        }
        GitMessage::OpenDiffFileMenu(file) => {
            dismiss_preview_transient_ui(app);
            let effect = desktop_git_ui::message_git::diff::open_diff_file_menu(&mut app.git_diff_context_menu, &mut app.git_diff_file_menu, file);
            apply_effect(app, effect)
        }
        GitMessage::CloseDiffFileMenu => {
            desktop_git_ui::message_git::diff::close_diff_file_menu(&mut app.git_diff_file_menu);
            Task::none()
        }
        GitMessage::PreviewDiffFile(file) => {
            app.git_diff_file_menu = None;
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            if let Some(error) = crate::app::preview::preview_open_error_message(&full_path) {
                let fallback = error
                    .render(crate::app::i18n::I18n::new(
                        config_ui::DesktopLocale::SimplifiedChinese,
                    ))
                    .expect("preview error template");
                let task = app.show_error_toast(fallback);
                if let Some(toast) = &mut app.active_toast {
                    toast.localized_message = Some(error);
                }
                return task;
            }
            Task::done(Message::Preview(crate::app::message::PreviewMessage::Open(full_path)))
        }
        GitMessage::CopyDiffFile { file, deleted_content } => {
            app.git_diff_file_menu = None;
            if let Some(content) = deleted_content {
                return Task::done(Message::CopyCode(content));
            }
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let full_path = if let Some(base) = &repo_path {
                std::path::Path::new(base).join(&file).to_string_lossy().to_string()
            } else {
                file
            };
            Task::done(Message::CopyFile(full_path))
        }
        GitMessage::RevertDiffFile(file) => {
            desktop_git_ui::message_git::diff::revert_diff_file(&mut app.git_diff_file_menu, &mut app.file_to_discard, file);
            Task::none()
        }
        GitMessage::OpenDiffCommentDraft => {
            let effect = desktop_git_ui::message_git::diff::open_diff_comment_draft(&mut app.git_diff_context_menu, &mut app.git_diff_selected_range, &mut app.git_diff_selected_lines, &mut app.git_diff_comment_draft);
            apply_effect(app, effect)
        }
        GitMessage::SelectDiffContextStageLines => {
            let effect = desktop_git_ui::message_git::diff::select_diff_context_stage_lines(&mut app.git_diff_context_menu, &mut app.git_diff_selected_range, &mut app.git_diff_selected_lines, &mut app.staged_old_lines_selected, &mut app.staged_lines_selected);
            apply_effect(app, effect)
        }
        GitMessage::ClearDiffContextStageLines => {
            let effect = desktop_git_ui::message_git::diff::clear_diff_context_stage_lines(&mut app.git_diff_context_menu, &mut app.git_diff_selected_range, &mut app.git_diff_selected_lines, &mut app.staged_old_lines_selected, &mut app.staged_lines_selected);
            apply_effect(app, effect)
        }
        GitMessage::DiffHoverEnter(file, line, is_old) => {
            desktop_git_ui::message_git::diff::diff_hover_enter(&mut app.git_diff_hovered_line, file, line, is_old);
            Task::none()
        }
        GitMessage::DiffHoverExit(file, line, is_old) => {
            desktop_git_ui::message_git::diff::diff_hover_exit(&mut app.git_diff_hovered_line, file, line, is_old);
            Task::none()
        }
        GitMessage::DiffCommentEditorAction(action) => {
            desktop_git_ui::message_git::diff::diff_comment_editor_action(&mut app.git_diff_comment_draft, action);
            Task::none()
        }
        GitMessage::DiffCommentCancel => {
            desktop_git_ui::message_git::diff::diff_comment_cancel(&mut app.git_diff_comment_draft, &mut app.git_diff_selected_range);
            Task::none()
        }
        GitMessage::DiffCommentSubmit => {
            let effect = desktop_git_ui::message_git::diff::diff_comment_submit(&mut app.git_diff_comment_draft, &mut app.git_diff_selected_range);
            apply_effect(app, effect)
        }
        GitMessage::CopyDiffSelection => {
            let effect = desktop_git_ui::message_git::diff::copy_diff_selection(&mut app.git_diff_context_menu, &mut app.git_diff_selected_lines);
            apply_effect(app, effect)
        }
        GitMessage::DiscardDiffSelection => {
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let effect = desktop_git_ui::message_git::diff::discard_diff_selection(&mut app.git_diff_context_menu, &mut app.git_diff_selected_lines, &mut app.git_diff_selected_range, repo_path);
            apply_effect(app, effect)
        }
        GitMessage::InsertDiffSelectionToChat => {
            let effect = desktop_git_ui::message_git::diff::insert_diff_selection_to_chat(&mut app.git_diff_context_menu, &mut app.git_diff_selected_lines);
            apply_effect(app, effect)
        }
        GitMessage::InsertDiffSelectionComment => {
            let effect = desktop_git_ui::message_git::diff::insert_diff_selection_comment(&mut app.git_diff_context_menu, &mut app.git_diff_selected_lines);
            apply_effect(app, effect)
        }
        GitMessage::ConfirmDiscardFile(file) => {
            desktop_git_ui::message_git::diff::confirm_discard_file(&mut app.git_diff_file_menu, &mut app.file_to_discard, file);
            Task::none()
        }
        GitMessage::CancelDiscardFile => {
            desktop_git_ui::message_git::diff::cancel_discard_file(&mut app.git_diff_file_menu, &mut app.file_to_discard);
            Task::none()
        }
        GitMessage::DiscardFile(_file) => {
            desktop_git_ui::message_git::diff::cancel_discard_file(&mut app.git_diff_file_menu, &mut app.file_to_discard);
            desktop_git_ui::message_git::diff::discard(app.project_path.as_deref(), app.terminal.shell, desktop_git_ui::message_git::diff::DiscardAction::File(_file));
            refresh_git_panel_data_task()
        }
        GitMessage::DiscardHunk(_file, _idx) => {
            desktop_git_ui::message_git::diff::discard(app.project_path.as_deref(), app.terminal.shell, desktop_git_ui::message_git::diff::DiscardAction::Hunk(_file, _idx));
            refresh_git_panel_data_task()
        }
        GitMessage::RevertLineDelete(_file, _line_idx) => {
            desktop_git_ui::message_git::diff::discard(app.project_path.as_deref(), app.terminal.shell, desktop_git_ui::message_git::diff::DiscardAction::DeleteLine(_file, _line_idx));
            refresh_git_panel_data_task()
        }
        GitMessage::RevertLineRestore(_file, _new_idx, _old_idx) => {
            desktop_git_ui::message_git::diff::discard(app.project_path.as_deref(), app.terminal.shell, desktop_git_ui::message_git::diff::DiscardAction::RestoreLine(_file, _new_idx, _old_idx));
            refresh_git_panel_data_task()
        }
        GitMessage::ToggleFullscreen => {
            let active = desktop_git_ui::message_git::diff::toggle_fullscreen(&mut app.git_diff_fullscreen, &mut app.git_diff_half_fullscreen, false);
            app.fullscreen_layout_settling = true;
            if active {
                app.chat_panel_fullscreen = false;
                app.chat_panel_half_fullscreen = false;
                app.show_diff = true;
            }
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(crate::app::message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        GitMessage::ToggleHalfFullscreen => {
            let active = desktop_git_ui::message_git::diff::toggle_fullscreen(&mut app.git_diff_fullscreen, &mut app.git_diff_half_fullscreen, true);
            app.fullscreen_layout_settling = true;
            if active {
                app.chat_panel_fullscreen = false;
                app.chat_panel_half_fullscreen = false;
                app.show_diff = true;
            }
            crate::app::message::after(
                Duration::from_millis(180),
                Message::View(crate::app::message::ViewMessage::FullscreenLayoutSettled),
            )
        }
        GitMessage::ToggleExpandHunk(file, idx) => {
            desktop_git_ui::message_git::diff::toggle_expand_hunk(&mut app.expanded_hunks, file, idx);
            Task::none()
        }
        GitMessage::ExpandContext(file, gap_idx, direction) => {
            let direction = match direction {
                ExpandDirection::Up => desktop_git_ui::message_git::diff::ExpandDirection::Up,
                ExpandDirection::Down => desktop_git_ui::message_git::diff::ExpandDirection::Down,
                ExpandDirection::All => desktop_git_ui::message_git::diff::ExpandDirection::All,
            };
            desktop_git_ui::message_git::diff::expand_context(&mut app.context_expansions, file, gap_idx, direction);
            Task::none()
        }
        GitMessage::ToggleExpandFile(name) => {
            if desktop_git_ui::message_git::diff::toggle_expand_file(&mut app.expanded_files, &mut app.expanded_files_set, name.clone()) {
                load_missing_diff_contents(app, std::iter::once(name))
            } else { Task::none() }
        }
        GitMessage::FocusFile(name) => {
            dismiss_preview_transient_ui(app);
            app.git_focused_file = Some(name.clone());
            app.ensure_diff_file_expanded(name.clone());
            app.active_preview_path = None;
            app.show_diff = true;
            app.chat_text_diff = None;
            load_missing_diff_contents(app, std::iter::once(name))
        }
        GitMessage::DiffScrollChanged { offset_y, viewport_h } => {
            desktop_git_ui::message_git::diff::scroll_changed(&mut app.git_diff_scroll_offset_y, &mut app.git_diff_scroll_viewport_h, offset_y, viewport_h);
            Task::none()
        }
        GitMessage::DiffThemeSelected(theme) => {
            desktop_git_ui::message_git::diff::diff_theme_selected(&mut app.diff_theme, theme);
            Task::none()
        }
        _ => unreachable!("unexpected diff git message"),
    }
}

fn apply_effect(_app: &mut App, effect: desktop_git_ui::message_git::diff::Effect) -> Task<Message> {
    use desktop_git_ui::message_git::diff::Effect;
    match effect {
        Effect::None => Task::none(),
        Effect::CopyCode(content) => Task::done(Message::CopyCode(content)),
        Effect::AppendChat(text) => Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(text))),
        Effect::Refresh => refresh_git_panel_data_task(),
    }
}
