use super::agents::{AgentsRuntime, send_to_viewed_agent, stop_viewed_agent};
use super::super::model::agent_view::{
    AgentMember, AgentStatus, AgentViewState,
};
use super::super::model::{
    PermissionMode, UiMessage, UiMessageBase, UiMessageId, UiSystemMessage, UiSystemMessageLevel,
};

#[derive(Default)]
struct RecordingAgents {
    sent: Vec<(String, String, String)>,
    stopped: Vec<String>,
    fail: bool,
}

impl AgentsRuntime for RecordingAgents {
    fn send_message(
        &mut self,
        agent_id: &str,
        session_id: &str,
        message: &str,
    ) -> Result<(), String> {
        if self.fail {
            return Err("scheduler unavailable".to_string());
        }
        self.sent.push((agent_id.to_string(), session_id.to_string(), message.to_string()));
        Ok(())
    }

    fn stop_agent(&mut self, agent_id: &str) -> Result<(), String> {
        if self.fail {
            return Err("stop rejected".to_string());
        }
        self.stopped.push(agent_id.to_string());
        Ok(())
    }
}

fn message(id: &str, text: &str) -> UiMessage {
    UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local(id)),
        text: text.to_string(),
        level: UiSystemMessageLevel::Info,
    })
}

#[test]
fn tui_agent_team_view_isolates_transcripts_input_and_permissions() {
    let leader = vec![message("leader", "leader secret transcript")];
    let mut view = AgentViewState {
        members: vec![
            AgentMember {
                id: "agent-a".to_string(),
                session_id: "session-a".to_string(),
                team_id: Some("team".to_string()),
                name: "Researcher".to_string(),
                status: AgentStatus::Running,
                messages: None,
                permission_mode: PermissionMode::Plan,
                queued_input: Vec::new(),
            },
            AgentMember {
                id: "agent-b".to_string(),
                session_id: "session-b".to_string(),
                team_id: Some("team".to_string()),
                name: "Coder".to_string(),
                status: AgentStatus::Ready,
                messages: Some(vec![message("agent-b", "agent only transcript")]),
                permission_mode: PermissionMode::Default,
                queued_input: Vec::new(),
            },
        ],
        ..AgentViewState::default()
    };

    view.switch_to("agent-a").unwrap();
    assert!(view.visible_messages(&leader).is_empty(), "unloaded agent never falls back to leader");
    assert_eq!(view.viewed_member().unwrap().permission_mode, PermissionMode::Plan);
    view.switch_to("agent-b").unwrap();
    assert_eq!(view.visible_messages(&leader).len(), 1);

    let mut runtime = RecordingAgents::default();
    send_to_viewed_agent(&mut view, "work only here", &mut runtime).unwrap();
    assert_eq!(
        runtime.sent[0],
        (
            "agent-b".to_string(),
            "session-b".to_string(),
            "work only here".to_string(),
        )
    );
    runtime.fail = true;
    assert!(stop_viewed_agent(&mut view, true, &mut runtime).is_err());
    assert_eq!(view.viewed_member().unwrap().status, AgentStatus::Ready);
    runtime.fail = false;
    stop_viewed_agent(&mut view, true, &mut runtime).unwrap();
    assert!(view.viewed_agent_id.is_none(), "completed/stopped teammate exits view");
    assert_eq!(view.visible_messages(&leader), leader.as_slice());
}
