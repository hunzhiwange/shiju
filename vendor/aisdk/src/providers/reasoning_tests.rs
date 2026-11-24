use crate::core::language_model::{LanguageModelOptions, ReasoningEffort};
use serde_json::{Value, json};

#[test]
fn reasoning_effort_serializes_exactly_for_both_openai_protocols() {
    for effort in [
        ReasoningEffort::None,
        ReasoningEffort::Minimal,
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
        ReasoningEffort::Max,
    ] {
        let options = LanguageModelOptions {
            reasoning_effort: Some(effort),
            ..Default::default()
        };
        let response: super::openai::client::OpenAILanguageModelOptions =
            options.try_into().unwrap();
        let body = serde_json::to_value(response).unwrap();
        assert_eq!(body["reasoning"]["effort"], effort.as_str());
        let options = LanguageModelOptions {
            reasoning_effort: Some(effort),
            ..Default::default()
        };
        let chat: super::openai_chat_completions::client::ChatCompletionsOptions =
            options.try_into().unwrap();
        let body = serde_json::to_value(chat).unwrap();
        assert_eq!(body["reasoning_effort"], effort.as_str());
    }
}

#[test]
fn reasoning_effort_anthropic_uses_native_effort_without_fabricated_budgets() {
    for effort in [
        ReasoningEffort::Low,
        ReasoningEffort::Medium,
        ReasoningEffort::High,
        ReasoningEffort::Xhigh,
        ReasoningEffort::Max,
    ] {
        let options = LanguageModelOptions {
            reasoning_effort: Some(effort),
            ..Default::default()
        };
        let request: super::anthropic::client::AnthropicOptions = options.try_into().unwrap();
        let body = serde_json::to_value(request).unwrap();
        assert_eq!(body["output_config"], json!({"effort": effort.as_str()}));
        assert_eq!(body.get("thinking"), None);
    }
    let request: super::anthropic::client::AnthropicOptions = LanguageModelOptions {
        reasoning_effort: Some(ReasoningEffort::None),
        ..Default::default()
    }
    .try_into()
    .unwrap();
    let body: Value = serde_json::to_value(request).unwrap();
    assert_eq!(body["thinking"]["type"], "disabled");
    assert!(body.get("output_config").is_none());
}
