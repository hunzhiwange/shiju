//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use crate::app::components::chat_panel::message_view::{
    build_render_cache_entry, deduped_tool_last_indices,
};
use crate::app::components::chat_panel::tools::{
    is_explore_tool, pending_permission_targets_message, pending_permission_targets_tool_call,
    tool_identity_from_raw, tool_name_from_raw,
};
use crate::app::{App, Message, models};
use iced::{
    Task,
    widget::{operation, scrollable},
};

/// 模块内可见函数，执行 close_input_context_menu 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn close_input_context_menu(app: &mut App) {
    app.input_context_menu_open = false;
    app.input_context_menu_pos = None;
}

/// 模块内可见函数，执行 sync_global_input_editor_if_needed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn sync_global_input_editor_if_needed(app: &mut App) {
    if app.active_session_id.is_none() {
        app.input_editor = app.current_session_runtime_mut().input_editor.clone();
    }
}

/// 模块内可见函数，执行 focus_input_editor 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn focus_input_editor(app: &App) -> Task<Message> {
    operation::focus(app.input_editor_id.clone())
}

/// 模块内可见函数，执行 preferred_chat_message_index_by_id 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn preferred_chat_message_index_by_id(
    chat: &[models::ChatMessage],
    message_ids: &[Option<String>],
    message_id: &str,
) -> Option<usize> {
    let mut first_match = None;

    for (idx, candidate) in message_ids.iter().enumerate() {
        if candidate.as_deref() != Some(message_id) {
            continue;
        }

        if first_match.is_none() {
            first_match = Some(idx);
        }

        if chat.get(idx).is_some_and(|message| message.role == models::ChatRole::Tool) {
            return Some(idx);
        }
    }

    first_match
}

/// 模块内可见函数，执行 permission_target_tool_anchor_fraction 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn permission_target_tool_anchor_fraction(
    message: &models::ChatMessage,
    request: Option<&vw_gateway_client::PendingPermissionRequestDto>,
    message_id: Option<&str>,
    show_reasoning_summary: bool,
) -> Option<f32> {
    if !pending_permission_targets_message(request, message_id) {
        return None;
    }

    if message.role == models::ChatRole::Tool {
        return Some(0.30);
    }

    if message.role != models::ChatRole::Assistant {
        return None;
    }

    let (_, visible_for_copy, _) = crate::app::ui::chat::split_think(&message.content);
    let render_cache =
        build_render_cache_entry(&message.content, &visible_for_copy, 0, show_reasoning_summary);
    let tool_last = deduped_tool_last_indices(&render_cache.blocks);
    let mut visible_tools: Vec<Vec<usize>> = Vec::new();
    let mut pending_explore_group: Vec<usize> = Vec::new();

    for (block_idx, block) in render_cache.blocks.iter().enumerate() {
        let models::ParsedChatBlock::Tool { raw } = block else {
            if !pending_explore_group.is_empty() {
                visible_tools.push(std::mem::take(&mut pending_explore_group));
            }
            continue;
        };

        let Some(tool_name) = tool_name_from_raw(raw) else {
            if !pending_explore_group.is_empty() {
                visible_tools.push(std::mem::take(&mut pending_explore_group));
            }
            continue;
        };
        if is_explore_tool(&tool_name) {
            pending_explore_group.push(block_idx);
            continue;
        }

        if !pending_explore_group.is_empty() {
            visible_tools.push(std::mem::take(&mut pending_explore_group));
        }
        if let Some(identity) = tool_identity_from_raw(raw)
            && tool_last.get(&identity).copied() != Some(block_idx)
        {
            continue;
        }

        visible_tools.push(vec![block_idx]);
    }

    if !pending_explore_group.is_empty() {
        visible_tools.push(pending_explore_group);
    }

    if visible_tools.is_empty() {
        return Some(0.50);
    }

    let target_tool_idx = visible_tools.iter().position(|group| {
        group.iter().any(|block_idx| {
            let Some(models::ParsedChatBlock::Tool { raw }) = render_cache.blocks.get(*block_idx)
            else {
                return false;
            };
            pending_permission_targets_tool_call(request, message_id, raw)
        })
    })?;
    let step = 1.0 / (visible_tools.len() as f32 + 1.0);
    Some((step * (target_tool_idx as f32 + 1.0)).clamp(0.22, 0.78))
}

fn locate_chat_message_idx(
    app: &mut App,
    target_idx: usize,
    message_id: Option<&str>,
) -> Task<Message> {
    if target_idx >= app.chat.len() {
        return Task::none();
    }

    app.chat_auto_scroll = false;
    app.sync_chat_message_estimated_heights_len();
    app.refine_chat_message_estimated_heights(target_idx, (target_idx + 1).min(app.chat.len()));

    let viewport_h = app.chat_scroll_viewport_h.max(0.0);
    let target_anchor_fraction = app
        .chat
        .get(target_idx)
        .and_then(|message| {
            permission_target_tool_anchor_fraction(
                message,
                app.permission_modal_request.as_ref(),
                message_id,
                app.dialogue_flow_show_reasoning_summary,
            )
        })
        .unwrap_or(0.50);
    app.chat_virtualizer.set_viewport_height(viewport_h);
    app.chat_virtualizer.scroll_to_item_anchor(target_idx, target_anchor_fraction, 0.18);
    app.sync_chat_virtualizer_compat_fields();

    let target_chunk_start = crate::app::session::chat_ui_chunk_start_idx(target_idx);
    let prewarm_task =
        if !app.active_session_view_state.preparing_chat_ui_chunks.contains(&target_chunk_start) {
            if let Some(session_id) = app.active_session_id.clone() {
                app.mark_chat_ui_chunks_preparing(&[target_chunk_start]);
                crate::app::message::project::prepare_session_ui_task(
                    session_id,
                    app.active_shared_chat_messages(),
                    target_chunk_start,
                    false,
                    app.dialogue_flow_show_reasoning_summary,
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
            app.chat_scroll_id.clone(),
            scrollable::RelativeOffset { x: Some(0.0), y: Some(app.chat_scroll_offset_y) },
        )
        .map(|_: ()| Message::CopyDone),
    ])
}

/// 模块内可见函数，执行 locate_chat_message 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn locate_chat_message(app: &mut App, message_id: &str) -> Task<Message> {
    let Some(target_idx) =
        preferred_chat_message_index_by_id(&app.chat, &app.chat_message_ids, message_id)
    else {
        return Task::none();
    };

    locate_chat_message_idx(app, target_idx, Some(message_id))
}

/// 模块内可见函数，执行 locate_chat_message_index 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn locate_chat_message_index(app: &mut App, target_idx: usize) -> Task<Message> {
    let message_id = app.chat_message_ids.get(target_idx).cloned().flatten();
    locate_chat_message_idx(app, target_idx, message_id.as_deref())
}
