//! 子智能体列表与当前查看对象的视图状态。
//!
//! 本模块只维护成员快照和消息视图切换；智能体调度与会话生命周期由运行时处理。

use super::{PermissionMode, UiMessage};

/// 成员在 TUI 中展示的粗粒度运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Loading,
    Ready,
    Running,
    Completed,
    Failed,
    Stopped,
}

/// 一个可切换查看的智能体成员快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMember {
    pub id: String,
    pub session_id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub status: AgentStatus,
    pub messages: Option<Vec<UiMessage>>,
    pub permission_mode: PermissionMode,
    pub queued_input: Vec<String>,
}

/// 成员面板状态；`viewed_agent_id` 为空时显示主智能体消息。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentViewState {
    pub members: Vec<AgentMember>,
    pub viewed_agent_id: Option<String>,
    pub error: Option<String>,
}

impl AgentViewState {
}
