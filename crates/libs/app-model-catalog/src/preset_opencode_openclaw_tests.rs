use super::{
    preset_catalog::{opencode_openclaw_preset, opencode_openclaw_presets},
    preset_opencode_openclaw::OpenCodeOpenClawPreset,
};
use std::collections::HashSet;
use vw_api_types::agent_apps::AppId::{self, OpenClaw, OpenCode};

fn preset(app: AppId, name: &str) -> &'static OpenCodeOpenClawPreset {
    opencode_openclaw_preset(app, name).unwrap_or_else(|| panic!("missing preset {app:?}/{name}"))
}

#[test]
fn catalog_presets_opencode_openclaw_separates_endpoint_metadata() {
    let id = "deepseek-v4.1-flash";
    let code_url = "https://api.tokenfactory.cn/v1";
    let claw_url = "https://api.moark.com/v1";
    let window = |app, name, endpoint| {
        opencode_openclaw_preset(app, name)
            .and_then(|row| row.model_at_endpoint(endpoint, id))
            .and_then(|model| model.context_window)
    };
    assert_eq!(
        (window(OpenCode, "TokenFactory", code_url), window(OpenClaw, "模力方舟", claw_url)),
        (Some(1_048_576), Some(1_000_000)),
    );
    assert_eq!(window(OpenCode, "TokenFactory", claw_url), None);
    assert_eq!(window(OpenClaw, "模力方舟", code_url), None);
}

#[test]
fn catalog_presets_opencode_openclaw_keeps_same_host_paths_separate() {
    let id = "ark-code-latest";
    let agent_url = "https://ark.cn-beijing.volces.com/api/plan/v3";
    let coding_url = "https://ark.cn-beijing.volces.com/api/coding/v3";
    for (app, context) in [(OpenCode, None), (OpenClaw, Some(256_000))] {
        let agent = preset(app, "火山 Agent Plan");
        let coding = preset(app, "火山 Coding Plan");
        assert_eq!(agent.model_at_endpoint(agent_url, id).unwrap().context_window, context);
        assert_eq!(coding.model_at_endpoint(coding_url, id).unwrap().context_window, context);
        assert!(agent.model_at_endpoint(coding_url, id).is_none());
        assert!(coding.model_at_endpoint(agent_url, id).is_none());
    }
}

#[test]
fn catalog_presets_opencode_openclaw_preserves_opencode_limits_and_modalities() {
    let mimo = preset(OpenCode, "Xiaomi MiMo");
    let endpoint = "https://api.xiaomimimo.com/v1";
    assert_eq!(mimo.npm.as_deref(), Some("@ai-sdk/openai-compatible"));
    for id in ["mimo-v2.6-pro", "mimo-v2.6-flash"] {
        let model = mimo.model_at_endpoint(endpoint, id).unwrap();
        assert_eq!(model.context_window, Some(1_048_576));
        assert_eq!(model.max_output_tokens, Some(131_072));
        assert_eq!(model.input_modalities.as_deref().unwrap(), ["text", "image"]);
        assert_eq!(model.output_modalities.as_deref().unwrap(), ["text"]);
    }
    let qwen = preset(OpenCode, "QwenCloud");
    let endpoint = qwen.base_url.as_deref().unwrap();
    assert_eq!(
        qwen.model_at_endpoint(endpoint, "qwen3.8-max")
            .unwrap()
            .input_modalities
            .as_deref()
            .unwrap(),
        ["text", "image", "video"],
    );
    assert!(qwen.model_at_endpoint(endpoint, "qwen3.7-max").unwrap().input_modalities.is_none());
}

#[test]
fn catalog_presets_opencode_openclaw_preserves_openclaw_model_refs_and_unknowns() {
    let endpoint = "https://api.therouter.ai/v1";
    let claw = preset(OpenClaw, "TheRouter");
    let code = preset(OpenCode, "TheRouter");
    assert_eq!(claw.api.as_deref(), Some("openai-completions"));
    let id = "anthropic/claude-sonnet-5-5";
    let model = claw.model_at_endpoint(endpoint, id).unwrap();
    assert_eq!(model.context_window, Some(1_000_000));
    assert_eq!(model.max_output_tokens, Some(128_000));
    assert_eq!(model.input_modalities.as_deref().unwrap(), ["text", "image"]);
    assert!(model.output_modalities.is_none());
    assert!(claw.model_at_endpoint(endpoint, "claude-sonnet-5-5").is_none());
    assert!(code.model_at_endpoint(endpoint, id).unwrap().context_window.is_none());
    let defaults = claw.suggested_defaults.as_ref().unwrap();
    let refs = defaults.model.as_ref().unwrap();
    assert_eq!(refs.primary, "therouter/anthropic/claude-sonnet-5-5");
    assert_eq!(
        refs.fallbacks,
        ["therouter/openai/gpt-6-astra", "therouter/google/gemini-3.8-flash"]
    );
    assert_eq!(defaults.model_catalog[&refs.primary].alias.as_deref(), Some("Sonnet"));
}

#[test]
fn catalog_presets_opencode_openclaw_lists_every_source_preset_and_model() {
    for (app, preset_count, model_count) in [(OpenCode, 110, 338), (OpenClaw, 122, 383)] {
        let rows: Vec<_> = opencode_openclaw_presets(app).collect();
        assert_eq!(rows.len(), preset_count);
        assert_eq!(rows.first().unwrap().name, "Kimi");
        assert_eq!(rows.last().unwrap().name, "AIComing");
        assert_eq!(rows.iter().map(|row| row.models.len()).sum::<usize>(), model_count);
        assert_eq!(rows.iter().map(|row| &row.name).collect::<HashSet<_>>().len(), preset_count);
        for row in rows {
            assert_eq!(row.app_id, app);
            assert_eq!(
                row.models.iter().map(|model| &model.id).collect::<HashSet<_>>().len(),
                row.models.len()
            );
        }
    }
    let bedrock = preset(OpenCode, "AWS Bedrock");
    assert_eq!(bedrock.npm.as_deref(), Some("@ai-sdk/amazon-bedrock"));
    assert!(bedrock.models.iter().any(|model| model.id == "global.anthropic.claude-opus-5"));
    assert!(bedrock.model_at_endpoint("", "global.anthropic.claude-opus-5").is_none());
    assert!(opencode_openclaw_preset(OpenClaw, "Oh My OpenCode").is_none());
    assert!(opencode_openclaw_presets(AppId::Codex).next().is_none());
}
