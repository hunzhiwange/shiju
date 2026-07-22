use super::*;
use serde_json::json;

fn tool() -> ModelRoutingConfigTool {
    ModelRoutingConfigTool::new(Arc::new(Config::default()), Arc::new(SecurityPolicy::default()))
}

#[test]
fn model_routing_config_schema_lists_every_supported_action() {
    let schema = tool().parameters_schema();
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!([
            "get",
            "list_hints",
            "set_default",
            "upsert_scenario",
            "remove_scenario",
            "upsert_agent",
            "remove_agent"
        ])
    );
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[tokio::test]
async fn unknown_action_returns_validation_error_without_loading_config() {
    let error = tool()
        .execute(json!({"action": "not_an_action"}))
        .await
        .expect_err("unknown actions must fail validation");

    let message = error.to_string();
    assert!(message.contains("Unknown action 'not_an_action'"));
    assert!(message.contains("list_hints"));
}
