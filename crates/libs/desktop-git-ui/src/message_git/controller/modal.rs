//! 本领域完整消息分支；宿主只执行返回的跨域和系统效果。
use super::Context;
use super::Effect;
use super::GitMessage;
use super::State;
use super::Task;
use super::dismiss;

fn open_copy_modal_with_text(app: &mut State, ctx: &Context, text: String) {
    let theme = ctx.editor_theme.clone();
    crate::message_git::modal::open_copy_modal_with_text(
        &mut app.show_git_copy_modal,
        &mut app.git_copy_modal_use_color,
        &mut app.git_copy_modal_editor,
        &mut app.git_copy_modal_code_editor,
        text,
        theme,
        ctx.font_size,
        ctx.line_height,
        ctx.language.clone(),
    );
}

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub fn update(
    app: &mut State,
    ctx: &Context,
    effects: &mut Vec<Effect>,
    message: GitMessage,
) -> Task<GitMessage> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::OpenCustomDiffModal => {
            crate::message_git::modal::open_custom_diff_modal(
                &mut app.show_git_custom_diff_modal,
                &mut app.git_custom_diff_hide_inputs,
                &mut app.git_custom_diff_title,
            );
            Task::none()
        }
        GitMessage::CloseCustomDiffModal => {
            crate::message_git::modal::close_custom_diff_modal(
                &mut app.show_git_custom_diff_modal,
                &mut app.git_custom_diff_hide_inputs,
            );
            Task::none()
        }
        GitMessage::CustomDiffTitleChanged(title) => {
            crate::message_git::modal::custom_diff_title_changed(
                &mut app.git_custom_diff_title,
                title,
            );
            Task::none()
        }
        GitMessage::CustomDiffBeforeEditorAction(action) => {
            crate::message_git::modal::custom_diff_before_editor_action(
                &mut app.git_custom_diff_before_editor,
                action,
            );
            Task::none()
        }
        GitMessage::CustomDiffAfterEditorAction(action) => {
            crate::message_git::modal::custom_diff_after_editor_action(
                &mut app.git_custom_diff_after_editor,
                action,
            );
            Task::none()
        }
        GitMessage::CustomDiffSwap => {
            crate::message_git::modal::custom_diff_swap(
                &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor,
            );
            Task::none()
        }
        GitMessage::OpenCustomDiffResult { title, before, after } => {
            dismiss(app, effects);
            crate::message_git::modal::open_custom_diff_result(
                &mut app.show_git_custom_diff_modal,
                &mut app.git_custom_diff_hide_inputs,
                &mut app.git_custom_diff_title,
                &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor,
                title,
                before,
                after,
            );
            effects.push(Effect::ClearPreview);
            effects.push(Effect::ShowDiff { changes: false });
            Task::none()
        }
        GitMessage::OpenChatTextDiff { title, file: _, before, after } => {
            // 与 OpenCustomDiffResult 共用自定义 Diff 弹窗，避免在面板中再插一块独立消息框。
            dismiss(app, effects);
            app.chat_text_diff = None;
            crate::message_git::modal::open_custom_diff_result(
                &mut app.show_git_custom_diff_modal,
                &mut app.git_custom_diff_hide_inputs,
                &mut app.git_custom_diff_title,
                &mut app.git_custom_diff_before_editor,
                &mut app.git_custom_diff_after_editor,
                title,
                before,
                after,
            );
            effects.push(Effect::ClearPreview);
            effects.push(Effect::ShowDiff { changes: false });
            effects.push(Effect::ShowDiff { changes: true });
            Task::none()
        }
        GitMessage::CloseChatTextDiff => {
            crate::message_git::modal::close_chat_text_diff(
                &mut app.chat_text_diff,
                &mut app.show_git_custom_diff_modal,
                &mut app.git_custom_diff_hide_inputs,
            );
            Task::none()
        }
        GitMessage::OpenDiffCopyMode(file) => {
            let patch = crate::message_git::modal::diff_patch(ctx.project_path.as_deref(), file);
            open_copy_modal_with_text(app, ctx, patch);
            Task::none()
        }
        GitMessage::OpenCopyModalWithText(text) => {
            open_copy_modal_with_text(app, ctx, text);
            Task::none()
        }
        GitMessage::OpenCopyModalFromPath(path) => {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            open_copy_modal_with_text(app, ctx, content);
            Task::none()
        }
        GitMessage::CloseCopyModal => {
            crate::message_git::modal::close_copy_modal(&mut app.show_git_copy_modal);
            Task::none()
        }
        GitMessage::ToggleCopyModalColored(enabled) => {
            let theme = ctx.editor_theme.clone();
            crate::message_git::modal::toggle_copy_modal_colored(
                &mut app.git_copy_modal_use_color,
                &mut app.git_copy_modal_editor,
                &mut app.git_copy_modal_code_editor,
                enabled,
                theme,
                ctx.font_size,
                ctx.line_height,
                ctx.language.clone(),
            );
            Task::none()
        }
        GitMessage::CopyModalEditorAction(action) => {
            crate::message_git::modal::copy_modal_editor_action(
                &mut app.git_copy_modal_editor,
                action,
            );
            Task::none()
        }
        GitMessage::CopyModalCodeEditorEvent(event) => app
            .git
            .git_copy_modal_code_editor
            .update(&event)
            .map(|next| GitMessage::CopyModalCodeEditorEvent(next)),
        GitMessage::CopyModalCopyCurrent => {
            let content = crate::message_git::modal::current_copy_content(
                app.git_copy_modal_use_color,
                &app.git_copy_modal_editor,
                &app.git_copy_modal_code_editor,
            );
            {
                effects.push(Effect::CopyCode(content));
                Task::none()
            }
        }
        GitMessage::InsertCopyModalToChatCurrent => {
            let content = crate::message_git::modal::current_copy_content(
                app.git_copy_modal_use_color,
                &app.git_copy_modal_editor,
                &app.git_copy_modal_code_editor,
            );
            app.show_git_copy_modal = false;
            {
                effects.push(Effect::AppendChat(content));
                Task::none()
            }
        }
        GitMessage::InsertCopyModalToChat(text) => {
            app.show_git_copy_modal = false;
            {
                effects.push(Effect::AppendChat(text));
                Task::none()
            }
        }
        _ => unreachable!("unexpected modal git message"),
    }
}
