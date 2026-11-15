//! 定义桌面应用共享的数据模型和 UI 状态结构。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// 对外暴露当前模块需要复用的能力。
pub use session_ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, SessionTodoItem,
    ThinkTiming, TokenUsage, ToolTiming,
};

pub use desktop_chat_view::message_view::ParsedChatBlock;

pub use desktop_chat_view::render_cache::{ChatRenderCacheEntry, ToolBlockRenderMetadata, ToolCardTextSource};

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;
