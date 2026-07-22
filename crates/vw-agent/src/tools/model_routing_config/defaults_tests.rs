use super::ModelRoutingConfigTool;
use crate::app::agent::config::{self, Config};
use crate::app::agent::security::SecurityPolicy;
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
async fn handle_get_returns_the_project_routing_snapshot() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "default_temperature": 0.25,
            "model_routes": [{
                "hint": "coding",
                "provider": "openai",
                "model": "gpt-test",
                "max_tokens": 2048,
                "api_key": null
            }],
            "query_classification": {
                "enabled": true,
                "rules": [{
                    "hint": "coding",
                    "keywords": ["code"],
                    "patterns": ["```"],
                    "min_length": 10,
                    "max_length": 1000,
                    "priority": 50
                }]
            }
        }),
    )
    .await;
    let tool = test_tool(&tmp);

    let result = tool.handle_get().unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(output["default"]["temperature"], json!(0.25));
    assert_eq!(output["query_classification"]["enabled"], json!(true));
    assert_eq!(output["query_classification"]["rules_count"], json!(1));
    assert_eq!(output["scenarios"][0]["hint"], json!("coding"));
    assert_eq!(output["scenarios"][0]["provider"], json!("openai"));
    assert_eq!(output["scenarios"][0]["model"], json!("gpt-test"));
    assert_eq!(output["scenarios"][0]["api_key_configured"], json!(false));
    assert_eq!(output["scenarios"][0]["classification"]["priority"], json!(50));
}

#[tokio::test]
async fn handle_list_hints_sorts_and_deduplicates_each_hint_kind() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "model_routes": [
                {
                    "hint": "conversation",
                    "provider": "openai",
                    "model": "chat-a"
                },
                {
                    "hint": "coding",
                    "provider": "openai",
                    "model": "code-a"
                },
                {
                    "hint": "coding",
                    "provider": "openai",
                    "model": "code-b"
                }
            ],
            "query_classification": {
                "enabled": true,
                "rules": [
                    {"hint": "triage", "keywords": ["issue"], "priority": 10},
                    {"hint": "coding", "patterns": ["```"], "priority": 20},
                    {"hint": "triage", "keywords": ["bug"], "priority": 30}
                ]
            }
        }),
    )
    .await;
    let tool = test_tool(&tmp);

    let result = tool.handle_list_hints().unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(output["model_route_hints"], json!(["coding", "conversation"]));
    assert_eq!(output["classification_hints"], json!(["coding", "triage"]));
    assert_eq!(output["example"]["conversation"]["action"], json!("upsert_scenario"));
    assert_eq!(output["example"]["coding"]["classification_enabled"], json!(true));
}

#[test]
fn read_handlers_propagate_invalid_project_override_errors() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".vibewindow.json"), "{not-json").unwrap();
    let tool = test_tool(&tmp);

    assert!(tool.handle_get().is_err());
    assert!(tool.handle_list_hints().is_err());
}

#[tokio::test]
async fn set_default_requires_at_least_one_update() {
    let tmp = TempDir::new().unwrap();
    let error = test_tool(&tmp).handle_set_default(&json!({})).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "set_default requires at least one of: provider, model, temperature"
    );
}

#[tokio::test]
async fn set_default_rejects_wrong_field_types_before_loading_config() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(&tmp);

    let provider_error = tool.handle_set_default(&json!({"provider": 7})).await.unwrap_err();
    let model_error = tool.handle_set_default(&json!({"model": false})).await.unwrap_err();
    let temperature_error =
        tool.handle_set_default(&json!({"temperature": "warm"})).await.unwrap_err();

    assert_eq!(provider_error.to_string(), "'provider' must be a string or null");
    assert_eq!(model_error.to_string(), "'model' must be a string or null");
    assert_eq!(temperature_error.to_string(), "'temperature' must be a number or null");
}

#[tokio::test]
async fn set_default_updates_selected_strings_and_preserves_unset_fields() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "default_provider": "custom:https://seed.invalid",
            "default_model": "seed-model",
            "default_temperature": 1.25
        }),
    )
    .await;
    let tool = test_tool(&tmp);

    let provider_result = tool
        .handle_set_default(&json!({"provider": " custom:https://updated.invalid "}))
        .await
        .unwrap();
    assert!(provider_result.success);
    assert_eq!(provider_result.error, None);
    let provider_output: Value = serde_json::from_str(&provider_result.output).unwrap();
    assert_eq!(provider_output["message"], json!("Default provider/model settings updated"));
    assert_eq!(
        provider_output["config"]["default"]["provider"],
        json!("custom:https://updated.invalid")
    );
    assert_eq!(provider_output["config"]["default"]["model"], json!("seed-model"));
    assert_eq!(provider_output["config"]["default"]["temperature"], json!(1.25));

    let model_result = tool.handle_set_default(&json!({"model": " updated-model "})).await.unwrap();
    let model_output: Value = serde_json::from_str(&model_result.output).unwrap();
    assert_eq!(
        model_output["config"]["default"]["provider"],
        json!("custom:https://updated.invalid")
    );
    assert_eq!(model_output["config"]["default"]["model"], json!("updated-model"));
    assert_eq!(model_output["config"]["default"]["temperature"], json!(1.25));
}

#[tokio::test]
async fn set_default_nulls_restore_inherited_strings_and_default_temperature() {
    let tmp = TempDir::new().unwrap();
    let inherited = config::get_project_blocking(tmp.path()).unwrap();
    seed_project(
        &tmp,
        json!({
            "default_provider": "custom:https://seed.invalid",
            "default_model": "seed-model",
            "default_temperature": 1.25
        }),
    )
    .await;
    let tool = test_tool(&tmp);

    let result = tool
        .handle_set_default(&json!({
            "provider": "   ",
            "model": null,
            "temperature": null
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(
        output["config"]["default"]["provider"],
        serde_json::to_value(inherited.default_provider).unwrap()
    );
    assert_eq!(
        output["config"]["default"]["model"],
        serde_json::to_value(inherited.default_model).unwrap()
    );
    assert_eq!(
        output["config"]["default"]["temperature"],
        json!(Config::default().default_temperature)
    );
}

#[tokio::test]
async fn set_default_accepts_temperature_boundaries() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(&tmp);

    let minimum = tool.handle_set_default(&json!({"temperature": 0.0})).await.unwrap();
    let minimum_output: Value = serde_json::from_str(&minimum.output).unwrap();
    assert_eq!(minimum_output["config"]["default"]["temperature"], json!(0.0));

    let maximum = tool.handle_set_default(&json!({"temperature": 2.0})).await.unwrap();
    let maximum_output: Value = serde_json::from_str(&maximum.output).unwrap();
    assert_eq!(maximum_output["config"]["default"]["temperature"], json!(2.0));
}

#[tokio::test]
async fn set_default_rejects_temperatures_outside_the_supported_range() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(&tmp);

    let below_minimum = tool.handle_set_default(&json!({"temperature": -0.01})).await.unwrap_err();
    let above_maximum = tool.handle_set_default(&json!({"temperature": 2.01})).await.unwrap_err();

    assert_eq!(below_minimum.to_string(), "'temperature' must be between 0.0 and 2.0");
    assert_eq!(above_maximum.to_string(), "'temperature' must be between 0.0 and 2.0");
}
