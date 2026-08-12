use super::*;
use crate::app::agent::config::{self, Config};
use crate::app::agent::security::SecurityPolicy;
use crate::tools::traits::Tool;
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::TempDir;

fn test_tool(tmp: &TempDir) -> ModelRoutingConfigTool {
    let config = Config {
        workspace_dir: tmp.path().to_path_buf(),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    ModelRoutingConfigTool::new(Arc::new(config), Arc::new(SecurityPolicy::default()))
}

async fn seed_project(tmp: &TempDir, patch: Value) {
    config::update_project(tmp.path(), patch).await.unwrap();
}

#[tokio::test]
async fn upsert_agent_sets_optional_fields_and_allowlists() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(&tmp);

    let result = tool
        .handle_upsert_agent(&json!({
            "name": " researcher ",
            "provider": " openai ",
            "model": " gpt-test ",
            "system_prompt": " investigate ",
            "api_key": " secret ",
            "temperature": 2.0,
            "max_depth": 4,
            "max_iterations": 7,
            "agentic": true,
            "allowed_tools": [" web_search ", "file_read"],
            "allowed_skills": "research, summarize",
            "tool_policy": "strict",
            "skill_policy": "user_override"
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    let agent = &output["config"]["agents"]["researcher"];
    assert_eq!(agent["provider"], "openai");
    assert_eq!(agent["model"], "gpt-test");
    assert_eq!(agent["system_prompt"], "investigate");
    assert_eq!(agent["api_key_configured"], true);
    assert_eq!(agent["temperature"], 2.0);
    assert_eq!(agent["max_depth"], 4);
    assert_eq!(agent["max_iterations"], 7);
    assert_eq!(agent["agentic"], true);
    assert_eq!(agent["allowed_tools"], json!(["web_search", "file_read"]));
    assert_eq!(agent["allowed_skills"], json!(["research", "summarize"]));
    assert_eq!(agent["tool_policy"], "strict");
    assert_eq!(agent["skill_policy"], "user_override");
    let stored = config::get_project_blocking(tmp.path()).unwrap();
    assert!(stored.agents["researcher"].allow_as_subagent);
    assert_eq!(stored.agents["researcher"].tool_policy, DelegateCapabilityPolicy::Strict);
    assert_eq!(
        stored.agents["researcher"].skill_policy,
        DelegateCapabilityPolicy::UserOverride
    );
}

#[test]
fn upsert_agent_schema_exposes_independent_capability_policies() {
    let tmp = TempDir::new().unwrap();
    let schema = test_tool(&tmp).parameters_schema();

    assert_eq!(
        schema["properties"]["tool_policy"]["enum"],
        json!(["strict", "user_override", "unrestricted"])
    );
    assert_eq!(
        schema["properties"]["skill_policy"]["enum"],
        json!(["strict", "user_override", "unrestricted"])
    );
}

#[tokio::test]
async fn upsert_agent_nulls_restore_optional_and_limit_defaults() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "agents": {
                "writer": {
                    "provider": "old-provider",
                    "model": "old-model",
                    "system_prompt": "old prompt",
                    "api_key": "old-key",
                    "temperature": 0.8,
                    "max_depth": 8,
                    "max_iterations": 12
                }
            }
        }),
    )
    .await;
    let tool = test_tool(&tmp);

    let result = tool
        .handle_upsert_agent(&json!({
            "name": "writer",
            "provider": "new-provider",
            "model": "new-model",
            "system_prompt": null,
            "api_key": null,
            "temperature": null,
            "max_depth": null,
            "max_iterations": null
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    let agent = &output["config"]["agents"]["writer"];
    assert!(agent["system_prompt"].is_null());
    assert_eq!(agent["api_key_configured"], false);
    assert!(agent["temperature"].is_null());
    assert_eq!(agent["max_depth"], DEFAULT_AGENT_MAX_DEPTH);
    assert_eq!(agent["max_iterations"], DEFAULT_AGENT_MAX_ITERATIONS);
}

#[tokio::test]
async fn upsert_agent_rejects_invalid_limits() {
    for (args, expected) in [
        (
            json!({"name": "a", "provider": "p", "model": "m", "temperature": 2.01}),
            "'temperature' must be between 0.0 and 2.0",
        ),
        (
            json!({"name": "a", "provider": "p", "model": "m", "max_depth": 0}),
            "'max_depth' must be greater than 0",
        ),
        (
            json!({"name": "a", "provider": "p", "model": "m", "max_iterations": 0}),
            "'max_iterations' must be greater than 0",
        ),
    ] {
        let tmp = TempDir::new().unwrap();
        let error = test_tool(&tmp).handle_upsert_agent(&args).await.unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
}

#[tokio::test]
async fn remove_agent_rejects_an_unknown_name() {
    let tmp = TempDir::new().unwrap();

    let error = test_tool(&tmp).handle_remove_agent(&json!({"name": "missing"})).await.unwrap_err();

    assert_eq!(error.to_string(), "No delegate agent found with name 'missing'");
}
