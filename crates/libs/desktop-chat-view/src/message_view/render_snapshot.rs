//! Borrowed message rendering inputs; tool and attachment ports retain their domain owners.
use super::{markdown::MarkdownBlock, markdown_body::WorkspaceLinks};
use crate::render_cache::ChatRenderCacheEntry;
use desktop_chat_tools::{
    tool_text_support::ToolTextState,
    tools::types::{CommandItem, ExploreItem},
};
use iced::{Element, Theme};
use session_ui_types::ChatMessage;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Debug, Clone)]
pub enum Action {
    CopyChatMessage {
        session_id: String,
        message_idx: usize,
    },
    OpenChatMessageContextMenu {
        target: u64,
        x: f32,
        y: f32,
        session_id: String,
        message_idx: usize,
    },
    Fork(usize),
    Reset(usize),
    ToggleUserMessage(usize),
    PermissionSelectRequest(String),
    Text(super::text::Message),
    Think(super::think_block::Message),
    MarkdownEditor(usize, usize, usize, iced::widget::text_editor::Action),
    CopyCode(String),
}
pub struct ToolViews<'a, M> {
    pub card: Rc<dyn Fn(usize, usize, &str) -> Option<Element<'a, M>> + 'a>,
    pub commands: Rc<dyn Fn(usize, &[CommandItem]) -> Option<Element<'a, M>> + 'a>,
    pub explore:
        Rc<dyn Fn(usize, usize, &[ExploreItem], bool, bool) -> Option<Element<'a, M>> + 'a>,
    pub editor: Rc<
        dyn Fn(
                desktop_chat_tools::tool_text_support::ToolTextTarget,
                bool,
            ) -> Option<Element<'a, M>>
            + 'a,
    >,
}
pub struct MessageViewSnapshot<'a, M> {
    pub chat: &'a [ChatMessage],
    pub chat_message_ids: &'a [Option<String>],
    pub chat_render_cache: &'a HashMap<usize, ChatRenderCacheEntry>,
    pub chat_visible_text_cache: &'a [Option<String>],
    pub chat_copy_hash_cache: &'a [Option<u64>],
    pub chat_message_expanded: &'a HashSet<usize>,
    pub chat_markdown_blocks: &'a HashMap<u64, Vec<MarkdownBlock>>,
    pub chat_think_expanded: &'a HashSet<u64>,
    pub chat_think_collapsed: &'a HashSet<u64>,
    pub chat_think_hovered_idx: Option<u64>,
    pub markdown_selection_text_idx: Option<usize>,
    pub text: ToolTextState<'a>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub dialogue_flow_show_reasoning_summary: bool,
    pub active_session_id: Option<&'a str>,
    pub permission_modal_requests: &'a [gateway_interaction::PendingPermissionRequestDto],
    pub permission_modal_request: Option<&'a gateway_interaction::PendingPermissionRequestDto>,
    pub permission_modal_request_id: Option<&'a str>,
    pub is_requesting: bool,
    pub live_processing_elapsed_ms: Option<u64>,
    pub recovery_text: Option<String>,
    pub localized_recovery_text: Option<String>,
    pub last_copied_code_hash: Option<u64>,
    pub copied_recently: bool,
    pub now_ms: u64,
    pub status_animation_frame: usize,
    pub links: WorkspaceLinks<'a, M>,
    pub emit: Rc<dyn Fn(Action) -> M + 'a>,
    pub tools: ToolViews<'a, M>,
    pub attachments: Rc<dyn Fn(&str) -> AttachmentView<'a, M> + 'a>,
}
pub struct AttachmentView<'a, M> {
    pub text: String,
    pub has_attachments: bool,
    pub preview: Rc<dyn Fn() -> Element<'a, M> + 'a>,
}
impl<M> MessageViewSnapshot<'_, M> {
    pub fn theme(&self) -> Theme {
        self.text.theme.clone()
    }
}
