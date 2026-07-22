use super::*;
use crate::app::agent::config::{DelegateAgentConfig, MultimodalConfig};
use crate::app::agent::providers::ProviderRuntimeOptions;
use crate::app::agent::security::policy::AutonomyLevel;
use crate::app::agent::tools::subagent_registry::{SubAgentSession, SubAgentStatus};
use crate::app::agent::tools::traits::Tool;
use chrono::{Duration, Utc};
use serde_json::json;

fn build_tool_with_registry(
    registry: Arc<SubAgentRegistry>,
    security: SecurityPolicy,
) -> AgentTool {
    let agents = HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]);
    build_tool_with_agents(agents, registry, security)
}

fn build_tool_with_agents(
    agents: HashMap<String, DelegateAgentConfig>,
    registry: Arc<SubAgentRegistry>,
    mut security: SecurityPolicy,
) -> AgentTool {
    security.max_actions_per_hour = 10;
    let security = Arc::new(security);
    let delegate_tool = Arc::new(DelegateTool::new(
        agents.clone(),
        Some("fallback-key".to_string()),
        Arc::clone(&security),
    ));
    let background_tool = Arc::new(SubAgentSpawnTool::new(
        agents.clone(),
        Some("fallback-key".to_string()),
        Arc::clone(&security),
        ProviderRuntimeOptions::default(),
        Arc::clone(&registry),
        Arc::new(Vec::new()),
        MultimodalConfig::default(),
    ));

    AgentTool::new(agents, delegate_tool, background_tool, registry, security)
}

fn build_tool() -> (AgentTool, Arc<SubAgentRegistry>) {
    let registry = Arc::new(SubAgentRegistry::new());
    (build_tool_with_registry(Arc::clone(&registry), SecurityPolicy::default()), registry)
}

fn insert_session(
    registry: &SubAgentRegistry,
    id: &str,
    status: SubAgentStatus,
    result: Option<ToolResult>,
) {
    let now = Utc::now();
    registry.insert(SubAgentSession {
        id: id.to_string(),
        agent_name: "writer".to_string(),
        title: Some("Draft".to_string()),
        task: "write a careful summary".to_string(),
        metadata: json!({"topic": "coverage"}),
        status,
        started_at: now - Duration::milliseconds(12),
        updated_at: now,
        completed_at: result.as_ref().map(|_| now),
        result,
        #[cfg(not(target_arch = "wasm32"))]
        handle: None,
    });
}

#[test]
fn truncate_output_preserves_short_text_and_marks_truncation() {
    assert_eq!(truncate_output("short", 10), "short");
    assert_eq!(truncate_output("abcdef", 3), "abc... (truncated)");
}

#[test]
fn truncate_output_respects_char_boundaries() {
    assert_eq!(truncate_output("你好世界", 2), "你好... (truncated)");
}

#[test]
fn truncate_output_handles_zero_and_exact_character_limits() {
    assert_eq!(truncate_output("", 0), "");
    assert_eq!(truncate_output("abc", 0), "... (truncated)");
    assert_eq!(truncate_output("你好", 2), "你好");
}

#[test]
fn action_resolution_accepts_aliases_and_legacy_inference() {
    let (tool, _) = build_tool();

    assert!(matches!(tool.resolve_action(&json!({"action": "run"})).unwrap(), AgentAction::Launch));
    assert!(matches!(tool.resolve_action(&json!({"action": "status"})).unwrap(), AgentAction::Get));
    assert!(matches!(tool.resolve_action(&json!({"action": "kill"})).unwrap(), AgentAction::Stop));
    assert!(matches!(
        tool.resolve_action(&json!({"status": "running"})).unwrap(),
        AgentAction::List
    ));
    assert!(matches!(tool.resolve_action(&json!({"session_id": "s1"})).unwrap(), AgentAction::Get));
    assert!(matches!(tool.resolve_action(&json!({})).unwrap(), AgentAction::Launch));

    let message_err = match tool.resolve_action(&json!({"action": "message"})) {
        Ok(_) => panic!("message action should fail"),
        Err(err) => err,
    };
    assert!(message_err.to_string().contains("not implemented"));

    let unknown_err = match tool.resolve_action(&json!({"action": "dance"})) {
        Ok(_) => panic!("unknown action should fail"),
        Err(err) => err,
    };
    assert!(unknown_err.to_string().contains("Unknown AgentTool action"));
}

#[test]
fn action_resolution_normalizes_explicit_actions_and_preserves_precedence() {
    let (tool, _) = build_tool();

    assert!(matches!(
        tool.resolve_action(&json!({"action": " SPAWN "})).unwrap(),
        AgentAction::Launch
    ));
    assert!(matches!(tool.resolve_action(&json!({"action": "LIST"})).unwrap(), AgentAction::List));
    assert!(matches!(tool.resolve_action(&json!({"action": " get "})).unwrap(), AgentAction::Get));
    assert!(matches!(tool.resolve_action(&json!({"action": "STOP"})).unwrap(), AgentAction::Stop));

    assert!(matches!(
        tool.resolve_action(&json!({"action": " ", "status": "running"})).unwrap(),
        AgentAction::List
    ));
    assert!(matches!(
        tool.resolve_action(&json!({"status": "running", "agent": "writer"})).unwrap(),
        AgentAction::Launch
    ));
    assert!(matches!(
        tool.resolve_action(&json!({"status": "running", "session_id": "s1"})).unwrap(),
        AgentAction::Get
    ));
    assert!(matches!(
        tool.resolve_action(&json!({"action": "list", "session_id": "s1"})).unwrap(),
        AgentAction::List
    ));
}

#[test]
fn argument_helpers_trim_and_validate_aliases() {
    let (tool, _) = build_tool();
    let args = json!({
        "subagent_type": " writer ",
        "task": " draft ",
        "run_in_background": false,
        "status": "all"
    });

    assert_eq!(tool.agent_name_from_args(&args), Some("writer"));
    assert_eq!(tool.prompt_from_args(&args), Some("draft"));
    assert!(!tool.run_in_background(&args));
    assert_eq!(tool.normalized_status_filter(&args).unwrap(), None);
    assert_eq!(tool.session_id_from_args(&json!({"session_id": " s1 "})).unwrap(), "s1");

    let err = tool.session_id_from_args(&json!({"session_id": " "})).unwrap_err();
    assert!(err.to_string().contains("Missing 'session_id'"));

    let err = tool.normalized_status_filter(&json!({"status": "paused"})).unwrap_err();
    assert!(err.to_string().contains("Invalid status filter"));
}

#[test]
fn argument_helpers_cover_primary_fields_types_and_status_boundaries() {
    let (tool, _) = build_tool();
    let primary = json!({
        "agent": " writer ",
        "subagent_type": "ignored",
        "prompt": " draft ",
        "task": "ignored"
    });

    assert_eq!(tool.agent_name_from_args(&primary), Some("writer"));
    assert_eq!(tool.prompt_from_args(&primary), Some("draft"));
    assert_eq!(tool.agent_name_from_args(&json!({"agent": 7})), None);
    assert_eq!(tool.prompt_from_args(&json!({"prompt": false})), None);
    assert!(tool.run_in_background(&json!({"run_in_background": true})));
    assert!(tool.run_in_background(&json!({"task": "background"})));
    assert!(!tool.run_in_background(&json!({"prompt": "foreground"})));

    for status in ["running", "completed", "failed", "killed"] {
        let args = json!({"status": status});
        assert_eq!(tool.normalized_status_filter(&args).unwrap(), Some(status));
    }
    assert_eq!(tool.normalized_status_filter(&json!({"status": "  "})).unwrap(), None);
    assert_eq!(tool.normalized_status_filter(&json!({"status": 1})).unwrap(), None);
}

#[test]
fn blank_primary_fields_do_not_fall_back_to_compatibility_aliases() {
    let (tool, _) = build_tool();
    let args = json!({
        "agent": "   ",
        "subagent_type": "writer",
        "prompt": "\t",
        "task": "draft"
    });

    assert_eq!(tool.agent_name_from_args(&args), None);
    assert_eq!(tool.prompt_from_args(&args), None);

    let typed_aliases = json!({
        "agent": 7,
        "subagent_type": " writer ",
        "prompt": false,
        "task": " draft "
    });
    assert_eq!(tool.agent_name_from_args(&typed_aliases), Some("writer"));
    assert_eq!(tool.prompt_from_args(&typed_aliases), Some("draft"));
}

#[test]
fn spec_and_schema_describe_configured_agents() {
    let (tool, _) = build_tool();

    assert_eq!(tool.name(), "AgentTool");
    assert!(tool.description().contains("AgentTool"));
    assert!(
        tool.parameters_schema()["properties"]["agent"]["description"]
            .as_str()
            .unwrap()
            .contains("writer")
    );

    let spec = tool.spec();
    assert_eq!(spec.id, "AgentTool");
    assert!(spec.aliases.contains(&"agent".to_string()));
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn schema_is_closed_and_agent_description_is_stable() {
    let registry = Arc::new(SubAgentRegistry::new());
    let agents = HashMap::from([
        ("zeta".to_string(), DelegateAgentConfig::default()),
        ("alpha".to_string(), DelegateAgentConfig::default()),
    ]);
    let tool = build_tool_with_agents(agents, Arc::clone(&registry), SecurityPolicy::default());

    assert_eq!(tool.available_agents_description(), "alpha, zeta");
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!(["launch", "run", "spawn", "list", "get", "stop", "kill"])
    );
    assert_eq!(
        schema["properties"]["status"]["enum"],
        json!(["running", "completed", "failed", "killed", "all"])
    );

    let empty = build_tool_with_agents(HashMap::new(), registry, SecurityPolicy::default());
    assert_eq!(empty.available_agents_description(), "（未配置）");
    assert!(
        empty.parameters_schema()["properties"]["agent"]["description"]
            .as_str()
            .unwrap()
            .contains("（未配置）")
    );
}

#[tokio::test]
async fn execute_rejects_missing_required_parameters_before_delegation() {
    let (tool, _) = build_tool();

    let missing_agent =
        tool.execute(json!({"action": "launch", "prompt": "draft"})).await.unwrap_err();
    assert!(missing_agent.to_string().contains("Missing 'agent' parameter"));

    let missing_prompt =
        tool.execute(json!({"action": "launch", "agent": "writer"})).await.unwrap_err();
    assert!(missing_prompt.to_string().contains("Missing 'prompt' parameter"));

    let missing_get_session = tool.execute(json!({"action": "get"})).await.unwrap_err();
    assert!(missing_get_session.to_string().contains("Missing 'session_id' parameter"));

    let missing_stop_session = tool.execute(json!({"action": "stop"})).await.unwrap_err();
    assert!(missing_stop_session.to_string().contains("Missing 'session_id' parameter"));
}

#[tokio::test]
async fn launch_forwards_unknown_agent_failures_without_creating_sessions() {
    let (tool, registry) = build_tool();

    let foreground = tool
        .execute(json!({
            "action": "launch",
            "agent": "missing",
            "prompt": "draft",
            "run_in_background": false
        }))
        .await
        .unwrap();
    assert!(!foreground.success);
    assert!(foreground.error.unwrap().contains("Unknown agent 'missing'"));

    let background = tool
        .execute(json!({"action": "launch", "agent": "missing", "task": "draft"}))
        .await
        .unwrap();
    assert!(!background.success);
    assert!(background.error.unwrap().contains("Unknown agent 'missing'"));
    assert!(registry.list(None).is_empty());
}

#[tokio::test]
async fn get_serializes_running_and_failed_session_shapes() {
    let (tool, registry) = build_tool();
    insert_session(&registry, "running", SubAgentStatus::Running, None);
    insert_session(
        &registry,
        "failed",
        SubAgentStatus::Failed,
        Some(ToolResult {
            success: false,
            output: "partial output".to_string(),
            error: Some("provider rejected request".to_string()),
        }),
    );

    let running = tool.execute(json!({"action": "get", "session_id": "running"})).await.unwrap();
    let running_body: serde_json::Value = serde_json::from_str(&running.output).unwrap();
    assert_eq!(running_body["status"], "running");
    assert_eq!(running_body["duration_ms"], serde_json::Value::Null);
    assert!(running_body.get("completed_at").is_none());
    assert!(running_body.get("result").is_none());
    assert_eq!(running_body["title"], "Draft");
    assert_eq!(running_body["metadata"], json!({"topic": "coverage"}));

    let failed = tool.execute(json!({"action": "get", "session_id": "failed"})).await.unwrap();
    let failed_body: serde_json::Value = serde_json::from_str(&failed.output).unwrap();
    assert_eq!(failed_body["status"], "failed");
    assert_eq!(failed_body["duration_ms"], 12);
    assert!(failed_body.get("completed_at").is_some());
    assert_eq!(failed_body["result"]["success"], false);
    assert_eq!(failed_body["result"]["output"], "partial output");
    assert_eq!(failed_body["result"]["error"], "provider rejected request");
}

#[tokio::test]
async fn get_clamps_negative_session_duration_to_zero() {
    let (tool, registry) = build_tool();
    let started_at = Utc::now();
    registry.insert(SubAgentSession {
        id: "reversed-time".to_string(),
        agent_name: "writer".to_string(),
        title: None,
        task: "inspect timestamps".to_string(),
        metadata: serde_json::Value::Null,
        status: SubAgentStatus::Completed,
        started_at,
        updated_at: started_at,
        completed_at: Some(started_at - Duration::milliseconds(1)),
        result: None,
        #[cfg(not(target_arch = "wasm32"))]
        handle: None,
    });

    let result =
        tool.execute(json!({"action": "get", "session_id": "reversed-time"})).await.unwrap();
    let body: serde_json::Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(body["duration_ms"], 0);
    assert_eq!(body["status"], "completed");
    assert!(body.get("completed_at").is_some());
}

#[tokio::test]
async fn list_get_and_stop_cover_registry_paths() {
    let (tool, registry) = build_tool();
    insert_session(
        &registry,
        "done",
        SubAgentStatus::Completed,
        Some(ToolResult { success: true, output: "x".repeat(520), error: None }),
    );
    insert_session(&registry, "running", SubAgentStatus::Running, None);

    let list = tool.execute(json!({"action": "list", "status": "completed"})).await.unwrap();
    assert!(list.success);
    assert!(list.output.contains("\"session_id\": \"done\""));
    assert!(!list.output.contains("\"session_id\": \"running\""));

    let get = tool.execute(json!({"action": "get", "session_id": "done"})).await.unwrap();
    assert!(get.success);
    let body: serde_json::Value = serde_json::from_str(&get.output).unwrap();
    assert_eq!(body["session_id"], "done");
    assert_eq!(body["result"]["success"], true);
    assert!(body["result"]["output"].as_str().unwrap().ends_with("... (truncated)"));

    let missing = tool.execute(json!({"action": "get", "session_id": "missing"})).await.unwrap();
    assert!(!missing.success);
    assert_eq!(missing.error.as_deref(), Some("Unknown session 'missing'"));

    let stopped = tool.execute(json!({"action": "stop", "session_id": "running"})).await.unwrap();
    assert!(stopped.success);
    let stopped_body: serde_json::Value = serde_json::from_str(&stopped.output).unwrap();
    assert_eq!(stopped_body["status"], "killed");

    let stopped_again =
        tool.execute(json!({"action": "stop", "session_id": "running"})).await.unwrap();
    assert!(!stopped_again.success);
    assert!(stopped_again.error.unwrap().contains("is not running"));
}

#[tokio::test]
async fn stop_reports_unknown_or_read_only_sessions() {
    let registry = Arc::new(SubAgentRegistry::new());
    insert_session(&registry, "running", SubAgentStatus::Running, None);

    let mut read_only = SecurityPolicy::default();
    read_only.autonomy = AutonomyLevel::ReadOnly;
    let tool = build_tool_with_registry(Arc::clone(&registry), read_only);

    let denied = tool.execute(json!({"action": "stop", "session_id": "running"})).await.unwrap();
    assert!(!denied.success);
    assert!(denied.error.unwrap().contains("read-only mode"));

    let (tool, _) = build_tool();
    let unknown = tool.execute(json!({"action": "stop", "session_id": "missing"})).await.unwrap();
    assert!(!unknown.success);
    assert_eq!(unknown.error.as_deref(), Some("Unknown session 'missing'"));
}
