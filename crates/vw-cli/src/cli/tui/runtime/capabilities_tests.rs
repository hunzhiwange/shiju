use super::*;
use serde_json::json;

#[test]
fn model_efforts_follow_each_models_protocol_and_declared_options() {
    for (provider, id, expected) in [
        (
            "shiju",
            "deepseek-v4-flash",
            vec![
                ReasoningEffort::Auto,
                ReasoningEffort::None,
                ReasoningEffort::High,
                ReasoningEffort::Max,
            ],
        ),
        (
            "token-plan",
            "deepseek-v4.1-flash-1017",
            vec![
                ReasoningEffort::Auto,
                ReasoningEffort::None,
                ReasoningEffort::Low,
                ReasoningEffort::High,
                ReasoningEffort::Max,
            ],
        ),
    ] {
        let io =
            json!({"text": true, "audio": false, "image": false, "video": false, "pdf": false});
        let mut model: vw_shared::provider::types::Model = serde_json::from_value(json!({
            "id": id, "providerID": provider, "name": id,
            "api": {"id": id, "adapter": "openai-compatible", "url": "https://example.com/v1"},
            "capabilities": {"temperature": true, "reasoning": true, "attachment": false,
                "toolcall": true, "input": io, "output": io, "interleaved": false},
            "cost": {"input": 0, "output": 0, "cache": {"read": 0, "write": 0}},
            "limit": {"context": 1000, "output": 1000}, "status": "active", "release_date": ""
        }))
        .expect("model fixture");
        assert_eq!(model_efforts(&model), expected);
        model.capabilities.reasoning_options =
            Some(vec![vw_shared::provider::reasoning::ReasoningOption::Effort {
                values: vec!["high".into()],
            }]);
        assert_eq!(model_efforts(&model), vec![ReasoningEffort::Auto, ReasoningEffort::High]);
        model.capabilities.reasoning = false;
        assert_eq!(model_efforts(&model), vec![ReasoningEffort::Auto]);
    }
}
