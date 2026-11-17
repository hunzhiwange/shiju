
pub(crate) use desktop_settings_agents::state::MAIN_AGENT_KEY;


pub(crate) use desktop_session::session::agent::clear_pending_guide_handoff;
pub(crate) type SessionRuntimeState = desktop_session::session::agent::SessionRuntimeState<
    desktop_chat_view::stream_render::StreamRenderState,
>;

pub use desktop_session::message::chat::request::{
    AcpHistoryReplayMode, AgentRequest, ChatSendBehavior, QueueItem,
};


pub(crate) use desktop_session::session::active::UserQuestionMarker;

pub(crate) type ActiveSessionViewState = desktop_session::session::active::ActiveSessionViewState<
    crate::app::components::chat_panel::trajectory::TrajectoryProjection,
>;


/// 会话原始 LLM 载荷的稳定读取键。
pub(crate) use desktop_session::message::chat::trajectory::SessionStepRawKey;


#[cfg(test)]
use super::*;
#[cfg(test)]
pub(crate) use desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState;
#[cfg(test)]
pub(crate) use desktop_session::message::chat::input::tool_detail::ToolDetailDialog;
#[cfg(test)]
pub(crate) use desktop_session::message::chat::stream::tail::StreamTailState;
#[cfg(test)]
pub(crate) use desktop_session::session::active::StepUiMeta;
#[cfg(test)]
pub(crate) use desktop_session::session::agent::SessionDraft;
#[cfg(test)]
pub(crate) use desktop_session::session::agent::mark_pending_guide_handoff;
#[cfg(test)]
pub(crate) use desktop_session::session::agent::take_pending_guide_handoff;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_DETAIL_BASIC_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_DETAIL_IDENTITY_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_DETAIL_MCP_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_DETAIL_SKILLS_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_DETAIL_TOOLS_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_PROMPT_SYSTEM_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_TOOLS_BUILTIN_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AGENT_TOOLS_OPTIONAL_TAB;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::AgentSettingsEntryKind;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::WORKSPACE_IDENTITY_FILES;
#[cfg(test)]
pub(crate) use desktop_settings_agents::state::WorkspaceIdentityFileState;
