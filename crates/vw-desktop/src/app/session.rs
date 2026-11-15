//! 聚合会话子模块并导出会话 UI 与加载相关能力。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::App;
pub(crate) use desktop_session::message::chat::stream::{bind_message_model_to_step, assign_message_model_to_step};
use super::models::{
    ChatMessage, ChatRenderCacheEntry, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep,
    TokenUsage,
};
use super::state::StepUiMeta;
use std::collections::HashMap;
use std::sync::Arc;

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
pub(crate) use self::view_cache::SessionViewCache;

#[cfg(test)]
use self::chat_ui::{explore_summary_animation_key, prioritize_chat_ui_chunk_starts};

const EMPTY_SESSION_ID: &str = "__empty__";

#[allow(dead_code)] // Used only by the retained full-session compatibility loader.
pub(crate) fn backfill_message_models_from_steps(
    messages: &mut [ChatMessage],
    steps: &[ChatSessionStep],
) {
    for step in steps {
        if let Some(model) = step.model.as_deref() {
            assign_message_model_to_step(messages, step.index, model, false);
        }
    }
}

#[cfg(test)]
#[path = "session/chat_timeline_tests.rs"]
mod chat_timeline_tests;
#[cfg(test)]
#[path = "session/stream_render_tests.rs"]
mod stream_render_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod view_cache_tests;
