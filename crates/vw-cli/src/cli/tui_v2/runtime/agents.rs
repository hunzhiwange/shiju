use super::super::model::agent_view::{
    AgentMember, AgentStatus, AgentViewState,
};
use super::super::model::PermissionMode;
use super::gateway::{GatewayUiRuntime, block_on_gateway};

pub(crate) trait AgentsRuntime {
    fn list_agents(&mut self) -> Result<Vec<AgentMember>, String> {
        Err("agent listing is unavailable".to_string())
    }
    fn send_message(&mut self, agent_id: &str, session_id: &str, message: &str)
        -> Result<(), String>;
    fn stop_agent(&mut self, agent_id: &str) -> Result<(), String>;
}

impl AgentsRuntime for GatewayUiRuntime {
    fn list_agents(&mut self) -> Result<Vec<AgentMember>, String> {
        let scope = self.agent_scheduler_scope_blocking()?;
        block_on_gateway(self.client().agent_scheduler_workers(&scope)).map(|response| {
            response
                .workers
                .into_iter()
                .map(|worker| AgentMember {
                    id: worker.id.clone(),
                    session_id: String::new(),
                    team_id: worker.team_id,
                    name: worker
                        .worker_role
                        .or(worker.runtime)
                        .unwrap_or_else(|| worker.id.clone()),
                    status: match format!("{:?}", worker.status).as_str() {
                        "Running" | "Busy" => AgentStatus::Running,
                        "Stopped" => AgentStatus::Stopped,
                        "Failed" | "Unavailable" => AgentStatus::Failed,
                        _ => AgentStatus::Ready,
                    },
                    messages: None,
                    permission_mode: PermissionMode::Default,
                    queued_input: Vec::new(),
                })
                .collect()
        })
    }

    fn send_message(
        &mut self,
        _agent_id: &str,
        _session_id: &str,
        _message: &str,
    ) -> Result<(), String> {
        Err("Gateway does not expose a scoped worker message command".to_string())
    }

    fn stop_agent(&mut self, _agent_id: &str) -> Result<(), String> {
        Err("Gateway does not expose a scoped worker stop command".to_string())
    }
}

pub(crate) fn send_to_viewed_agent(
    view: &mut AgentViewState,
    message: &str,
    runtime: &mut dyn AgentsRuntime,
) -> Result<(), String> {
    let member = view
        .viewed_member()
        .ok_or_else(|| "no teammate is currently viewed".to_string())?;
    if message.trim().is_empty() {
        return Err("agent message is empty".to_string());
    }
    let agent_id = member.id.clone();
    let session_id = member.session_id.clone();
    runtime.send_message(&agent_id, &session_id, message)?;
    Ok(())
}

pub(crate) fn stop_viewed_agent(
    view: &mut AgentViewState,
    confirmed: bool,
    runtime: &mut dyn AgentsRuntime,
) -> Result<(), String> {
    if !confirmed {
        return Err("stopping a teammate requires confirmation".to_string());
    }
    let agent_id = view
        .viewed_member()
        .map(|member| member.id.clone())
        .ok_or_else(|| "no teammate is currently viewed".to_string())?;
    runtime.stop_agent(&agent_id)?;
    view.update_status(&agent_id, AgentStatus::Stopped);
    Ok(())
}
