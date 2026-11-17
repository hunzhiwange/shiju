//! 聚合会话子模块并导出会话 UI 与加载相关能力。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::App;
#[cfg(test)]
use super::models::ChatRole;
use super::models::{ChatMessage, ChatSession, ChatSessionStep, TokenUsage};
pub(crate) use desktop_session::message::chat::stream::bind_message_model_to_step;
use std::collections::HashMap;

mod active;
mod agent_children;
pub(crate) mod chat_timeline;
mod chat_ui;
pub(crate) mod hover_details;
mod loading;
mod runtime;
pub(crate) mod stream_render;
pub(crate) mod ui_projection;
pub(crate) mod view_cache;

pub use self::agent_children::{
    AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus,
    agent_child_session_projection,
};
/// 对外暴露当前模块需要复用的能力。
pub use self::chat_ui::{
    CHAT_UI_CHUNK_SIZE, PreparedChatUiChunk, PreparedChatUiMessage, PreparedChatUiPhase,
    chat_ui_chunk_bounds, chat_ui_chunk_start_idx, prepare_chat_ui_chunk_phase,
    prepare_chat_ui_message_phase,
};
/// 对外暴露当前模块需要复用的能力。
pub use self::loading::{
    PreparedSessionOpen, SharedChatMessages, build_session_previews, prepare_bounded_session_open,
    prepare_session_open, prepare_session_open_from_ui_store, prepare_timeline_page,
    session_content_signature, session_usage, shared_chat_messages,
};

const EMPTY_SESSION_ID: &str = "__empty__";

pub(crate) use desktop_session::session::backfill_message_models_from_steps;

#[cfg(test)]
mod view_cache_tests;

#[cfg(test)]
pub(crate) use desktop_session::message::chat::stream::assign_message_model_to_step;
#[cfg(test)]
pub(crate) use self::view_cache::SessionViewCache;
