use super::{
    pi_thinking::{self, PiThinkingLevel},
    preset_catalog::{omp_preset, pi_preset},
    preset_pi::{
        PiPresetConfig,
        PiPresetProtocol::{self, *},
    },
};
use serde_json::{Value, json};
use vw_api_types::agent_apps::AppId;

fn model(provider: &str, endpoint: &str, protocol: PiPresetProtocol, id: &str) -> Value {
    let preset = pi_preset(AppId::Pi, provider).unwrap();
    let model = preset.settings_config.model_at_deployment(endpoint, protocol, id).unwrap();
    serde_json::to_value(model).unwrap()
}

#[test]
fn catalog_pi_thinking_rejects_incompatible_binding() {
    let id = "gpt-6.1-sol";
    let responses =
        model("cc-switch-right-code", "https://www.rightapi.ai/codex/v1", OpenaiResponses, id);
    let completions =
        model("cc-switch-teamo-router", "https://api.teamorouter.cn/v1", OpenaiCompletions, id);
    assert_eq!(
        (completions.get("thinkingLevelMap"), responses.get("thinkingLevelMap")),
        (
            None,
            Some(&json!({
                "off": "none", "minimal": null, "low": "low", "medium": "medium",
                "high": "high", "xhigh": "xhigh", "max": "max",
            })),
        )
    );
}

#[test]
fn catalog_pi_thinking_keeps_local_overrides_at_their_deployment() {
    let id = "glm-5.3";
    let go =
        model("cc-switch-open-code-go", "https://opencode.ai/zen/go/v1", OpenaiCompletions, id);
    assert_eq!(
        go["thinkingLevelMap"],
        json!({
            "off": null, "minimal": null, "low": null, "medium": null,
            "high": "high", "xhigh": null, "max": "max",
        })
    );
    let tokenfactory =
        model("cc-switch-tokenfactory", "https://api.tokenfactory.cn/v1", OpenaiCompletions, id);
    assert!(tokenfactory.get("thinkingLevelMap").is_none());
}

#[test]
fn catalog_pi_thinking_distinguishes_null_from_an_absent_level() {
    let deepseek = model(
        "cc-switch-deep-seek",
        "https://api.deepseek.com/v1",
        OpenaiCompletions,
        "deepseek-flash",
    );
    let map = &deepseek["thinkingLevelMap"];
    assert_eq!(map.get("minimal"), Some(&Value::Null));
    assert!(map.get("off").is_none());
    assert!(map.get("xhigh").is_none());
    assert_eq!(map["max"], "max");
}

#[test]
fn catalog_pi_thinking_pairs_adaptive_compat_with_the_anthropic_binding() {
    let anthropic = model(
        "cc-switch-aicoming",
        "https://api.aicoming.top",
        AnthropicMessages,
        "claude-opus-5-5",
    );
    assert_eq!(anthropic["thinkingLevelMap"], json!({"off": null, "xhigh": "xhigh", "max": "max"}));
    assert_eq!(anthropic["compat"]["forceAdaptiveThinking"], true);
    let bedrock = model(
        "cc-switch-aws-bedrock",
        "https://bedrock-runtime.us-east-1.amazonaws.com",
        BedrockConverseStream,
        "global.anthropic.claude-opus-5",
    );
    assert_eq!(bedrock["thinkingLevelMap"], json!({"xhigh": "xhigh", "max": "max"}));
    assert!(bedrock["compat"].get("forceAdaptiveThinking").is_none());
}

#[test]
fn catalog_pi_thinking_keeps_source_references_and_maps_out_of_omp() {
    let key = "cc-switch-aicoming";
    let endpoint = "https://api.aicoming.top";
    let id = "claude-opus-5-5";
    let pi = model(key, endpoint, AnthropicMessages, id);
    let omp = omp_preset(AppId::Omp, key).unwrap();
    let omp_model =
        omp.settings_config.model_at_deployment(endpoint, AnthropicMessages, id).unwrap();
    let omp = serde_json::to_value(omp_model).unwrap();
    assert!(pi["thinkingLevelMap"].is_object());
    assert!(pi.get("thinkingReference").is_none());
    assert!(omp.get("thinkingLevelMap").is_none());
    assert!(omp["compat"].get("forceAdaptiveThinking").is_none());
}

#[test]
fn catalog_pi_thinking_rejects_adaptive_compat_conflict() {
    let mut config: PiPresetConfig = serde_json::from_value(json!({
        "name": "fixture",
        "baseUrl": "https://example.test",
        "api": "anthropic-messages",
        "compat": { "forceAdaptiveThinking": false },
        "models": [
            {
                "id": "inherited",
                "name": "Claude Opus 5",
                "reasoning": true,
                "input": ["text"],
                "contextWindow": 1_000_000,
                "maxTokens": 128_000,
                "thinkingReference": { "catalogKey": "anthropic/claude-opus-5" },
            },
            {
                "id": "enabled",
                "name": "Claude Opus 5",
                "reasoning": true,
                "input": ["text"],
                "contextWindow": 1_000_000,
                "maxTokens": 128_000,
                "compat": { "forceAdaptiveThinking": true },
                "thinkingReference": { "catalogKey": "anthropic/claude-opus-5" },
            },
        ],
    }))
    .unwrap();
    config.apply_thinking_profiles();
    assert!(config.models[0].thinking_level_map().is_none());
    assert!(config.models[0].compat.is_none());
    assert!(config.models[1].thinking_level_map().is_some());
}

#[test]
fn catalog_pi_thinking_returns_independent_maps() {
    let preset = pi_preset(AppId::Pi, "cc-switch-kimi").unwrap();
    let model = preset
        .settings_config
        .model_at_deployment("https://api.moonshot.cn/v1", OpenaiCompletions, "kimi-k2.7-code")
        .unwrap();
    let mut draft = model.thinking_level_map().unwrap();
    draft.insert(PiThinkingLevel::Off, Some("none".into()));
    assert_eq!(draft[&PiThinkingLevel::Off].as_deref(), Some("none"));
    assert_eq!(model.thinking_level_map().unwrap()[&PiThinkingLevel::Off], None);
}

#[test]
fn catalog_pi_thinking_requires_the_exact_catalog_key() {
    let gemini =
        pi_thinking::resolve("google/gemini-3.1-pro-preview", GoogleGenerativeAi, None).unwrap();
    assert_eq!(gemini.map[&PiThinkingLevel::Low].as_deref(), Some("LOW"));
    assert_eq!(gemini.map[&PiThinkingLevel::High].as_deref(), Some("HIGH"));
    assert!(pi_thinking::resolve("gemini-3.1-pro-preview", GoogleGenerativeAi, None).is_none());
    assert!(pi_thinking::resolve("google/gemini-3.1-pro-preview", OpenaiResponses, None).is_none());
    assert!(pi_thinking::resolve("moonshotai/kimi-k3", OpenaiCompletions, None).is_none());
}
