use std::collections::HashMap;

use vw_shared::session::info::Info;

use super::{ChatMessage, ChatRole};
use crate::app::components::chat_panel::tools::{
    AgentToolSessionPayload, AgentToolSessionStatus, agent_tool_session_payloads_from_content,
};

const FALLBACK_TASK_MAX_CHARS: usize = 41;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentChildSessionKind {
    #[default]
    Agent,
    WorkflowExecution,
    WorkflowPlanning,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentChildSessionStatus {
    #[default]
    Running,
    Waiting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentChildSession {
    pub session_id: String,
    pub agent: String,
    pub task: String,
    pub title: String,
    pub directory: String,
    pub kind: AgentChildSessionKind,
    pub status: AgentChildSessionStatus,
    pub depth: usize,
    pub plan_run_task_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanRunSessionControl {
    pub(crate) session_id: String,
    pub(crate) task_id: String,
    pub(crate) status: AgentChildSessionStatus,
}

fn compact_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_task(value: &str) -> String {
    let compact = compact_text(value);
    let mut chars = compact.chars();
    let truncated = chars.by_ref().take(FALLBACK_TASK_MAX_CHARS).collect::<String>();
    if chars.next().is_some() { format!("{truncated}…") } else { truncated }
}

fn fallback_title(agent: &str, task: &str) -> String {
    let agent = compact_text(agent);
    let task = truncate_task(task);
    match (agent.is_empty(), task.is_empty()) {
        (false, false) => format!("{agent} · {task}"),
        (false, true) => agent,
        (true, false) => task,
        (true, true) => "AgentTool 子会话".to_string(),
    }
}

fn plan_run_control_from_value(value: &serde_json::Value) -> Option<PlanRunSessionControl> {
    let metadata = value.get("metadata")?.as_object()?;
    if metadata.get("plan_run").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }
    let session_id =
        metadata.get("session_id")?.as_str()?.trim().to_string();
    let task_id = metadata.get("task_id")?.as_str()?.trim().to_string();
    if session_id.is_empty() || task_id.is_empty() {
        return None;
    }
    let status = if value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status.trim().eq_ignore_ascii_case("waiting"))
    {
        AgentChildSessionStatus::Waiting
    } else {
        AgentChildSessionStatus::Running
    };

    Some(PlanRunSessionControl { session_id, task_id, status })
}

fn is_tool_block_boundary(content: &str, start: usize) -> bool {
    start == 0 || content[..start].ends_with('\n')
}

pub(crate) fn plan_run_session_controls_from_content(
    content: &str,
) -> Vec<PlanRunSessionControl> {
    let mut controls = Vec::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut cursor = 0;

    while let Some(relative_start) = content[cursor..].find("tool ") {
        let start = cursor + relative_start;
        if !is_tool_block_boundary(content, start) {
            cursor = start + "tool ".len();
            continue;
        }
        let Some(tail) = content.get(start..) else {
            break;
        };
        let Some(line_end) = tail.find('\n') else {
            break;
        };
        let tool_name = tail[..line_end].trim().strip_prefix("tool ").map(str::trim);
        let json_start = start + line_end + 1;
        let Some(json_tail) = content.get(json_start..) else {
            break;
        };
        let mut values =
            serde_json::Deserializer::from_str(json_tail).into_iter::<serde_json::Value>();
        let Some(Ok(value)) = values.next() else {
            cursor = start + "tool ".len();
            continue;
        };
        cursor = json_start + values.byte_offset();

        if tool_name != Some("workflow_node") {
            continue;
        }
        let Some(control) = plan_run_control_from_value(&value) else {
            continue;
        };
        if let Some(position) = positions.get(&control.session_id).copied() {
            controls[position] = control;
        } else {
            positions.insert(control.session_id.clone(), controls.len());
            controls.push(control);
        }
    }

    controls
}

fn payload_targets_parent(
    payload: &AgentToolSessionPayload,
    parent_session_id: &str,
    known_session: Option<&Info>,
) -> bool {
    if payload.parent_session_id.as_deref().is_some_and(|parent| parent != parent_session_id) {
        return false;
    }

    !known_session
        .and_then(|session| session.parent_id.as_deref())
        .is_some_and(|parent| !parent.trim().is_empty() && parent.trim() != parent_session_id)
}

pub fn agent_child_session_projection<'a>(
    active_parent_session_id: Option<&str>,
    chat: &[ChatMessage],
    known_sessions: impl IntoIterator<Item = &'a Info>,
) -> Vec<AgentChildSession> {
    let Some(parent_session_id) =
        active_parent_session_id.map(str::trim).filter(|session_id| !session_id.is_empty())
    else {
        return Vec::new();
    };

    let mut known_by_id = HashMap::new();
    for session in known_sessions {
        known_by_id.entry(session.id.as_str()).or_insert(session);
    }
    let fallback_directory = known_by_id
        .get(parent_session_id)
        .map(|session| session.directory.trim())
        .filter(|directory| !directory.is_empty())
        .unwrap_or_default()
        .to_string();

    let mut projected = Vec::<AgentToolSessionPayload>::new();
    let mut positions = HashMap::<String, usize>::new();
    for message in chat {
        if !matches!(message.role, ChatRole::Assistant | ChatRole::Tool) {
            continue;
        }
        for payload in agent_tool_session_payloads_from_content(&message.content) {
            let known_session = known_by_id.get(payload.child_session_id.as_str()).copied();
            if !payload_targets_parent(&payload, parent_session_id, known_session) {
                continue;
            }

            if let Some(position) = positions.get(&payload.child_session_id).copied() {
                projected[position] = payload;
            } else {
                positions.insert(payload.child_session_id.clone(), projected.len());
                projected.push(payload);
            }
        }
    }

    projected
        .into_iter()
        .filter(|payload| {
            matches!(
                payload.status,
                AgentToolSessionStatus::Running | AgentToolSessionStatus::Waiting
            )
        })
        .map(|payload| {
            let known_session = known_by_id.get(payload.child_session_id.as_str()).copied();
            let title = known_session
                .map(|session| session.title.trim())
                .filter(|title| !title.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| fallback_title(&payload.agent, &payload.task));
            let directory = known_session
                .map(|session| session.directory.trim())
                .filter(|directory| !directory.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| fallback_directory.clone());

            AgentChildSession {
                session_id: payload.child_session_id,
                agent: payload.agent,
                task: payload.task,
                title,
                directory,
                kind: AgentChildSessionKind::Agent,
                status: if payload.status == AgentToolSessionStatus::Waiting {
                    AgentChildSessionStatus::Waiting
                } else {
                    AgentChildSessionStatus::Running
                },
                depth: 0,
                plan_run_task_id: None,
            }
        })
        .collect()
}
