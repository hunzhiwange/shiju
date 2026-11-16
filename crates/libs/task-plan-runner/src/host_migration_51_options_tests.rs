use crate::SessionRunRequest;
#[test]
fn gateway_session_options_always_enable_full_access() {
    let request = SessionRunRequest {
        session_id: "s1".to_string(),
        query: "q".to_string(),
        root: "/tmp/project".to_string(),
        model: Some("m".to_string()),
        agent: Some("main".to_string()),
        acp_agent: None,
    };
    let options = crate::gateway_session_options(&request, false);
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("cwd").and_then(serde_json::Value::as_str), Some("/tmp/project"));
    assert_eq!(options.get("agent").and_then(serde_json::Value::as_str), Some("main"));
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(options.get("goal_mode").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("workflow_mode").and_then(serde_json::Value::as_bool), Some(true));
    assert!(!options.contains_key("acp_permission_mode"));
}



#[test]
fn acp_background_approval_plan_run_defaults_to_approval() {
    let request = SessionRunRequest {
        session_id: "s1".to_string(),
        query: "q".to_string(),
        root: "/tmp/project".to_string(),
        model: None,
        agent: None,
        acp_agent: Some("codex".to_string()),
    };
    let options = crate::gateway_session_options(&request, false);
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(options.get("acp_agent").and_then(serde_json::Value::as_str), Some("codex"));
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-reads")
    );
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(true));
}



#[test]
fn gateway_session_options_respects_confirmed_acp_full_access() {
    let request = SessionRunRequest {
        session_id: "s1".into(),
        query: "q".into(),
        root: "/tmp/project".into(),
        model: None,
        agent: None,
        acp_agent: Some("codex".into()),
    };
    let options = crate::gateway_session_options(&request, true);
    assert_eq!(options["full_access"], true);
    assert_eq!(options["acp_permission_mode"], "approve-all");
    assert_eq!(options["acp_non_interactive_permissions"], "deny");
}
