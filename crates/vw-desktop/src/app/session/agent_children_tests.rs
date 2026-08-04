use super::{AgentChildSession, ChatMessage, ChatRole, agent_child_session_projection};
use vw_shared::session::info::{Info, TimeInfo};

fn message(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session(id: &str, parent_id: Option<&str>, title: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: parent_id.map(str::to_string),
        summary: None,
        share: None,
        title: title.to_string(),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 1, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}

#[test]
fn agent_child_session_projection_dedupes_running_tools_and_drops_terminal_items() {
    let long_task =
        "检查 Unicode 子代理任务摘要是否能够稳定截断并且不会破坏字符边界，同时保留可辨识内容";
    let chat = vec![
        message(
            r#"前置文本
tool Agent
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"child-known","agent":"writer","task":"first"}}
tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"child-known","agent":"writer-2","task":"latest"}}"#,
        ),
        message(&format!(
            r#"tool agent_tool
{{"status":"running","metadata":{{"parent_session_id":"parent","child_session_id":"child-fallback","agent":"reviewer","task":"{long_task}"}}}}"#
        )),
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"other-parent","child_session_id":"cross-parent","agent":"writer","task":"wrong parent"}}"#,
        ),
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":" ","agent":"writer","task":"empty id"}}"#,
        ),
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"child-completed","agent":"writer","task":"complete me"}}
tool Agent
{"status":"completed","output":"{\"session_id\":\"child-completed\",\"agent\":\"writer\",\"task\":\"complete me\"}"}"#,
        ),
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"child-failed","agent":"tester","task":"fail me"}}
tool AgentTool
{"status":"error","metadata":{"parent_session_id":"parent","child_session_id":"child-failed","agent":"tester","task":"fail me"},"error":"provider unavailable"}"#,
        ),
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"wrong-info-parent","agent":"tester","task":"wrong info parent"}}"#,
        ),
        message("tool AgentTool\nnot-json"),
    ];
    let known_sessions = [
        session("parent", None, "Parent", "/repo"),
        session("child-known", Some("parent"), "真实子会话标题", "/repo/child-known"),
        session("wrong-info-parent", Some("other-parent"), "Wrong", "/other"),
    ];

    let projected = agent_child_session_projection(Some("parent"), &chat, known_sessions.iter());

    assert_eq!(
        projected,
        vec![
            AgentChildSession {
                session_id: "child-known".to_string(),
                agent: "writer-2".to_string(),
                task: "latest".to_string(),
                title: "真实子会话标题".to_string(),
                directory: "/repo/child-known".to_string(),
                kind: super::AgentChildSessionKind::Agent,
                status: super::AgentChildSessionStatus::Running,
                depth: 0,
                plan_run_task_id: None,
            },
            AgentChildSession {
                session_id: "child-fallback".to_string(),
                agent: "reviewer".to_string(),
                task: long_task.to_string(),
                title: "reviewer · 检查 Unicode 子代理任务摘要是否能够稳定截断并且不会破坏字符边界，同时保留…"
                    .to_string(),
                directory: "/repo".to_string(),
                kind: super::AgentChildSessionKind::Agent,
                status: super::AgentChildSessionStatus::Running,
                depth: 0,
                plan_run_task_id: None,
            },
        ]
    );

    assert!(agent_child_session_projection(None, &chat, std::iter::empty::<&Info>(),).is_empty());
}

#[test]
fn agent_tool_batch_lifecycle_projection_removes_terminal_children() {
    let chat = vec![
        message(
            r#"tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"done","agent":"writer","task":"done"}}
tool AgentTool
{"status":"running","metadata":{"parent_session_id":"parent","child_session_id":"active","agent":"reviewer","task":"active"}}"#,
        ),
        message(
            r#"tool AgentTool
{"status":"waiting","output":"{\"action\":\"wait\",\"timed_out\":true,\"tasks\":[{\"parent_session_id\":\"parent\",\"child_session_id\":\"done\",\"agent\":\"writer\",\"task\":\"done\",\"state\":\"succeeded\"},{\"parent_session_id\":\"parent\",\"child_session_id\":\"active\",\"agent\":\"reviewer\",\"task\":\"active\",\"state\":\"running\"}]}"}"#,
        ),
    ];
    let known_sessions = [session("parent", None, "Parent", "/repo")];

    let projected = agent_child_session_projection(Some("parent"), &chat, known_sessions.iter());

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].session_id, "active");
    assert_eq!(projected[0].status, super::AgentChildSessionStatus::Waiting);
}

#[test]
fn plan_run_controls_extract_latest_task_owner_and_waiting_state() {
    let content = concat!(
        "tool workflow_node\n",
        "{\"status\":\"running\",\"metadata\":{\"plan_run\":true,\"task_id\":\"task-1\",",
        "\"session_id\":\"task-1-1\"}}\n",
        "tool workflow_node\n",
        "{\"status\":\"waiting\",\"metadata\":{\"plan_run\":true,\"task_id\":\"task-1\",",
        "\"session_id\":\"task-1-1\"}}\n",
        "tool workflow_node\n",
        "{\"status\":\"running\",\"metadata\":{\"plan_run\":false,\"task_id\":\"ignored\",",
        "\"session_id\":\"ignored-1\"}}\n",
    );

    assert_eq!(
        super::plan_run_session_controls_from_content(content),
        vec![super::PlanRunSessionControl {
            session_id: "task-1-1".to_string(),
            task_id: "task-1".to_string(),
            status: super::AgentChildSessionStatus::Waiting,
        }]
    );
}
