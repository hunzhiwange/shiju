use super::{
    fill_pi::{PiModelFill, fill_pi_model},
    metadata::{ResolvedModelMetadata, resolve_model_metadata},
};
use crate::providers::pi::{declarations, patch};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::pi::{PiActivation, PiProfile},
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState, ModelMetadataDto, ModelMetadataField::ReasoningEfforts,
        ModelMetadataProvenance, ModelMetadataSource,
    },
};

const MOONSHOT: &str = "https://api.moonshot.cn/v1";
const GO: &str = "https://opencode.ai/zen/go/v1";
const RELAY: &str = "https://relay.example.test/v1";
const COMPLETIONS: &str = "openai-completions";
const RESPONSES: &str = "openai-responses";

fn preset(endpoint: &str, id: &str) -> ResolvedModelMetadata {
    let metadata = resolve_model_metadata(AppId::Pi, endpoint, id, None).unwrap();
    assert!(metadata.pi_thinking.is_some());
    metadata
}

fn efforts(levels: &[&str]) -> ResolvedModelMetadata {
    ResolvedModelMetadata {
        metadata: ModelMetadataDto {
            reasoning: ModelCapabilityState::Supported,
            reasoning_efforts: Some(levels.iter().map(|level| (*level).into()).collect()),
            provenance: [(
                ReasoningEfforts,
                ModelMetadataProvenance {
                    source: ModelMetadataSource::ModelsDev,
                    fetched_at: Some(TimestampMs(123)),
                },
            )]
            .into(),
            ..Default::default()
        },
        pi_thinking: None,
    }
}

fn fill(
    app: AppId,
    provider_id: &str,
    declaration: Value,
    model: &Value,
    metadata: &ResolvedModelMetadata,
) -> PiModelFill {
    let profile = PiProfile { name: "Fixture".into(), declaration };
    fill_pi_model(app, provider_id, &profile, model.as_object().unwrap(), metadata)
}

#[test]
fn catalog_fill_pi_rejects_compat_conflict() {
    let mut metadata = preset(MOONSHOT, "kimi-k3.1-pro");
    // A separately known effort list cannot bypass a conflicting reviewed binding.
    metadata.metadata.reasoning_efforts = Some(vec!["high".into()]);
    let model = json!({"id":"kimi-k3.1-pro", "reasoning":true, "compat":{
        "supportsReasoningEffort":false, "keep":"custom"
    }});
    let original = model.clone();
    let result = fill(
        AppId::Pi,
        "fixture",
        json!({"api":COMPLETIONS,"baseUrl":MOONSHOT,"compat":{"supportsReasoningEffort":true}}),
        &model,
        &metadata,
    );
    assert_eq!(result, PiModelFill::default());
    assert_eq!(model, original);
}

#[test]
fn catalog_fill_pi_merges_only_missing_compat_and_preserves_native_row() {
    let metadata = preset(MOONSHOT, "kimi-k3.1-pro");
    let input = br#"{"providers":{"fixture":{
        "baseUrl":"https://api.moonshot.cn/v1","api":"openai-completions","compat":{
            "thinkingFormat":"openai","supportsReasoningEffort":false
        },
        "models":[{"id":"kimi-k3.1-pro","reasoning":true,"contextWindow":8000,
            "compat":{"supportsReasoningEffort":true,"keep":"custom"},"extra":{"keep":true}}]
    }}}"#;
    let mut profile = declarations(Some(input)).unwrap().remove("fixture").unwrap();
    let model = &profile.declaration["models"][0];
    let result =
        fill_pi_model(AppId::Pi, "fixture", &profile, model.as_object().unwrap(), &metadata);
    assert_eq!(
        result.changes["thinkingLevelMap"],
        json!(metadata.pi_thinking.as_ref().unwrap().levels)
    );
    assert_eq!(result.changes.len(), 1);
    assert_eq!(result.missing_compat.get("supportsReasoningEffort"), None);
    assert_eq!(result.missing_compat.get("thinkingFormat"), None);
    assert_eq!(result.missing_compat["requiresReasoningContentOnAssistantMessages"], true);
    assert_eq!(
        result.provenance,
        [(
            ReasoningEfforts,
            ModelMetadataProvenance { source: ModelMetadataSource::Preset, fetched_at: None }
        )]
        .into()
    );

    let row = profile.declaration["models"][0].as_object_mut().unwrap();
    row.extend(result.changes);
    row.get_mut("compat").unwrap().as_object_mut().unwrap().extend(result.missing_compat);
    let saved =
        patch(Some(input), "fixture", Some(&profile.declaration), PiActivation::IfMember).unwrap();
    let round_trip = declarations(Some(&saved)).unwrap().remove("fixture").unwrap();
    let model = &round_trip.declaration["models"][0];
    assert_eq!(model["contextWindow"], 8000);
    assert_eq!(model["extra"]["keep"], true);
    assert_eq!(model["compat"]["keep"], "custom");
    assert_eq!(model["compat"]["supportsReasoningEffort"], true);
    assert_eq!(model["compat"]["requiresReasoningContentOnAssistantMessages"], true);
    assert_eq!(
        fill_pi_model(AppId::Pi, "fixture", &round_trip, model.as_object().unwrap(), &metadata),
        PiModelFill::default()
    );
}

#[test]
fn catalog_fill_pi_rejects_different_deployment_or_protocol() {
    let metadata = preset(GO, "glm-5.3");
    let provider = json!({"api":COMPLETIONS, "baseUrl":GO});
    for model in [
        json!({"baseUrl":RELAY}),
        json!({"baseUrl":"https://opencode.ai/zen/go/v2"}),
        json!({"api":RESPONSES}),
    ] {
        assert_eq!(
            fill(AppId::Pi, "fixture", provider.clone(), &model, &metadata),
            PiModelFill::default()
        );
    }
    let provider = json!({"api":RESPONSES, "baseUrl":RELAY});
    let model = json!({"id":"glm-5.3", "api":COMPLETIONS, "baseUrl":GO});
    assert!(
        fill(AppId::Pi, "fixture", provider, &model, &metadata)
            .changes
            .contains_key("thinkingLevelMap")
    );
}

#[test]
fn catalog_fill_pi_checks_inferred_completions_behavior_for_presets() {
    let metadata = preset(GO, "glm-5.3");
    let provider = json!({"api":COMPLETIONS, "baseUrl":GO});
    let model = json!({"id":"glm-5.3"});
    assert!(
        fill(AppId::Pi, "fixture", provider.clone(), &model, &metadata)
            .changes
            .contains_key("thinkingLevelMap")
    );
    assert_eq!(fill(AppId::Pi, "zai", provider.clone(), &model, &metadata), PiModelFill::default());
    let model = json!({"id":"glm-5.3", "compat":{"supportsReasoningEffort":false}});
    assert_eq!(fill(AppId::Pi, "fixture", provider, &model, &metadata), PiModelFill::default());
}

#[test]
fn catalog_fill_pi_builds_response_map_and_reports_effort_source() {
    let metadata = efforts(&["none", "low", "max", "unknown"]);
    let result = fill(
        AppId::Pi,
        "zai",
        json!({"api":RESPONSES,"baseUrl":"https://api.z.ai/v1"}),
        &json!({"reasoning":true,"compat":{"supportsReasoningEffort":false}}),
        &metadata,
    );
    assert_eq!(
        result.changes["thinkingLevelMap"],
        json!({
            "off":"none", "minimal":null, "low":"low", "medium":null,
            "high":null, "xhigh":null, "max":"max"
        })
    );
    assert_eq!(result.provenance, metadata.metadata.provenance);
    assert!(result.missing_compat.is_empty());
}

#[test]
fn catalog_fill_pi_gates_efforts_on_completions_host_and_compat() {
    let metadata = efforts(&["high"]);
    let draft = json!({"reasoning":true});
    for endpoint in [
        "https://api.deepseek.com/v1",
        // Pi matches vendor markers anywhere in the URL.
        "https://deepseek.com.example.test/v1",
        "https://api.z.ai/v1",
        "https://openrouter.ai/api/v1",
        MOONSHOT,
    ] {
        assert_eq!(
            fill(
                AppId::Pi,
                "fixture",
                json!({"api":COMPLETIONS,"baseUrl":endpoint}),
                &draft,
                &metadata
            ),
            PiModelFill::default()
        );
    }
    let endpoint = "https://API.DeepSeek.COM/v1";
    let provider = json!({"api":COMPLETIONS,"baseUrl":endpoint});
    assert_eq!(
        fill(AppId::Pi, "fixture", provider.clone(), &draft, &metadata),
        PiModelFill::default()
    );
    let override_draft = json!({"reasoning":true,"compat":{"thinkingFormat":"openai"}});
    assert_eq!(
        fill(AppId::Pi, "fixture", provider, &override_draft, &metadata).changes["thinkingLevelMap"]
            ["high"],
        "high"
    );
    let unrelated_host = json!({"api":COMPLETIONS, "baseUrl":RELAY});
    assert_eq!(
        fill(AppId::Pi, "fixture", unrelated_host, &draft, &metadata).changes["thinkingLevelMap"]["off"],
        Value::Null
    );
}

#[test]
fn catalog_fill_pi_skips_unrecognized_efforts_and_non_effort_protocols() {
    let model = json!({"reasoning":true});
    for (protocol, levels) in [
        (RESPONSES, &["default", "off"][..]),
        ("anthropic-messages", &["high"][..]),
        ("unknown", &["high"][..]),
    ] {
        assert_eq!(
            fill(
                AppId::Pi,
                "fixture",
                json!({"api":protocol,"baseUrl":RELAY}),
                &model,
                &efforts(levels)
            ),
            PiModelFill::default()
        );
    }
}

#[test]
fn catalog_fill_pi_preserves_explicit_map_and_disabled_reasoning() {
    let metadata = preset(GO, "glm-5.3");
    for model in [
        json!({"reasoning":true,"thinkingLevelMap":{}}),
        json!({"reasoning":true,"thinkingLevelMap":{"off":null,"high":"custom"}}),
        json!({"reasoning":false}),
    ] {
        assert_eq!(
            fill(AppId::Pi, "fixture", json!({"api":COMPLETIONS,"baseUrl":GO}), &model, &metadata),
            PiModelFill::default()
        );
    }
}

#[test]
fn catalog_fill_pi_keeps_omp_out_of_both_map_paths() {
    let model = json!({"reasoning":true});
    let provider = json!({"api":COMPLETIONS,"baseUrl":GO});
    for metadata in [preset(GO, "glm-5.3"), efforts(&["high"])] {
        assert_eq!(
            fill(AppId::Omp, "fixture", provider.clone(), &model, &metadata),
            PiModelFill::default()
        );
    }
}
