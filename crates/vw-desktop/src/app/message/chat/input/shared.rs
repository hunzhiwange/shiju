//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use crate::app::components::chat_panel::message_view::{
    build_render_cache_entry, deduped_tool_last_indices,
};
use crate::app::{App, Message, models};
use iced::{
    Task,
    widget::{operation, scrollable},
};

/// 模块内可见函数，执行 close_input_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn close_input_context_menu(app: &mut App) {
    app.session_state.input_context_menu.close();
}

/// 模块内可见函数，执行 sync_global_input_editor_if_needed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn sync_global_input_editor_if_needed(app: &mut App) {
    if app.session_state.active_session_id.is_none() {
        app.session_state.input_editor = app.current_session_runtime_mut().input_editor.clone();
    }
}

/// 模块内可见函数，执行 focus_input_editor 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn focus_input_editor(app: &App) -> Task<Message> {
    operation::focus(app.session_state.input_editor_id.clone())
}

/// 模块内可见函数，执行 permission_target_tool_anchor_fraction 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn permission_target_tool_anchor_fraction(
    message: &models::ChatMessage,
    request: Option<&gateway_interaction::PendingPermissionRequestDto>,
    message_id: Option<&str>,
    show_reasoning_summary: bool,
) -> Option<f32> {
 let (_, visible_for_copy, _) = crate::app::ui::chat::split_think(&message.content);
 let render_cache = build_render_cache_entry(&message.content, &visible_for_copy, 0, show_reasoning_summary);
 let tool_last = deduped_tool_last_indices(&render_cache.blocks);
 desktop_chat_view::permission_anchor::permission_target_tool_anchor_fraction(message, request, message_id, &render_cache.blocks, &tool_last)
}

fn locate_chat_message_idx(
    app: &mut App,
    target_idx: usize,
    message_id: Option<&str>,
) -> Task<Message> {
    if target_idx >= app.chat_view.chat.len() {
        return Task::none();
    }

    app.chat_view.chat_auto_scroll = false;
    app.sync_chat_message_estimated_heights_len();
    app.refine_chat_message_estimated_heights(target_idx, (target_idx + 1).min(app.chat_view.chat.len()));

    let viewport_h = app.chat_view.chat_scroll_viewport_h.max(0.0);
    let target_anchor_fraction = app.chat_view.chat
        .get(target_idx)
        .and_then(|message| {
            permission_target_tool_anchor_fraction(
                message,
                app.session_state.permission_modal_request.as_ref(),
                message_id,
                app.general_settings.dialogue_flow_show_reasoning_summary,
            )
        })
        .unwrap_or(0.50);
    app.chat_view.chat_virtualizer.set_viewport_height(viewport_h);
    app.chat_view.chat_virtualizer.scroll_to_item_anchor(target_idx, target_anchor_fraction, 0.18);
    app.sync_chat_virtualizer_compat_fields();

    let target_chunk_start = crate::app::session::chat_ui_chunk_start_idx(target_idx);
    let prewarm_task =
        if !app.chat_view.active_session_view_state.preparing_chat_ui_chunks.contains(&target_chunk_start) {
            if let Some(session_id) = app.session_state.active_session_id.clone() {
                app.mark_chat_ui_chunks_preparing(&[target_chunk_start]);
                crate::app::message::project::prepare_session_ui_task(
                    session_id,
                    app.active_shared_chat_messages(),
                    target_chunk_start,
                    false,
                    app.general_settings.dialogue_flow_show_reasoning_summary,
                )
            } else {
                Task::none()
            }
        } else {
            Task::none()
        };

    Task::batch(vec![
        prewarm_task,
        operation::snap_to(
            app.chat_view.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(app.chat_view.chat_scroll_offset_y) },
        )
        .map(|_: ()| Message::CopyDone),
    ])
}

/// 模块内可见函数，执行 locate_chat_message 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn locate_chat_message(app: &mut App, message_id: &str) -> Task<Message> {
    let Some(target_idx) =
        preferred_chat_message_index_by_id(&app.chat_view.chat, &app.chat_view.chat_message_ids, message_id)
    else {
        return Task::none();
    };

    locate_chat_message_idx(app, target_idx, Some(message_id))
}

/// 模块内可见函数，执行 locate_chat_message_index 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn locate_chat_message_index(app: &mut App, target_idx: usize) -> Task<Message> {
    let message_id = app.chat_view.chat_message_ids.get(target_idx).cloned().flatten();
    locate_chat_message_idx(app, target_idx, message_id.as_deref())
}

pub(super) use desktop_session::message::chat::input::shared::preferred_chat_message_index_by_id;
