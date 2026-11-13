use super::target_model::resolve;
use serde_json::json;
use vw_api_types::agent_apps::AppId;

#[test]
fn cc_route_032_target_model_expected_behavior() {
    let settings = json!({"env": {
        "ANTHROPIC_DEFAULT_SONNET_MODEL": "mapped[1m]",
        "ANTHROPIC_DEFAULT_OPUS_MODEL": "opus-target",
        "ANTHROPIC_MODEL": "default"
    }});
    assert_eq!(resolve(AppId::Claude, &settings, "claude-sonnet-5[1M]"), "mapped");
    assert_eq!(resolve(AppId::Claude, &settings, "claude-fable-5"), "opus-target");
    assert_eq!(resolve(AppId::Claude, &settings, "unknown"), "default");
}

#[test]
fn cc_route_032_other_app_namespaces_preserve_models() {
    let settings = json!({"env": {"ANTHROPIC_MODEL": "wrong-app"}});
    for app in [AppId::GrokBuild, AppId::Codex, AppId::Gemini, AppId::ClaudeDesktop] {
        for model in ["ccs-other/model", "ccs-claude-other--model", "grok[1M]"] {
            assert_eq!(resolve(app, &settings, model), model);
        }
    }
}

#[test]
fn cc_route_032_suffix_without_mapping_and_unicode_are_safe() {
    for (input, expected) in [("模型[1m]  ", "模型"), ("x[1M]-y", "x[1M]-y"), ("模型", "模型")]
    {
        assert_eq!(resolve(AppId::Claude, &json!({}), input), expected);
    }
}

#[test]
fn cc_route_032_grok_aggregation_names_are_opaque() {
    use crate::aggregation::model_ids::{Decoded, decode};
    for model in ["ccs-other/model", "ccs-claude-other--model", "claude-fable-ccs-123456"] {
        assert_eq!(decode(AppId::GrokBuild, model), Ok(Decoded::Plain));
    }
}

#[test]
fn cc_route_032_subagent_does_not_fall_back_to_default() {
    let settings = json!({"env":{"CLAUDE_CODE_SUBAGENT_MODEL":"custom-subagent[1M]", "ANTHROPIC_MODEL":"default"}});
    assert_eq!(resolve(AppId::Claude, &settings, "custom-subagent"), "custom-subagent");
}

#[test]
fn cc_route_032_subagent_catalog_alias_uses_configured_upstream() {
    let settings = json!({"env":{"CLAUDE_CODE_SUBAGENT_MODEL":"anthropic/claude-opus-5.5[1M]", "ANTHROPIC_MODEL":"default"}});
    assert_eq!(
        resolve(AppId::Claude, &settings, "claude-opus-5-5[1M]"),
        "anthropic/claude-opus-5.5"
    );
}
