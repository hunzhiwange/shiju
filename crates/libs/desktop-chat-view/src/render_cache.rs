//! 聊天视图拥有的解析缓存与修订键。
use crate::message_view::ParsedChatBlock;
use desktop_session::timeline_identity::{ChatEntryKey, ChatEntryRevision};

#[derive(Debug, Clone, Default)]
/// 表示 ChatRenderCacheEntry 相关的应用状态或派生数据。
pub struct ChatRenderCacheEntry {
    pub revision_key: Option<RevisionRenderKey>,
    pub content_hash: u64,
    pub show_reasoning_summary: bool,
    pub copy_content_hash: Option<u64>,
    pub blocks: Vec<ParsedChatBlock>,
    pub tool_block_metadata: Vec<Option<ToolBlockRenderMetadata>>,
    pub has_special_blocks: bool,
    pub special_text_blocks: Vec<String>,
    pub tool_card_text_blocks: Vec<ToolCardTextSource>,
    pub explore_summary_text_blocks: Vec<(usize, desktop_i18n::chat::ExploreCounts)>,
    pub display_text: String,
    pub preview_text: String,
    pub foldable: bool,
    pub is_large_message: bool,
    pub estimated_collapsed_height: f32,
    pub estimated_expanded_height: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolBlockRenderMetadata {
    pub name: Option<String>,
    pub identity: Option<String>,
}


/// Parsed tool payload cached independently of generated display text.
#[derive(Debug, Clone)]
pub struct ToolCardTextSource {
    pub tool_name: String,
    pub value: serde_json::Value,
}

const RENDER_WIDTH_BUCKET_PX: f32 = 32.0;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRenderKey {
    entry_key: ChatEntryKey,
    revision: ChatEntryRevision,
    width_bucket: u32,
    dark_theme: bool,
    show_reasoning_summary: bool,
}

impl RevisionRenderKey {
    pub fn new(
        entry_key: ChatEntryKey,
        revision: ChatEntryRevision,
        width: f32,
        dark_theme: bool,
        show_reasoning_summary: bool,
    ) -> Self {
        Self {
            entry_key,
            revision,
            width_bucket: width_bucket(width),
            dark_theme,
            show_reasoning_summary,
        }
    }

    pub fn matches_key(&self, other: &Self) -> bool {
        self == other
    }
}

fn width_bucket(width: f32) -> u32 {
    (width.max(0.0) / RENDER_WIDTH_BUCKET_PX).floor() as u32
}

#[cfg(test)]
#[path = "render_cache_tests.rs"]
mod render_cache_tests;
