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
    /// 切换到已知成员；未知 ID 会显式报错且不改变当前视图。
    #[cfg(test)]
    pub(crate) fn switch_to(&mut self, agent_id: &str) -> Result<(), String> {
        if !self.members.iter().any(|member| member.id == agent_id) {
            return Err(format!("unknown agent: {agent_id}"));
        }
        self.viewed_agent_id = Some(agent_id.to_string());
        Ok(())
    }

    /// 恢复显示主智能体的消息。
    #[cfg(test)]
    pub(crate) fn return_to_leader(&mut self) {
        self.viewed_agent_id = None;
    }

    /// 返回当前视图应渲染的消息；成员尚无快照时返回空切片。
    #[cfg(test)]
    pub(crate) fn visible_messages<'a>(
        &'a self,
        leader_messages: &'a [UiMessage],
    ) -> &'a [UiMessage] {
        let Some(agent_id) = self.viewed_agent_id.as_deref() else {
            return leader_messages;
        };
        self.members
            .iter()
            .find(|member| member.id == agent_id)
            .and_then(|member| member.messages.as_deref())
            .unwrap_or(&[])
    }

    /// 返回当前正在查看的成员，主智能体视图下为空。
    #[cfg(test)]
    pub(crate) fn viewed_member(&self) -> Option<&AgentMember> {
        let id = self.viewed_agent_id.as_deref()?;
        self.members.iter().find(|member| member.id == id)
    }

    /// 更新成员状态，并在当前成员结束或停止后自动返回主视图。
    #[cfg(test)]
    pub(crate) fn update_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(member) = self.members.iter_mut().find(|member| member.id == agent_id) {
            member.status = status;
        }
        if self.viewed_agent_id.as_deref() == Some(agent_id)
            && matches!(status, AgentStatus::Completed | AgentStatus::Stopped)
        {
            self.return_to_leader();
        }
    }
}
