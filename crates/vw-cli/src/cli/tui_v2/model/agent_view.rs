use super::{PermissionMode, UiMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentStatus {
    Loading,
    Ready,
    Running,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentMember {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) team_id: Option<String>,
    pub(crate) name: String,
    pub(crate) status: AgentStatus,
    pub(crate) messages: Option<Vec<UiMessage>>,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) queued_input: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AgentViewState {
    pub(crate) members: Vec<AgentMember>,
    pub(crate) viewed_agent_id: Option<String>,
    pub(crate) error: Option<String>,
}

impl AgentViewState {
    pub(crate) fn switch_to(&mut self, agent_id: &str) -> Result<(), String> {
        if !self.members.iter().any(|member| member.id == agent_id) {
            return Err(format!("unknown agent: {agent_id}"));
        }
        self.viewed_agent_id = Some(agent_id.to_string());
        Ok(())
    }

    pub(crate) fn return_to_leader(&mut self) {
        self.viewed_agent_id = None;
    }

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

    pub(crate) fn viewed_member(&self) -> Option<&AgentMember> {
        let id = self.viewed_agent_id.as_deref()?;
        self.members.iter().find(|member| member.id == id)
    }

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
