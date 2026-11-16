use super::*;

pub(crate) use desktop_settings_agents::state::{
    AGENT_PROMPT_SYSTEM_TAB, AgentSettingsEntryKind, MAIN_AGENT_KEY, WORKSPACE_IDENTITY_FILES,
    WorkspaceIdentityFileState,
};

pub(crate) use desktop_settings_agents::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_IDENTITY_TAB, AGENT_DETAIL_MCP_TAB,
    AGENT_DETAIL_SKILLS_TAB, AGENT_DETAIL_TOOLS_TAB, AGENT_TOOLS_BUILTIN_TAB,
    AGENT_TOOLS_OPTIONAL_TAB,
};

pub(crate) use desktop_session::session::agent::{
    SessionDraft, clear_pending_guide_handoff, mark_pending_guide_handoff,
    take_pending_guide_handoff,
};
pub(crate) type SessionRuntimeState = desktop_session::session::agent::SessionRuntimeState<
    desktop_chat_view::stream_render::StreamRenderState,
>;

pub use desktop_session::message::chat::request::{
    AcpHistoryReplayMode, AgentRequest, ChatSendBehavior, QueueItem,
};

pub(crate) use desktop_session::message::chat::stream::tail::StreamTailState;

pub(crate) use desktop_session::session::active::{StepUiMeta, UserQuestionMarker};

pub(crate) type ActiveSessionViewState = desktop_session::session::active::ActiveSessionViewState<
    crate::app::components::chat_panel::trajectory::TrajectoryProjection,
>;

pub(crate) use desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState;

/// 会话原始 LLM 载荷的稳定读取键。
pub(crate) use desktop_session::message::chat::trajectory::SessionStepRawKey;

pub(crate) use desktop_session::message::chat::input::tool_detail::ToolDetailDialog;
