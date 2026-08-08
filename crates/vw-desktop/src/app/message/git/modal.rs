//! 处理 Git 相关弹窗的打开、关闭和输入状态变化。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::GitMessage;
use super::shared::{
    configure_git_code_editor, dismiss_preview_transient_ui, text_too_large_for_code_editor,
};
use crate::app::{App, Message};
use iced::Task;

fn open_copy_modal_with_text(app: &mut App, text: String) {
    app.show_git_copy_modal = true;
    app.git_copy_modal_use_color = !text_too_large_for_code_editor(&text);
    app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text(&text);
    if app.git_copy_modal_use_color {
        app.git_copy_modal_code_editor = iced_code_editor::CodeEditor::new(&text, "diff");
        let theme = app.effective_editor_theme();
        let font_size = app.current_font_size.clamp(10.0, 30.0);
        let line_height = app.current_line_height.clamp(10.0, 60.0);
        let language = app.current_language;
        configure_git_code_editor(
            theme,
            font_size,
            line_height,
            language,
            &mut app.git_copy_modal_code_editor,
        );
    }
}

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::OpenCustomDiffModal => {
            app.show_git_custom_diff_modal = true;
            app.git_custom_diff_hide_inputs = false;
            if app.git_custom_diff_title.trim().is_empty() {
                app.git_custom_diff_title = "自定义对比".to_string();
            }
            Task::none()
        }
        GitMessage::CloseCustomDiffModal => {
            app.show_git_custom_diff_modal = false;
            app.git_custom_diff_hide_inputs = false;
            Task::none()
        }
        GitMessage::CustomDiffTitleChanged(title) => {
            app.git_custom_diff_title = title;
            Task::none()
        }
        GitMessage::CustomDiffBeforeEditorAction(action) => {
            app.git_custom_diff_before_editor.perform(action);
            Task::none()
        }
        GitMessage::CustomDiffAfterEditorAction(action) => {
            app.git_custom_diff_after_editor.perform(action);
            Task::none()
        }
        GitMessage::CustomDiffSwap => {
            std::mem::swap(
                &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor,
            );
            Task::none()
        }
        GitMessage::OpenCustomDiffResult { title, before, after } => {
            dismiss_preview_transient_ui(app);
            app.show_git_custom_diff_modal = true;
            app.git_custom_diff_hide_inputs = true;
            app.git_custom_diff_title = title;
            app.git_custom_diff_before_editor =
                iced::widget::text_editor::Content::with_text(&before);
            app.git_custom_diff_after_editor =
                iced::widget::text_editor::Content::with_text(&after);
            app.active_preview_path = None;
            app.show_diff = true;
            Task::none()
        }
        GitMessage::OpenChatTextDiff { title, file: _, before, after } => {
            // 与 OpenCustomDiffResult 共用自定义 Diff 弹窗，避免在面板中再插一块独立消息框。
            dismiss_preview_transient_ui(app);
            app.chat_text_diff = None;
            app.show_git_custom_diff_modal = true;
            app.git_custom_diff_hide_inputs = true;
            app.git_custom_diff_title = title;
            app.git_custom_diff_before_editor =
                iced::widget::text_editor::Content::with_text(&before);
            app.git_custom_diff_after_editor =
                iced::widget::text_editor::Content::with_text(&after);
            app.active_preview_path = None;
            app.show_diff = true;
            app.file_manager_show_changes = true;
            Task::none()
        }
        GitMessage::CloseChatTextDiff => {
            app.chat_text_diff = None;
            app.show_git_custom_diff_modal = false;
            app.git_custom_diff_hide_inputs = false;
            Task::none()
        }
        GitMessage::OpenDiffCopyMode(file) => {
            #[cfg(target_arch = "wasm32")]
            let _ = &file;
            #[cfg(not(target_arch = "wasm32"))]
            let mut patch = String::new();
            #[cfg(target_arch = "wasm32")]
            let patch = String::new();

            if let Some(_path) = app.project_path.clone() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    patch = (|| {
                        let repo = git2::Repository::open(&_path).ok()?;
                        let head = repo.head().ok()?;
                        let tree = head.peel_to_tree().ok()?;
                        let mut opts = git2::DiffOptions::new();
                        opts.include_untracked(true);
                        opts.recurse_untracked_dirs(true);
                        opts.pathspec(file.clone());
                        let diff = repo
                            .diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))
                            .ok()?;
                        let mut out = String::new();
                        diff.print(git2::DiffFormat::Patch, |_d, _h, line| {
                            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
                            true
                        })
                        .ok()?;
                        Some(out)
                    })()
                    .unwrap_or_default();
                }
            }

            open_copy_modal_with_text(app, patch);
            Task::none()
        }
        GitMessage::OpenCopyModalWithText(text) => {
            open_copy_modal_with_text(app, text);
            Task::none()
        }
        GitMessage::OpenCopyModalFromPath(path) => {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            open_copy_modal_with_text(app, content);
            Task::none()
        }
        GitMessage::CloseCopyModal => {
            app.show_git_copy_modal = false;
            Task::none()
        }
        GitMessage::ToggleCopyModalColored(enabled) => {
            app.git_copy_modal_use_color = enabled;
            if enabled {
                let text = app.git_copy_modal_editor.text();
                if text_too_large_for_code_editor(&text) {
                    app.git_copy_modal_use_color = false;
                    return Task::none();
                }
                app.git_copy_modal_code_editor = iced_code_editor::CodeEditor::new(&text, "diff");
                let theme = app.effective_editor_theme();
                let font_size = app.current_font_size.clamp(10.0, 30.0);
                let line_height = app.current_line_height.clamp(10.0, 60.0);
                let language = app.current_language;
                configure_git_code_editor(
                    theme,
                    font_size,
                    line_height,
                    language,
                    &mut app.git_copy_modal_code_editor,
                );
            } else {
                let text = app.git_copy_modal_code_editor.content();
                app.git_copy_modal_editor = iced::widget::text_editor::Content::with_text(&text);
            }
            Task::none()
        }
        GitMessage::CopyModalEditorAction(action) => {
            app.git_copy_modal_editor.perform(action);
            Task::none()
        }
        GitMessage::CopyModalCodeEditorEvent(event) => app
            .git_copy_modal_code_editor
            .update(&event)
            .map(|next| Message::Git(GitMessage::CopyModalCodeEditorEvent(next))),
        GitMessage::CopyModalCopyCurrent => {
            let content = if app.git_copy_modal_use_color {
                app.git_copy_modal_code_editor.content()
            } else {
                app.git_copy_modal_editor.text()
            };
            Task::done(Message::CopyCode(content))
        }
        GitMessage::InsertCopyModalToChatCurrent => {
            let content = if app.git_copy_modal_use_color {
                app.git_copy_modal_code_editor.content()
            } else {
                app.git_copy_modal_editor.text()
            };
            app.show_git_copy_modal = false;
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(content)))
        }
        GitMessage::InsertCopyModalToChat(text) => {
            app.show_git_copy_modal = false;
            Task::done(Message::Chat(crate::app::message::ChatMessage::AppendText(text)))
        }
        _ => unreachable!("unexpected modal git message"),
    }
}
