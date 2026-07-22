use super::*;
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
async fn upsert_scenario_enabling_classification_adds_a_default_keyword() {
    let tmp = TempDir::new().unwrap();

    let result = test_tool(&tmp)
        .handle_upsert_scenario(&json!({
            "hint": " coding ",
            "provider": " openai ",
            "model": " gpt-test ",
            "classification_enabled": true
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(output["config"]["scenarios"][0]["hint"], "coding");
    assert_eq!(output["config"]["scenarios"][0]["classification"]["keywords"], json!(["coding"]));
    assert_eq!(output["config"]["query_classification"]["enabled"], true);
}

#[tokio::test]
async fn upsert_scenario_nulls_clear_route_key_and_rule_bounds() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "model_routes": [{
                "hint": "coding",
                "provider": "old-provider",
                "model": "old-model",
                "api_key": "old-key"
            }],
            "query_classification": {
                "enabled": true,
                "rules": [{
                    "hint": "coding",
                    "keywords": ["code"],
                    "min_length": 10,
                    "max_length": 100,
                    "priority": 9
                }]
            }
        }),
    )
    .await;

    let result = test_tool(&tmp)
        .handle_upsert_scenario(&json!({
            "hint": "coding",
            "provider": "new-provider",
            "model": "new-model",
            "api_key": null,
            "min_length": null,
            "max_length": null,
            "priority": null
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    let scenario = &output["config"]["scenarios"][0];
    assert_eq!(scenario["api_key_configured"], false);
    assert!(scenario["classification"]["min_length"].is_null());
    assert!(scenario["classification"]["max_length"].is_null());
    assert_eq!(scenario["classification"]["priority"], 0);
    assert_eq!(scenario["classification"]["keywords"], json!(["code"]));
}

#[tokio::test]
async fn upsert_scenario_disabling_classification_removes_existing_rule() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "model_routes": [{"hint": "coding", "provider": "old", "model": "old"}],
            "query_classification": {
                "enabled": true,
                "rules": [{"hint": "coding", "keywords": ["code"]}]
            }
        }),
    )
    .await;

    let result = test_tool(&tmp)
        .handle_upsert_scenario(&json!({
            "hint": "coding",
            "provider": "new",
            "model": "new",
            "classification_enabled": false
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert!(output["config"]["scenarios"][0]["classification"].is_null());
    assert_eq!(output["config"]["query_classification"]["enabled"], false);
}

#[tokio::test]
async fn upsert_scenario_rejects_a_rule_without_matching_criteria() {
    let tmp = TempDir::new().unwrap();

    let error = test_tool(&tmp)
        .handle_upsert_scenario(&json!({
            "hint": "coding",
            "provider": "openai",
            "model": "gpt-test",
            "priority": 3
        }))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Classification rule for hint 'coding' has no matching criteria. Provide keywords/patterns or set min_length/max_length."
    );
}

#[tokio::test]
async fn remove_scenario_can_preserve_its_classification_rule() {
    let tmp = TempDir::new().unwrap();
    seed_project(
        &tmp,
        json!({
            "model_routes": [{"hint": "coding", "provider": "openai", "model": "gpt-test"}],
            "query_classification": {
                "enabled": true,
                "rules": [{"hint": "coding", "keywords": ["code"], "priority": 4}]
            }
        }),
    )
    .await;

    let result = test_tool(&tmp)
        .handle_remove_scenario(&json!({
            "hint": "coding",
            "remove_classification": false
        }))
        .await
        .unwrap();

    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(output["routes_removed"], 1);
    assert_eq!(output["classification_rules_removed"], 0);
    assert!(output["config"]["scenarios"].as_array().unwrap().is_empty());
    assert_eq!(output["config"]["classification_only_rules"][0]["hint"], "coding");
    assert_eq!(output["config"]["query_classification"]["enabled"], true);
}
