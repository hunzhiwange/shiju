//! Host snapshot and cross-domain message mapping for Markdown rendering.
use super::markdown_links::WorkspaceLinks;
use crate::app::{App, Message, message};
pub(crate) use desktop_chat_view::message_view::markdown::*;
use iced::Element;
use std::rc::Rc;
pub(crate) fn markdown_body_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, Message> {
    use desktop_chat_view::message_view::markdown_body::{MarkdownView, WorkspaceLinks as Links};
    let links = WorkspaceLinks::new(app);
    let open = links.clone();
    let selected = app.chat_markdown_selection.as_ref().is_some_and(|s| {
        s.message_idx == msg_idx
            && s.text_idx == text_idx
            && app.active_session_id.as_deref() == Some(s.session_id.as_str())
    });
    let view = MarkdownView {
        theme: app.theme(),
        editors: &app.chat_markdown_block_editors,
        session_id: app.active_session_id.as_deref(),
        selected,
        links: Links {
            visible: Rc::new(move |target| links.visible_target(target)),
            open: Rc::new(move |target| open.open(target)),
        },
        editor_action: |msg, text, block, action| {
            Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(msg, text, block, action))
        },
        code_block: Rc::new(move |language, code, body| {
            super::markdown_code::code_block_view(app, language, code, body)
        }),
    };
    desktop_chat_view::message_view::markdown_body::markdown_body_view(
        &view, msg_idx, text_idx, blocks,
    )
}
