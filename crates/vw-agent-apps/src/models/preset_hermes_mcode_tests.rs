use super::{
    preset_catalog::{hermes_mcode_preset, hermes_mcode_presets},
    preset_hermes_mcode::{HermesMcodePreset, McodeCompatValue},
};
use std::collections::HashSet;
use vw_api_types::agent_apps::AppId::{self, Hermes, Mcode};

fn preset(app: AppId, name: &str) -> &'static HermesMcodePreset {
    hermes_mcode_preset(app, name).unwrap_or_else(|| panic!("missing {app:?} preset {name}"))
}

#[test]
fn catalog_presets_hermes_mcode_keeps_application_window_fields() {
    let window = |app, name, url, id| {
        hermes_mcode_preset(app, name)
            .and_then(|preset| preset.model_at_endpoint(url, id))
            .and_then(|model| model.context_window)
    };
    assert_eq!(
        (
            window(Hermes, "Kimi", "https://api.moonshot.cn/v1", "kimi-k3.1-pro"),
            window(Mcode, "MiniMax", "https://api.minimax.cn/v1", "MiniMax-M3.1-Flash"),
        ),
        (Some(1_048_576), Some(1_000_000)),
    );
    let hermes = preset(Hermes, "MiniMax");
    let flash =
        hermes.model_at_endpoint("https://api.minimax.cn/v1", "MiniMax-M3.1-Flash").unwrap();
    assert_eq!(flash.context_window, None);
    assert_eq!(flash.max_output_tokens, None);
    assert_eq!(flash.supports_vision, Some(true));
    assert!(flash.input_modalities.is_none());
    assert!(flash.output_modalities.is_none());
    assert_eq!(hermes.provider_key.as_deref(), Some("cc-switch-minimax"));
    let default = hermes.suggested_model.as_ref().unwrap();
    assert_eq!(default.provider, hermes.provider_key);
    assert_eq!(default.model_id, flash.id);
}

#[test]
fn catalog_presets_hermes_mcode_keeps_hermes_deployments_separate() {
    let agent = preset(Hermes, "火山 Agent Plan");
    let id = "ark-code-latest";
    let model = agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/plan", id).unwrap();
    assert_eq!(model.context_window, None);
    assert!(agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/coding", id).is_none());
    assert!(agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/plan/v3", id).is_none());
    assert!(hermes_mcode_preset(Hermes, "OpenAI API").is_none());
}

#[test]
fn catalog_presets_hermes_mcode_keeps_mcode_deployments_separate() {
    let agent = preset(Mcode, "火山 Agent Plan");
    let id = "ark-code-latest";
    let model =
        agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/plan/v3", id).unwrap();
    assert_eq!(model.context_window, Some(128_000));
    assert_eq!(model.max_output_tokens, Some(16_384));
    assert_eq!(model.input_modalities.as_deref().unwrap(), ["text"]);
    assert_eq!(model.output_modalities.as_deref().unwrap(), ["text"]);
    assert!(
        agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/coding/v3", id).is_none()
    );
    assert!(agent.model_at_endpoint("https://ark.cn-beijing.volces.com/api/plan", id).is_none());
    assert!(hermes_mcode_preset(Mcode, "Nous Research").is_none());
}

#[test]
fn catalog_presets_hermes_mcode_retains_filtered_mcode_models_and_metadata() {
    let kimi = preset(Mcode, "Kimi");
    let model = kimi.model_at_endpoint("https://api.moonshot.cn/v1", "kimi-k2.7-code").unwrap();
    assert_eq!(model.input_modalities.as_deref().unwrap(), ["text", "image"]);
    assert!(kimi.model_at_endpoint("https://api.moonshot.cn/v1", "kimi-k3").is_none());
    assert!(hermes_mcode_preset(Mcode, "AWS Bedrock").is_none());
    let packy = preset(Mcode, "PackyCode");
    let opus = packy.model_at_endpoint("https://www.packyapi.ai", "claude-opus-5-5").unwrap();
    assert_eq!(
        opus.compat.as_ref().unwrap()["forceAdaptiveThinking"],
        McodeCompatValue::Flag(true)
    );
    assert_eq!(opus.effort_options.as_deref().unwrap(), ["low", "medium", "high", "xhigh", "max"]);
    let openai = preset(Mcode, "OpenAI API");
    assert_eq!(openai.api, "openai-responses");
    assert!(openai.model_at_endpoint("https://api.openai.com/v1", "gpt-6.1-sol").is_some());
    assert!(openai.model_at_endpoint("https://api.openai.com/v1", "openai/gpt-6.1-sol").is_none());
}

#[test]
fn catalog_presets_hermes_mcode_lists_all_source_rows_in_application_order() {
    for (app, preset_count, model_count, last) in
        [(Hermes, 121, 400, "AIComing"), (Mcode, 115, 406, "OpenAI API")]
    {
        let rows: Vec<_> = hermes_mcode_presets(app).collect();
        assert_eq!(rows.len(), preset_count);
        assert_eq!(rows.first().unwrap().name, "Kimi");
        assert_eq!(rows.last().unwrap().name, last);
        assert_eq!(rows.iter().map(|row| row.models.len()).sum::<usize>(), model_count);
        assert_eq!(rows.iter().map(|row| &row.name).collect::<HashSet<_>>().len(), preset_count);
        for row in rows {
            assert_eq!(row.app_id, app);
            assert_eq!(
                row.models.iter().map(|model| &model.id).collect::<HashSet<_>>().len(),
                row.models.len(),
            );
        }
    }
    assert!(hermes_mcode_presets(AppId::Pi).next().is_none());
}
