//! Stable render snapshot data and memory accounting.
use crate::{height_index::ChatHeightIndex, virtualizer::ChatVirtualizer};
use crate::message_view::{markdown::MarkdownBlock, ParsedChatBlock};
use crate::render_cache::ChatRenderCacheEntry;
use session_ui_types::{ChatMessage, TokenUsage};
use iced::widget::text_editor;
use std::{collections::{HashMap, HashSet}, mem};
/// 一个会话恢复聊天视图所需的稳定工作集。
pub struct SessionViewSnapshot<A, M> {
    pub content_signature: u64,
    pub chat: Vec<ChatMessage>,
    pub chat_message_ids: Vec<Option<String>>,
    pub usage: TokenUsage,
    pub active_session_view_state: A,
    pub chat_panel_mode: M,
    pub chat_render_cache: HashMap<usize, ChatRenderCacheEntry>,
    pub chat_visible_text_cache: Vec<Option<String>>,
    pub chat_copy_hash_cache: Vec<Option<u64>>,
    pub chat_message_expanded: HashSet<usize>,
    pub chat_message_estimated_heights: Vec<f32>,
    pub chat_virtualizer: ChatVirtualizer,
    pub chat_height_index: ChatHeightIndex,
    pub chat_message_measured_heights: HashMap<usize, f32>,
    pub chat_message_editors: Vec<text_editor::Content>,
    pub chat_special_text_editors: HashMap<u64, text_editor::Content>,
    pub chat_markdown_blocks: HashMap<u64, Vec<MarkdownBlock>>,
    pub chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
    pub chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    pub chat_think_editors: HashMap<u64, text_editor::Content>,
    pub chat_think_expanded: HashSet<u64>,
    pub chat_think_collapsed: HashSet<u64>,
    pub chat_tool_file_expanded: HashSet<String>,
    pub chat_tool_expanded: HashSet<u64>,
    pub chat_explore_expanded: HashSet<u64>,
    pub chat_auto_scroll: bool,
    pub chat_scroll_id: iced::widget::Id,
    pub chat_scroll_offset_y: f32,
    pub chat_scroll_viewport_h: f32,
}

impl<A, M> SessionViewSnapshot<A, M> {
    pub fn chat(&self) -> &[ChatMessage] {
        &self.chat
    }

    pub fn content_signature(&self) -> u64 {
        self.content_signature
    }

    pub fn estimated_bytes(&self, active_state_bytes: usize) -> usize {
        let message_bytes = self
            .chat
            .iter()
            .map(|message| message.content.len() + message.model.as_deref().map_or(0, str::len));
        let render_bytes = self.chat_render_cache.values().map(render_cache_bytes);
        let visible_text_bytes =
            self.chat_visible_text_cache.iter().filter_map(Option::as_deref).map(str::len);
        let message_id_bytes =
            self.chat_message_ids.iter().filter_map(Option::as_deref).map(str::len);
        let markdown_bytes =
            self.chat_markdown_blocks.values().flatten().map(MarkdownBlock::stored_bytes);
        let editor_bytes = self
            .chat_message_editors
            .iter()
            .chain(self.chat_special_text_editors.values())
            .chain(self.chat_markdown_block_editors.values())
            .chain(self.chat_tool_text_editors.values())
            .chain(self.chat_think_editors.values())
            .map(text_editor_bytes);
        let expanded_file_bytes = self.chat_tool_file_expanded.iter().map(String::len);
        let structural_bytes = self.chat.len() * mem::size_of::<ChatMessage>()
            + self.chat_message_ids.len() * mem::size_of::<Option<String>>()
            + self.chat_render_cache.len() * mem::size_of::<(usize, ChatRenderCacheEntry)>()
            + self.chat_visible_text_cache.len() * mem::size_of::<Option<String>>()
            + self.chat_copy_hash_cache.len() * mem::size_of::<Option<u64>>()
            + self.chat_message_estimated_heights.len() * mem::size_of::<f32>()
            + self.chat_message_measured_heights.len() * mem::size_of::<(usize, f32)>()
            + self.chat_markdown_blocks.len() * mem::size_of::<(u64, Vec<MarkdownBlock>)>();

        message_bytes
            .chain(render_bytes)
            .chain(visible_text_bytes)
            .chain(message_id_bytes)
            .chain(markdown_bytes)
            .chain(editor_bytes)
            .chain(expanded_file_bytes)
            .sum::<usize>()
            .saturating_add(active_state_bytes)
            .saturating_add(structural_bytes)
    }
}

fn render_cache_bytes(entry: &ChatRenderCacheEntry) -> usize {
    let block_bytes = entry.blocks.iter().map(|block| match block {
        ParsedChatBlock::Think { content, .. } | ParsedChatBlock::Text { content } => content.len(),
        ParsedChatBlock::Tool { raw } => raw.len(),
    });
    let special_text_bytes = entry.special_text_blocks.iter().map(String::len);
    let tool_text_bytes = entry
        .tool_card_text_blocks
        .iter()
        .map(|source| source.tool_name.len() + source.value.to_string().len());
    let explore_bytes =
        entry.explore_summary_text_blocks.iter().map(|(_, counts)| std::mem::size_of_val(counts));

    entry.display_text.len()
        + entry.preview_text.len()
        + block_bytes
            .chain(special_text_bytes)
            .chain(tool_text_bytes)
            .chain(explore_bytes)
            .sum::<usize>()
}

fn text_editor_bytes(content: &text_editor::Content) -> usize {
    content.lines().map(|line| line.text.len()).sum()
}

/// Borrowed inputs used by message bubbles and assistant bodies.
pub use crate::message_view::render_snapshot::MessageViewSnapshot;
