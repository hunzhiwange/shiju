//! Assemble borrowed domain inputs and map cross-domain UI effects.
use crate::app::{App, Message, message};
use desktop_chat_view::message_view::{
    markdown_body::WorkspaceLinks,
    render_snapshot::{Action, MessageViewSnapshot, ToolViews},
};
use std::rc::Rc;
pub(super) fn snapshot(app: &App) -> MessageViewSnapshot<'_, Message> {
    let links = super::markdown_links::WorkspaceLinks::new(app);
    let open = links.clone();
    let runtime = app.current_session_runtime_ref();
    let recovery = runtime.and_then(|r| r.model_recovery.as_ref()).map(|(_, event)| event);
    MessageViewSnapshot {
        chat: &app.chat_view.chat,
        chat_message_ids: &app.chat_view.chat_message_ids,
        chat_render_cache: &app.chat_view.chat_render_cache,
        chat_visible_text_cache: &app.chat_view.chat_visible_text_cache,
        chat_copy_hash_cache: &app.chat_view.chat_copy_hash_cache,
        chat_message_expanded: &app.chat_view.chat_message_expanded,
        chat_markdown_blocks: &app.chat_view.chat_markdown_blocks,
        chat_think_expanded: &app.chat_view.chat_think_expanded,
        chat_think_collapsed: &app.chat_view.chat_think_collapsed,
        chat_think_hovered_idx: app.chat_interaction.chat_think_hovered_idx,
        text: super::super::tool_text_support::text_state(app),
        markdown_selection_text_idx: app.chat_interaction.chat_markdown_selection
            .as_ref()
            .filter(|s| app.session_state.active_session_id.as_deref() == Some(s.session_id.as_str()))
            .map(|s| s.text_idx),
        desktop_locale: app.general_settings.desktop_locale,
        dialogue_flow_show_reasoning_summary: app.general_settings.dialogue_flow_show_reasoning_summary,
        active_session_id: app.session_state.active_session_id.as_deref(),
        permission_modal_requests: &app.session_state.permission_modal_requests,
        permission_modal_request: app.session_state.permission_modal_request.as_ref(),
        permission_modal_request_id: app.session_state.permission_modal_request_id.as_deref(),
        is_requesting: runtime.is_some_and(|r| r.request_lifecycle.is_requesting),
        live_processing_elapsed_ms: runtime
            .map(|r| r.request_lifecycle.processing_timer.elapsed_ms_at(web_time::Instant::now())),
        recovery_text: recovery.map(super::model_recovery_text),
        localized_recovery_text: recovery.map(|e| {
            super::localized_model_recovery_text(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), e)
        }),
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
        copied_recently: app.clipboard_state.last_copied_code_hash
            .is_some_and(|h| super::super::utils::is_recent_copy(app, h)),
        now_ms: crate::app::time::now_ms(),
        status_animation_frame: app.status_animation_frame,
        links: WorkspaceLinks {
            visible: Rc::new(move |target| links.visible_target(target)),
            open: Rc::new(move |target| open.open(target)),
        },
        emit: Rc::new(move |event| match event {
            Action::CopyChatMessage { session_id, message_idx } => {
                Message::CopyChatMessage { session_id, message_idx }
            }
            Action::OpenChatMessageContextMenu { target, x, y, session_id, message_idx } => {
                Message::Chat(message::ChatMessage::OpenChatMessageContextMenu {
                    target,
                    x,
                    y,
                    session_id,
                    message_idx,
                })
            }
            Action::Fork(idx) => Message::Chat(message::ChatMessage::OpenForkSessionDialog(
                crate::app::message::chat::fork_session_anchor_from_user_row(app, idx),
            )),
            Action::Reset(idx) => Message::Chat(message::ChatMessage::ToggleResetMenu(idx)),
            Action::ToggleUserMessage(idx) => {
                Message::Chat(message::ChatMessage::ToggleUserMessage(idx))
            }
            Action::PermissionSelectRequest(id) => {
                Message::Chat(message::ChatMessage::PermissionSelectRequest(id))
            }
            Action::Text(event) => map_text(event),
            Action::Think(event) => map_think(event),
            Action::MarkdownEditor(m, t, b, a) => {
                Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(m, t, b, a))
            }
            Action::CopyCode(text) => Message::CopyCode(text),
        }),
        tools: ToolViews {
            card: Rc::new(move |m, t, raw| {
                super::super::tools::render_shared_tool_view(app, m, t, raw)
            }),
            commands: Rc::new(move |m, items| {
                super::super::tools::tool_command_summary_view(app, m, items)
            }),
            explore: Rc::new(move |m, g, items, r, c| {
                super::super::tools::tool_explore_summary_view(app, m, g, items, r, c)
            }),
            editor: Rc::new(move |target, danger| {
                super::super::tools::tool_text_editor(
                    app,
                    target,
                    super::super::tool_text_support::chat_text_font_name(),
                    desktop_chat_view::message_view::styles::MESSAGE_TEXT_SIZE,
                    false,
                    danger,
                )
            }),
        },
        attachments: Rc::new(move |raw| {
            let (text, items) =
                crate::app::components::input_panel::attachment::parse_attachment_markers(raw);
            desktop_chat_view::message_view::render_snapshot::AttachmentView {
                text: text.into_owned(),
                has_attachments: !items.is_empty(),
                preview: Rc::new(move || {
                    crate::app::components::input_panel::attachment::attachment_preview_strip(
                        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
                        items.clone(),
                    )
                }),
            }
        }),
    }
}

fn map_text(message: desktop_chat_view::message_view::text::Message) -> Message {
    use desktop_chat_view::message_view::text::Message as Local;
    Message::Chat(match message {
        Local::MessageEditorAction(idx, action) => {
            message::ChatMessage::MessageEditorAction(idx, action)
        }
        Local::ThinkEditorAction(msg, idx, action) => {
            message::ChatMessage::ThinkEditorAction(msg, idx, action)
        }
    })
}

fn map_think(event: desktop_chat_view::message_view::think_block::Message) -> Message {
    use desktop_chat_view::message_view::think_block as local;
    Message::Chat(match event {
        local::Message::ThinkHover(m, i) => message::ChatMessage::ThinkHover(m, i),
        local::Message::ThinkHoverLeave => message::ChatMessage::ThinkHoverLeave,
        local::Message::ToggleThink(m, i, d) => message::ChatMessage::ToggleThink(m, i, d),
        local::Message::ThinkEditorAction(m, i, a) => {
            message::ChatMessage::ThinkEditorAction(m, i, a)
        }
    })
}
