//! Local rendering fixture. Native clipboard effects are recorded at their boundary.
pub use crate::message_view::workspace_links::LinkEffect as ViewMessage;
use crate::message_view::{
    markdown_selection::SelectionMessage,
    render_snapshot::{Action, AttachmentView, MessageViewSnapshot, ToolViews},
};
use crate::{chat_ui::ChatMarkdownSelection, render_cache::ChatRenderCacheEntry};
use iced::widget::text_editor;
use session_ui_types::ChatMessage;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Debug, Clone)]
pub enum ChatMessageEvent {
    OpenChatMessageContextMenu {
        target: u64,
        x: f32,
        y: f32,
        session_id: String,
        message_idx: usize,
    },
}
pub mod message {
    pub use super::ChatMessageEvent as ChatMessage;
}
#[derive(Debug, Clone)]
pub enum Message {
    None,
    CopyCode(String),
    CopyFeedbackExpired(u64),
    CopyChatMessage {
        session_id: String,
        message_idx: usize,
    },
    Chat(ChatMessageEvent),
    View(ViewMessage),
    Local(Action),
    ChatMarkdownSelectionChanged {
        session_id: String,
        message_idx: usize,
        text_idx: usize,
        text: Option<String>,
    },
}
impl SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(_: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        Self::None
    }
    fn selection(
        session_id: String,
        message_idx: usize,
        text_idx: usize,
        text: Option<String>,
    ) -> Self {
        Self::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text }
    }
}
pub fn action(event: Action) -> Message {
    match event {
        Action::CopyChatMessage { session_id, message_idx } => {
            Message::CopyChatMessage { session_id, message_idx }
        }
        Action::OpenChatMessageContextMenu { target, x, y, session_id, message_idx } => {
            Message::Chat(ChatMessageEvent::OpenChatMessageContextMenu {
                target,
                x,
                y,
                session_id,
                message_idx,
            })
        }
        Action::CopyCode(text) => Message::CopyCode(text),
        other => Message::Local(other),
    }
}
pub struct RenderFixture {
    pub chat: Vec<session_ui_types::ChatMessage>,
    pub chat_copy_hash_cache: Vec<Option<u64>>,
    pub chat_explore_expanded: HashSet<u64>,
    pub chat_explore_summary_animations:
        HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    pub chat_height_index: crate::height_index::ChatHeightIndex,
    pub chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
    pub chat_markdown_blocks: HashMap<u64, Vec<crate::message_view::markdown::MarkdownBlock>>,
    pub chat_markdown_selection: Option<ChatMarkdownSelection>,
    pub chat_message_editors: Vec<text_editor::Content>,
    pub chat_message_estimated_heights: Vec<f32>,
    pub chat_message_expanded: HashSet<usize>,
    pub chat_message_measured_heights: HashMap<usize, f32>,
    pub chat_render_cache: HashMap<usize, ChatRenderCacheEntry>,
    pub chat_reset_menu_idx: Option<usize>,
    pub chat_scroll_offset_y: f32,
    pub chat_scroll_viewport_h: f32,
    pub chat_special_text_editors: HashMap<u64, text_editor::Content>,
    pub chat_think_collapsed: HashSet<u64>,
    pub chat_think_editors: HashMap<u64, text_editor::Content>,
    pub chat_think_expanded: HashSet<u64>,
    pub chat_think_hovered_idx: Option<u64>,
    pub chat_think_scroll_ids: HashMap<u64, iced::widget::Id>,
    pub chat_tool_expanded: HashSet<u64>,
    pub chat_tool_file_expanded: HashSet<String>,
    pub chat_tool_file_hovered: Option<String>,
    pub chat_tool_hovered_idx: Option<u64>,
    pub chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    pub chat_virtualizer: crate::virtualizer::ChatVirtualizer,
    pub chat_visible_text_cache: Vec<Option<String>>,
    pub tool_detail_dialog:
        Option<desktop_session::message::chat::input::tool_detail::ToolDetailDialog>,
    pub message_meta_texts: Vec<Option<desktop_i18n::chat::MessageMeta>>,
    pub user_question_markers: Vec<desktop_session::session::active::UserQuestionMarker>,
    pub user_question_indexed_chat_len: usize,
    pub prepared_chat_ui_chunks: HashSet<usize>,
    pub preparing_chat_ui_chunks: HashSet<usize>,
    pub dirty_preparing_chat_ui_chunks: HashSet<usize>,
    pub last_visited_chat_ui_chunk_start: Option<usize>,
    pub pinned_chat_ui_chunk_start: Option<usize>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub dialogue_flow_show_reasoning_summary: bool,
    pub is_requesting: bool,
    pub window_width: f32,
    pub is_dark: bool,
    pub chat_message_ids: Vec<Option<String>>,
    pub last_copied_code_hash: Option<u64>,
    pub active_session_id: Option<String>,
    pub permission_modal_requests: Vec<gateway_interaction::PendingPermissionRequestDto>,
    pub permission_modal_request: Option<gateway_interaction::PendingPermissionRequestDto>,
    pub permission_modal_request_id: Option<String>,
    pub app_theme: iced::Theme,
    pub status_animation_frame: usize,
    pub project_path: Option<String>,
    pub file_tree_model_cache: HashMap<String, desktop_file_tree::model::FileTreeNode>,
}
impl RenderFixture {
    pub fn new() -> (Self, ()) {
        (
            Self {
                chat: Default::default(),
                chat_copy_hash_cache: Default::default(),
                chat_explore_expanded: Default::default(),
                chat_explore_summary_animations: Default::default(),
                chat_height_index: Default::default(),
                chat_markdown_block_editors: Default::default(),
                chat_markdown_blocks: Default::default(),
                chat_markdown_selection: Default::default(),
                chat_message_editors: Default::default(),
                chat_message_estimated_heights: Default::default(),
                chat_message_expanded: Default::default(),
                chat_message_measured_heights: Default::default(),
                chat_render_cache: Default::default(),
                chat_reset_menu_idx: Default::default(),
                chat_scroll_offset_y: Default::default(),
                chat_scroll_viewport_h: Default::default(),
                chat_special_text_editors: Default::default(),
                chat_think_collapsed: Default::default(),
                chat_think_editors: Default::default(),
                chat_think_expanded: Default::default(),
                chat_think_hovered_idx: Default::default(),
                chat_think_scroll_ids: Default::default(),
                chat_tool_expanded: Default::default(),
                chat_tool_file_expanded: Default::default(),
                chat_tool_file_hovered: Default::default(),
                chat_tool_hovered_idx: Default::default(),
                chat_tool_text_editors: Default::default(),
                chat_virtualizer: Default::default(),
                chat_visible_text_cache: Default::default(),
                tool_detail_dialog: Default::default(),
                message_meta_texts: Default::default(),
                user_question_markers: Default::default(),
                user_question_indexed_chat_len: Default::default(),
                prepared_chat_ui_chunks: Default::default(),
                preparing_chat_ui_chunks: Default::default(),
                dirty_preparing_chat_ui_chunks: Default::default(),
                last_visited_chat_ui_chunk_start: Default::default(),
                pinned_chat_ui_chunk_start: Default::default(),
                desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                dialogue_flow_show_reasoning_summary: Default::default(),
                is_requesting: Default::default(),
                window_width: Default::default(),
                is_dark: false,
                chat_message_ids: Default::default(),
                last_copied_code_hash: Default::default(),
                active_session_id: Default::default(),
                permission_modal_requests: Default::default(),
                permission_modal_request: Default::default(),
                permission_modal_request_id: Default::default(),
                app_theme: iced::Theme::Light,
                status_animation_frame: Default::default(),
                project_path: Default::default(),
                file_tree_model_cache: Default::default(),
            },
            (),
        )
    }
    pub fn theme(&self) -> iced::Theme {
        self.app_theme.clone()
    }
    pub fn set_file_index(&mut self, root: &str, files: Vec<String>) {
        self.file_tree_model_cache
            .insert(root.into(), desktop_file_tree::model::build_file_tree_model(root, &files));
    }
    pub fn text_state(&self) -> desktop_chat_tools::tool_text_support::ToolTextState<'_> {
        desktop_chat_tools::tool_text_support::ToolTextState {
            chat_message_editors: &self.chat_message_editors,
            chat_special_text_editors: &self.chat_special_text_editors,
            chat_tool_text_editors: &self.chat_tool_text_editors,
            chat_think_editors: &self.chat_think_editors,
            chat_markdown_block_editors: &self.chat_markdown_block_editors,
            markdown_selection: self
                .chat_markdown_selection
                .as_ref()
                .filter(|s| self.active_session_id.as_deref() == Some(s.session_id.as_str()))
                .map(|s| (s.message_idx, s.text.as_str())),
            theme: self.theme(),
        }
    }
    pub fn card_input(&self) -> desktop_chat_tools::card_input::CardInput<'_> {
        desktop_chat_tools::card_input::CardInput {
            i18n: desktop_i18n::I18n::new(self.desktop_locale),
            hovered: self.chat_tool_hovered_idx,
            expanded: &self.chat_tool_expanded,
            file_expanded: &self.chat_tool_file_expanded,
            project_path: self.project_path.as_deref(),
            text: self.text_state(),
        }
    }
    pub fn workspace_links(&self) -> crate::message_view::workspace_links::WorkspaceLinks<'_> {
        let directory = self.project_path.as_deref();
        crate::message_view::workspace_links::WorkspaceLinks::new(
            directory.map(std::borrow::Cow::Borrowed),
            directory
                .and_then(|root| self.file_tree_model_cache.get_key_value(root))
                .map(|(root, tree)| (root.as_str(), tree)),
        )
    }
    pub fn snapshot(&self) -> MessageViewSnapshot<'_, Message> {
        let links = self.workspace_links();
        let open = links.clone();
        MessageViewSnapshot {
            chat: &self.chat,
            chat_message_ids: &self.chat_message_ids,
            chat_render_cache: &self.chat_render_cache,
            chat_visible_text_cache: &self.chat_visible_text_cache,
            chat_copy_hash_cache: &self.chat_copy_hash_cache,
            chat_message_expanded: &self.chat_message_expanded,
            chat_markdown_blocks: &self.chat_markdown_blocks,
            chat_think_expanded: &self.chat_think_expanded,
            chat_think_collapsed: &self.chat_think_collapsed,
            permission_modal_requests: &self.permission_modal_requests,
            chat_think_hovered_idx: self.chat_think_hovered_idx,
            text: self.text_state(),
            desktop_locale: self.desktop_locale,
            dialogue_flow_show_reasoning_summary: self.dialogue_flow_show_reasoning_summary,
            active_session_id: self.active_session_id.as_deref(),
            permission_modal_request: self.permission_modal_request.as_ref(),
            permission_modal_request_id: self.permission_modal_request_id.as_deref(),
            is_requesting: self.is_requesting,
            live_processing_elapsed_ms: None,
            recovery_text: None,
            localized_recovery_text: None,
            last_copied_code_hash: self.last_copied_code_hash,
            copied_recently: self.last_copied_code_hash.is_some(),
            now_ms: 1000,
            status_animation_frame: self.status_animation_frame,
            markdown_selection_text_idx: self.chat_markdown_selection.as_ref().map(|s| s.text_idx),
            links: crate::message_view::markdown_body::WorkspaceLinks {
                visible: Rc::new(move |t| links.visible_target(t)),
                open: Rc::new(move |t| Message::View(open.open(t))),
            },
            emit: Rc::new(action),
            tools: ToolViews {
                card: Rc::new(move |m, t, raw| {
                    use desktop_chat_tools::tools::{
                        bash_view::tool_bash_view,
                        read_view::tool_read_view,
                        text_view::tool_text_view,
                        tool_renderer::{SharedToolRenderKind, shared_tool_render_kind},
                    };
                    let input = self.card_input();
                    match shared_tool_render_kind(raw)? {
                        SharedToolRenderKind::Bash => tool_bash_view(&input, m, t, raw)
                            .or_else(|| tool_text_view(&input, m, t, raw)),
                        SharedToolRenderKind::Read => tool_read_view(&input, m, t, raw)
                            .or_else(|| tool_text_view(&input, m, t, raw)),
                        _ => tool_text_view(&input, m, t, raw),
                    }
                    .map(|v| v.map(|_| Message::None))
                }),
                commands: Rc::new(move |m, items| {
                    desktop_chat_tools::tools::command_summary_view::tool_command_summary_view(
                        &self.card_input(),
                        m,
                        items,
                    )
                    .map(|v| v.map(|_| Message::None))
                }),
                explore: Rc::new(move |m, g, items, r, c| {
                    desktop_chat_tools::tools::explore_summary_view::tool_explore_summary_view(
                        &desktop_chat_tools::tools::explore_input::ExploreInput {
                            i18n: desktop_i18n::I18n::new(self.desktop_locale),
                            hovered: self.chat_tool_hovered_idx,
                            chat_explore_expanded: &self.chat_explore_expanded,
                            chat_explore_summary_animations: &self.chat_explore_summary_animations,
                            text: self.text_state(),
                            now_ms: 1000,
                            status_animation_frame: self.status_animation_frame,
                        },
                        m,
                        g,
                        items,
                        r,
                        c,
                    )
                    .map(|v| v.map(|_| Message::None))
                }),
                editor: Rc::new(move |target, danger| {
                    desktop_chat_tools::tool_text_support::tool_text_editor(
                        &self.text_state(),
                        target,
                        desktop_chat_tools::tool_text_support::chat_text_font_name(),
                        crate::message_view::styles::MESSAGE_TEXT_SIZE,
                        false,
                        danger,
                    )
                    .map(|v| v.map(|_| Message::None))
                }),
            },
            attachments: Rc::new(|raw| AttachmentView {
                text: raw.to_owned(),
                has_attachments: false,
                preview: Rc::new(|| iced::widget::Space::new().into()),
            }),
        }
    }
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text } => {
                crate::chat_ui::update_markdown_selection(
                    &mut self.chat_markdown_selection,
                    self.active_session_id.as_deref(),
                    session_id,
                    message_idx,
                    text_idx,
                    text,
                )
            }
            Message::CopyCode(text) => {
                self.last_copied_code_hash =
                    Some(crate::message_view::parse::hash_chat_content(&text))
            }
            Message::CopyFeedbackExpired(hash) if self.last_copied_code_hash == Some(hash) => {
                self.last_copied_code_hash = None
            }
            _ => {}
        };
        iced::Task::none()
    }
    pub fn sync_completed_stream_text(&mut self, idx: usize) {
        self.chat_ui().sync_completed_stream_text(idx)
    }
    pub(crate) fn chat_ui(&mut self) -> crate::chat_ui::ChatUi<'_> {
        crate::chat_ui::ChatUi {
            chat: &self.chat,
            chat_copy_hash_cache: &mut self.chat_copy_hash_cache,
            chat_explore_expanded: &mut self.chat_explore_expanded,
            chat_explore_summary_animations: &mut self.chat_explore_summary_animations,
            chat_height_index: &mut self.chat_height_index,
            chat_markdown_block_editors: &mut self.chat_markdown_block_editors,
            chat_markdown_blocks: &mut self.chat_markdown_blocks,
            chat_markdown_selection: &mut self.chat_markdown_selection,
            chat_message_editors: &mut self.chat_message_editors,
            chat_message_estimated_heights: &mut self.chat_message_estimated_heights,
            chat_message_expanded: &mut self.chat_message_expanded,
            chat_message_measured_heights: &mut self.chat_message_measured_heights,
            chat_render_cache: &mut self.chat_render_cache,
            chat_reset_menu_idx: &mut self.chat_reset_menu_idx,
            chat_scroll_offset_y: &mut self.chat_scroll_offset_y,
            chat_scroll_viewport_h: &mut self.chat_scroll_viewport_h,
            chat_special_text_editors: &mut self.chat_special_text_editors,
            chat_think_collapsed: &mut self.chat_think_collapsed,
            chat_think_editors: &mut self.chat_think_editors,
            chat_think_expanded: &mut self.chat_think_expanded,
            chat_think_hovered_idx: &mut self.chat_think_hovered_idx,
            chat_think_scroll_ids: &mut self.chat_think_scroll_ids,
            chat_tool_expanded: &mut self.chat_tool_expanded,
            chat_tool_file_expanded: &mut self.chat_tool_file_expanded,
            chat_tool_file_hovered: &mut self.chat_tool_file_hovered,
            chat_tool_hovered_idx: &mut self.chat_tool_hovered_idx,
            chat_tool_text_editors: &mut self.chat_tool_text_editors,
            chat_virtualizer: &mut self.chat_virtualizer,
            chat_visible_text_cache: &mut self.chat_visible_text_cache,
            tool_detail_dialog: &mut self.tool_detail_dialog,
            message_meta_texts: &mut self.message_meta_texts,
            user_question_markers: &mut self.user_question_markers,
            user_question_indexed_chat_len: &mut self.user_question_indexed_chat_len,
            prepared_chat_ui_chunks: &mut self.prepared_chat_ui_chunks,
            preparing_chat_ui_chunks: &mut self.preparing_chat_ui_chunks,
            dirty_preparing_chat_ui_chunks: &mut self.dirty_preparing_chat_ui_chunks,
            last_visited_chat_ui_chunk_start: &mut self.last_visited_chat_ui_chunk_start,
            pinned_chat_ui_chunk_start: &mut self.pinned_chat_ui_chunk_start,
            desktop_locale: self.desktop_locale,
            dialogue_flow_show_reasoning_summary: self.dialogue_flow_show_reasoning_summary,
            is_requesting: self.is_requesting,
            timeline: None,
            window_width: self.window_width,
            is_dark: self.is_dark,
        }
    }
}
