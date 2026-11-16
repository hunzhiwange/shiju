//! Toolset domain contracts migrated from the executable host.
mod host;
mod security;
use crate::{toolset::*, agents::*, Tool, ToolResult, ToolCallError};
use config_agent::DelegateAgentConfig;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

struct EchoTool;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn description(&self) -> &str {
        "echo tool"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type": "object"})
    }
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: args.to_string(), error: None })
    }
}

fn agent_config(mode: &str, enabled: bool, allowed_skills: Vec<String>) -> DelegateAgentConfig {
    DelegateAgentConfig {
        runtime: Default::default(),
        recommended_prompts: Vec::new(),
        defaults: Default::default(),
        label: None,
        description: None,
        builtin: false,
        mode: mode.to_string(),
        enabled,
        allow_as_subagent: true,
        max_parallel_subagents: 4,
        provider: "ollama".to_string(),
        model: "llama3".to_string(),
        force_model_selection: false,
        system_prompt: None,
        api_key: None,
        temperature: None,
        top_p: None,
        identity_format: None,
        hidden: false,
        max_depth: 3,
        agentic: false,
        allowed_tools: Vec::new(),
        skill_policy: Default::default(),
        allowed_skills,
        enabled_identity_files: Vec::new(),
        options: HashMap::new(),
        permission: serde_json::Value::Null,
        max_iterations: 10,
        compact_context: false,
        max_history_messages: 100,
        parallel_tools: false,
        steps: None,
    }
}

#[test]
fn configured_subagent_opt_out_overrides_builtin_default() {
    let mut agents = HashMap::new();

    let mut blocked = agent_config("subagent", true, Vec::new());
    blocked.allow_as_subagent = false;
    agents.insert("researcher".to_string(), blocked);

    let mut disabled = agent_config("subagent", false, Vec::new());
    disabled.allow_as_subagent = true;
    agents.insert("coder".to_string(), disabled);

    let eligible = enabled_delegate_agents(&agents);

    assert!(!eligible.contains_key("main"));
    assert!(eligible.contains_key("reviewer"));
    assert!(!eligible.contains_key("researcher"));
    assert!(!eligible.contains_key("coder"));
}

#[test]
fn agent_tool_parallel_limit_uses_current_agent_and_clamps_bounds() {
    let mut agents = HashMap::new();
    let mut main = agent_config("primary", true, Vec::new());
    main.max_parallel_subagents = 3;
    agents.insert("main".to_string(), main);

    let mut worker = agent_config("subagent", true, Vec::new());
    worker.max_parallel_subagents = 99;
    agents.insert("worker".to_string(), worker);

    assert_eq!(agent_tool_parallel_limit(&agents, "main"), 3);
    assert_eq!(agent_tool_parallel_limit(&agents, "worker"), 32);
    assert_eq!(agent_tool_parallel_limit(&agents, "missing"), 4);
}

#[test]
fn classify_message_and_binary_detection_cover_errors_and_content() {
    assert!(matches!(classify_message("blocked".to_string()), ToolCallError::Denied { .. }));
    assert!(matches!(
        classify_message("not allowed here".to_string()),
        ToolCallError::Denied { .. }
    ));
    assert!(matches!(classify_message("forbidden path".to_string()), ToolCallError::Denied { .. }));
    assert!(matches!(
        classify_message("permission denied".to_string()),
        ToolCallError::Denied { .. }
    ));
    assert!(matches!(classify_message("failed".to_string()), ToolCallError::Failed(_)));

    let dir = tempfile::tempdir().unwrap();
    let text = dir.path().join("a.txt");
    let bin = dir.path().join("a.bin");
    let nul = dir.path().join("a.data");
    std::fs::write(&text, b"hello\n").unwrap();
    std::fs::write(&bin, b"text").unwrap();
    std::fs::write(&nul, b"a\0b").unwrap();
    assert!(!is_binary(&text));
    assert!(is_binary(&bin));
    assert!(is_binary(&nul));
    assert!(!is_binary(&dir.path().join("missing")));
}

#[tokio::test]
async fn arc_delegating_tool_forwards_methods() {
    let boxed = boxed_registry_from_arcs(vec![Arc::new(EchoTool)]).remove(0);
    assert_eq!(boxed.name(), "echo");
    assert_eq!(boxed.parameters_schema(), json!({"type": "object"}));
    assert_eq!(boxed.validate_input(json!({"x": 1})).unwrap(), json!({"x": 1}));
    assert!(boxed.execute(json!({"x": 1})).await.unwrap().success);
}

#[tokio::test]
async fn arc_delegating_tool_forwards_v2_contract_methods() {
    let boxed = boxed_registry_from_arcs(vec![Arc::new(EchoTool)]).remove(0);
    let input = json!({"x": 1});

    assert_eq!(boxed.description(), "echo tool");
    boxed.check_permissions(&input).await.unwrap();
    let called = boxed.call(input.clone()).await.unwrap();
    assert!(called.is_success());
    assert_eq!(boxed.map_result_for_model(&called), called.default_model_result());
    assert_eq!(boxed.render_hint(&called).unwrap().title.as_deref(), Some("echo"));
    assert!(!boxed.is_concurrency_safe());
    assert!(!boxed.is_read_only());
    assert_eq!(boxed.to_audit_input(&input), input);
    assert_eq!(boxed.spec().id, "echo");
}
