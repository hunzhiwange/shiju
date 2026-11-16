use crate::compaction_summary::{request_input, summary_error};
use model_types::ModelLimit;
use serde_json::{Value,json};
fn model_limit(context: u64, input: u64, output: u64) -> ModelLimit { ModelLimit { context, input: Some(input), output } }
#[test]
fn compaction_budget_reserves_output_and_accepts_unset_input_limit() {
    assert_eq!(request_input::input_budget(&model_limit(1000, 1000, 200)), 760);
    assert_eq!(request_input::input_budget(&model_limit(1000, 500, 200)), 475);
    assert_eq!(request_input::input_budget(&model_limit(1000, 0, 200)), 760);
    assert_eq!(request_input::input_budget(&model_limit(1000, 1000, 2000)), 712);
}

#[test]
fn compaction_empty_error_translation_preserves_metadata_and_other_failures() {
    let original = json!({"name":"APIError", "message":"模型未返回内容", "status_code":400,
        "is_retryable":false, "response_body":"original body", "metadata":{"replay_safe":"false", "source":"aisdk"}});
    let mut expected = original.clone();
    expected["message"] = json!("internal compaction generated an empty summary");
    let mapped: Value = serde_json::from_str(&summary_error(original.to_string())).unwrap();
    for key in ["message", "status_code", "is_retryable", "response_body", "metadata"] {
        assert_eq!(mapped[key], expected[key]);
    }
    for error in [
        "执行已取消".to_string(),
        "HTTP 401: 模型未返回内容".to_string(),
        json!({"name":"APIError","message":"upstream failed: 模型未返回内容","is_retryable":false})
            .to_string(),
    ] {
        assert_eq!(summary_error(error.clone()), error);
    }
}
