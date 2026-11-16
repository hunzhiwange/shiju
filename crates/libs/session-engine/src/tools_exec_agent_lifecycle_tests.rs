//! 子代理载荷、状态解析与后台跟踪的库内契约。

fn new_state() -> crate::runner_types::ToolSessionState {
    crate::runner_types::ToolSessionState::default()
}

#[test]
fn agent_tool_running_event_exposes_persistent_child_before_completion() {
    let created = tool_agents::agent_tool::AgentToolChildSessionCreated {
        parent_session_id: "parent-session".to_string(),
        child_session_id: "child-session".to_string(),
        agent: "writer".to_string(),
        task: "inspect".to_string(),
    };
    let payload = super::agent_tool_running_payload(
        r#"{"action":"launch","agent":"writer","prompt":"inspect"}"#,
        &created,
    );

    assert_eq!(payload["status"], "running");
    assert_eq!(payload["metadata"]["parent_session_id"], "parent-session");
    assert_eq!(payload["metadata"]["child_session_id"], "child-session");
    assert_eq!(payload["metadata"]["agent"], "writer");
    assert_eq!(payload["metadata"]["task"], "inspect");

    let terminal = super::error_tool_payload_with_agent(
        Some("sanitized input"),
        "provider unavailable",
        Some(&created),
    );
    assert_eq!(terminal["status"], "error");
    assert_eq!(terminal["metadata"]["child_session_id"], "child-session");
}

#[test]
fn agent_tool_lifecycle_status_covers_wait_failure_and_cancellation() {
    for (output, expected) in [
        (r#"{"action":"status","tasks":[{"state":"running"}]}"#, "running"),
        (r#"{"action":"wait","timed_out":true,"tasks":[{"state":"running"}]}"#, "waiting"),
        (r#"{"action":"wait","tasks":[{"state":"failed"}]}"#, "failed"),
        (r#"{"action":"stop","tasks":[{"state":"cancelled"}]}"#, "cancelled"),
        (r#"{"action":"wait","tasks":[{"state":"succeeded"}]}"#, "completed"),
    ] {
        assert_eq!(super::agent_tool_lifecycle_status(output), expected, "{output}");
    }
}

#[test]
fn background_child_creation_is_tracked_for_finalization() {
    let created = tool_agents::agent_tool::AgentToolChildSessionCreated {
        parent_session_id: "parent-session".to_string(),
        child_session_id: "child-session".to_string(),
        agent: "writer".to_string(),
        task: "inspect".to_string(),
    };
    let mut state = new_state();

    super::track_background_agent_child(
        &mut state,
        r#"{"action":"launch","agent":"writer","prompt":"inspect"}"#,
        &created,
    );

    assert_eq!(state.agent_background_children, vec!["child-session"]);
}

#[test]
fn agent_tool_management_actions_do_not_track_new_background_children() {
    let created = tool_agents::agent_tool::AgentToolChildSessionCreated {
        parent_session_id: "parent-session".to_string(),
        child_session_id: "child-session".to_string(),
        agent: "writer".to_string(),
        task: "inspect".to_string(),
    };
    for action in ["status", "wait", "stop"] {
        let mut state = new_state();
        super::track_background_agent_child(
            &mut state,
            &format!(r#"{{"action":"{action}","task_ids":["child-session"]}}"#),
            &created,
        );
        assert!(state.agent_background_children.is_empty(), "{action}");
    }
}

#[test]
fn agent_tool_finalization_tracks_only_background_launches() {
    let mut state = new_state();
    let created = tool_agents::agent_tool::AgentToolChildSessionCreated {
        parent_session_id: "parent".to_string(),
        child_session_id: "child-launch".to_string(),
        agent: "researcher".to_string(),
        task: "search".to_string(),
    };

    super::track_background_agent_child(
        &mut state,
        r#"{"action":"launch","agent":"researcher","prompt":"search"}"#,
        &created,
    );
    super::track_background_agent_child(
        &mut state,
        r#"{"action":"spawn","agent":"researcher","prompt":"search"}"#,
        &created,
    );
    let mut sync_created = created.clone();
    sync_created.child_session_id = "child-run".to_string();
    super::track_background_agent_child(
        &mut state,
        r#"{"action":"run","agent":"researcher","prompt":"search"}"#,
        &sync_created,
    );

    assert_eq!(state.agent_background_children, vec!["child-launch"]);
    assert!(!super::agent_tool_launches_background(
        r#"{"action":"wait","child_session_id":"child-launch"}"#
    ));
}
