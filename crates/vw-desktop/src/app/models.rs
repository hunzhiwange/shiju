//! 定义桌面应用共享的数据模型和 UI 状态结构。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// 对外暴露当前模块需要复用的能力。
pub use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, SessionTodoItem,
    ThinkTiming, TokenUsage, ToolTiming,
};

#[derive(Debug, Clone)]
/// 描述 ParsedChatBlock 支持的离散状态或消息分支。
pub enum ParsedChatBlock {
    Think { content: String, open: bool },
    Tool { raw: String },
    Text { content: String },
}

#[derive(Debug, Clone, Default)]
/// 表示 ChatRenderCacheEntry 相关的应用状态或派生数据。
pub struct ChatRenderCacheEntry {
    pub(crate) revision_key:
        Option<crate::app::components::chat_panel::message_view::RevisionRenderKey>,
    pub content_hash: u64,
    pub show_reasoning_summary: bool,
    pub copy_content_hash: Option<u64>,
    pub blocks: Vec<ParsedChatBlock>,
    pub tool_block_metadata: Vec<Option<ToolBlockRenderMetadata>>,
    pub has_special_blocks: bool,
    pub special_text_blocks: Vec<String>,
    pub tool_card_text_blocks: Vec<Vec<String>>,
    pub explore_summary_text_blocks: Vec<(usize, String)>,
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

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
