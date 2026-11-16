//! Public specification and input validation, without persistent host services.
use super::*;
use super::input_test_support::InputHost;
use std::sync::atomic::{AtomicBool, Ordering};

type AgentTool = super::AgentTool<InputHost>;

fn build_tool_with_agents(agents: HashMap<String, DelegateAgentConfig>) -> AgentTool {
    AgentTool::new(agents, None, std::path::PathBuf::new())
}

fn build_tool() -> AgentTool {
    build_tool_with_agents(HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]))
}

#[test]
fn agent_tool_spec_allows_isolated_child_sessions_to_run_concurrently() {
    let spec = build_tool().spec();

    assert!(!spec.read_only);
    assert!(spec.concurrency_safe);
}

#[test]
fn action_validation_accepts_launch_aliases_and_rejects_legacy_actions() {
    let tool = build_tool();

    for (args, expected) in [
        (json!({}), AgentToolAction::Launch),
        (json!({"action": "launch"}), AgentToolAction::Launch),
        (json!({"action": "run"}), AgentToolAction::Run),
        (json!({"action": " SPAWN "}), AgentToolAction::Launch),
        (json!({"action": "status"}), AgentToolAction::Status),
        (json!({"action": "wait"}), AgentToolAction::Wait),
        (json!({"action": "stop"}), AgentToolAction::Stop),
    ] {
        assert_eq!(
            tool.action_from_args(&args).expect("supported action should be accepted"),
            expected
        );
    }

    for action in ["list", "get", "kill", "message", "dance"] {
        let err = tool
            .action_from_args(&json!({"action": action}))
            .expect_err("legacy or unknown action should fail");
        assert!(err.to_string().contains("Unknown AgentTool action"));
    }
}

#[test]
fn argument_helpers_trim_and_validate_agent_alias() {
    let tool = build_tool();
    let args = json!({
        "subagent_type": " writer ",
        "prompt": " draft "
    });

    assert_eq!(tool.agent_name_from_args(&args), Some("writer"));
    assert_eq!(tool.prompt_from_args(&args), Some("draft"));
}

#[test]
fn argument_helpers_cover_primary_fields_and_types() {
    let tool = build_tool();
    let primary = json!({
        "agent": " writer ",
        "subagent_type": "ignored",
        "prompt": " draft "
    });

    assert_eq!(tool.agent_name_from_args(&primary), Some("writer"));
    assert_eq!(tool.prompt_from_args(&primary), Some("draft"));
    assert_eq!(tool.agent_name_from_args(&json!({"agent": 7})), None);
    assert_eq!(tool.prompt_from_args(&json!({"prompt": false})), None);
}

#[test]
fn blank_primary_fields_do_not_fall_back_to_aliases() {
    let tool = build_tool();
    let args = json!({
        "agent": "   ",
        "subagent_type": "writer",
        "prompt": "\t"
    });

    assert_eq!(tool.agent_name_from_args(&args), None);
    assert_eq!(tool.prompt_from_args(&args), None);

    let typed_aliases = json!({
        "agent": 7,
        "subagent_type": " writer ",
        "prompt": false
    });
    assert_eq!(tool.agent_name_from_args(&typed_aliases), Some("writer"));
    assert_eq!(tool.prompt_from_args(&typed_aliases), None);
}

#[test]
fn spec_and_schema_describe_configured_agents() {
    let tool = build_tool_with_agents(HashMap::from([(
        "writer".to_string(),
        DelegateAgentConfig {
            label: Some("Writer".to_string()),
            description: Some("Draft and revise documents".to_string()),
            ..DelegateAgentConfig::default()
        },
    )]));

    assert_eq!(tool.name(), "AgentTool");
    let description = tool.description();
    for expected in ["complex", "simple", "parallel", "synthesize"] {
        assert!(
            description.contains(expected),
            "AgentTool description should explain {expected}: {description}"
        );
    }
    for unsupported in ["action=list", "action=get"] {
        assert!(
            !description.contains(unsupported),
            "AgentTool description must not advertise unsupported '{unsupported}': {description}"
        );
    }

    let schema = tool.parameters_schema();
    let agent_description =
        schema["properties"]["agent"]["description"].as_str().expect("agent description");
    for expected in ["writer", "Writer", "Draft and revise documents"] {
        assert!(
            agent_description.contains(expected),
            "agent description should contain {expected}: {agent_description}"
        );
    }
    assert_eq!(schema["properties"]["agent"]["enum"], json!(["writer"]));
    assert_eq!(schema["properties"]["subagent_type"]["enum"], json!(["writer"]));
    assert!(
        schema["properties"]["prompt"]["description"]
            .as_str()
            .expect("prompt description")
            .contains("validation")
    );

    let spec = tool.spec();
    assert_eq!(spec.id, "AgentTool");
    assert!(spec.aliases.contains(&"agent".to_string()));
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn default_agent_tool_schema_uses_chinese_role_catalog() {
    let agents = config_agent::merged_agent_configs(&HashMap::new())
        .into_iter()
        .filter(|(_, config)| config.enabled && config.allow_as_subagent)
        .collect();
    let tool = build_tool_with_agents(agents);

    let schema = tool.parameters_schema();
    assert_eq!(
        schema["properties"]["agent"]["enum"],
        json!(["coder", "explore", "researcher", "reviewer"])
    );

    let description =
        schema["properties"]["agent"]["description"].as_str().expect("agent description");
    for expected in ["代码实现", "代码探索", "资料研究", "代码审查", "只读审查代码、测试与风险"]
    {
        assert!(description.contains(expected), "schema should contain {expected}: {description}");
    }
}

#[test]
fn schema_is_closed_and_agent_description_is_stable() {
    let agents = HashMap::from([
        ("zeta".to_string(), DelegateAgentConfig::default()),
        ("alpha".to_string(), DelegateAgentConfig::default()),
    ]);
    let tool = build_tool_with_agents(agents);

    assert_eq!(tool.available_agents_description(), "alpha, zeta");
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!([
            "launch",
            "run",
            "spawn",
            "status",
            "wait",
            "stop",
            "send_message",
            "followup_task",
            "list_agents"
        ])
    );
    for removed_property in ["task", "run_in_background", "session_id", "status"] {
        assert!(
            schema["properties"].get(removed_property).is_none(),
            "legacy property '{removed_property}' must not be exposed"
        );
    }

    let empty = build_tool_with_agents(HashMap::new());
    assert_eq!(empty.available_agents_description(), "(not configured)");
    assert!(
        empty.parameters_schema()["properties"]["agent"]["description"]
            .as_str()
            .unwrap()
            .contains("(not configured)")
    );
}

#[tokio::test]
async fn execute_rejects_missing_required_parameters_before_delegation() {
    let tool = build_tool();

    let missing_agent =
        tool.execute(json!({"action": "launch", "prompt": "draft"})).await.unwrap_err();
    assert!(missing_agent.to_string().contains("Missing 'agent' parameter"));

    let missing_prompt =
        tool.execute(json!({"action": "launch", "agent": "writer"})).await.unwrap_err();
    assert!(missing_prompt.to_string().contains("Missing 'prompt' parameter"));
}

#[tokio::test]
async fn launch_reports_unknown_agent() {
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_in_tool = callback_called.clone();
    let tool = build_tool().with_child_session_created_callback(Arc::new(move |_created| {
        callback_called_in_tool.store(true, Ordering::SeqCst);
    }));

    let result = tool
        .execute(json!({"action": "launch", "agent": "missing", "prompt": "draft"}))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown agent 'missing'"));
    assert!(!callback_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn launch_without_parent_does_not_report_running_child() {
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_in_tool = callback_called.clone();
    let tool = build_tool().with_child_session_created_callback(Arc::new(move |_created| {
        callback_called_in_tool.store(true, Ordering::SeqCst);
    }));

    let error = tool
        .execute(json!({"action": "launch", "agent": "writer", "prompt": "draft"}))
        .await
        .expect_err("missing parent should fail");

    assert!(error.to_string().contains("current parent session ID"));
    assert!(!callback_called.load(Ordering::SeqCst));
}
