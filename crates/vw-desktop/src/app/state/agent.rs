use super::*;

pub(crate) use desktop_settings_agents::state::{
    MAIN_AGENT_KEY, AGENT_PROMPT_SYSTEM_TAB, WORKSPACE_IDENTITY_FILES,
    WorkspaceIdentityFileState, AgentSettingsEntryKind,
};

pub(crate) use desktop_settings_agents::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_DETAIL_TOOLS_TAB, AGENT_DETAIL_SKILLS_TAB, AGENT_DETAIL_MCP_TAB, AGENT_TOOLS_OPTIONAL_TAB, AGENT_TOOLS_BUILTIN_TAB,
};

pub(crate) use desktop_session::session::agent::{
    SessionDraft, mark_pending_guide_handoff, take_pending_guide_handoff, clear_pending_guide_handoff,
};
pub(crate) type SessionRuntimeState = desktop_session::session::agent::SessionRuntimeState<
    desktop_chat_view::stream_render::StreamRenderState,
>;

pub use desktop_session::message::chat::request::{
    AcpHistoryReplayMode, ChatSendBehavior, AgentRequest, QueueItem,
};

pub(crate) use desktop_session::message::chat::stream::tail::StreamTailState;

pub(crate) use desktop_session::session::active::{StepUiMeta, UserQuestionMarker};

impl ActiveSessionViewState {
    pub(crate) fn estimated_bytes(&self) -> usize {
        desktop_session::session::active::estimated_bytes(
            &self.steps, &self.calls, &self.step_llm_raw, &self.step_index_map,
            &self.message_meta_texts, &self.user_question_markers,
        )
    }
}

pub(crate) use desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState;

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveSessionViewState {
    pub(crate) updated_ms: u64,
    pub(crate) steps: Vec<ChatSessionStep>,
    pub(crate) calls: Vec<Value>,
    pub(crate) step_llm_raw: HashMap<SessionStepRawKey, Value>,
    pub(crate) trajectory_projection:
        crate::app::components::chat_panel::trajectory::TrajectoryProjection,
    #[cfg(test)]
    pub(crate) trajectory_projection_rebuild_count: usize,
    pub(crate) llm_raw_step_indices: HashSet<SessionStepRawKey>,
    pub(crate) step_index_map: HashMap<u32, StepUiMeta>,
    pub(crate) message_meta_texts: Vec<Option<crate::app::i18n::chat::MessageMeta>>,
    pub(crate) user_question_markers: Vec<UserQuestionMarker>,
    pub(crate) user_question_indexed_chat_len: usize,
    pub(crate) ui_preparing: bool,
    pub(crate) base_ready: bool,
    pub(crate) prepared_chat_ui_chunks: HashSet<usize>,
    pub(crate) preparing_chat_ui_chunks: HashSet<usize>,
    pub(crate) dirty_preparing_chat_ui_chunks: HashSet<usize>,
    pub(crate) last_visited_chat_ui_chunk_start: Option<usize>,
    pub(crate) pinned_chat_ui_chunk_start: Option<usize>,
}

/// 会话原始 LLM 载荷的稳定读取键。
pub(crate) use desktop_session::message::chat::trajectory::SessionStepRawKey;

pub(crate) use desktop_session::message::chat::input::tool_detail::ToolDetailDialog;
