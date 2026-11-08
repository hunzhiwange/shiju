use super::{
    metadata::resolve_model_metadata,
    pi_thinking::PiThinkingLevel,
    preset_pi::{PiPresetCompatValue, PiPresetProtocol},
};
use serde_json::{Value, json};
use std::time::{Duration, UNIX_EPOCH};
use vw_api_types::{
    agent_apps::AppId::*,
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::{Supported, Unknown, Unsupported},
        ModelMetadataField::*,
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
        ModelTokenCost,
    },
};
use vw_provider_resolver::models::ModelsSnapshot;

const FLUXA: &str = "https://api.baiduqianfan.ai/v2/tokenplan/team";

fn snapshot(providers: Value) -> ModelsSnapshot {
    ModelsSnapshot {
        providers: serde_json::from_value(providers).unwrap(),
        fetched_at: Some(UNIX_EPOCH + Duration::from_millis(1_234)),
    }
}

#[test]
fn catalog_metadata_endpoint_fills_only_missing_provider_fields() {
    let data = snapshot(json!({
        "fluxa": {
            "api": FLUXA,
            "models": {"glm-5.2": {
                "limit": {"context": 1_000_000, "output": 98_304},
                "reasoning": false, "tool_call": true,
                "reasoning_options": [{"type": "effort", "values": [" HIGH ", "high", ""]}],
                "modalities": {"output": ["Text", " text "]},
                "cost": {"input": 0, "output": 2, "cache_read": 0.1}
            }}
        }
    }));
    let metadata = resolve_model_metadata(Codex, FLUXA, "glm-5.2", Some(&data))
        .map(|resolved| resolved.metadata)
        .unwrap_or_default();
    assert_eq!(
        (metadata.context_window, metadata.max_output_tokens),
        (Some(500_000), Some(98_304))
    );
    assert_eq!(metadata.reasoning, Unsupported);
    assert_eq!(metadata.tool_calls, Supported);
    assert_eq!(metadata.reasoning_efforts.as_deref().unwrap(), ["high"]);
    assert_eq!(metadata.output_modalities.as_deref().unwrap(), ["text"]);
    assert_eq!(
        metadata.cost,
        Some(ModelTokenCost { input: 0.0, output: 2.0, cache_read: Some(0.1), cache_write: None })
    );
    assert_eq!(
        metadata.provenance[&ContextWindow],
        ModelMetadataProvenance { source: Preset, fetched_at: None }
    );
    for field in [MaxOutputTokens, Reasoning, ToolCalls, OutputModalities, Cost] {
        assert_eq!(
            metadata.provenance[&field],
            ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(1_234)) }
        );
    }
}

#[test]
fn catalog_metadata_endpoint_records_only_contributing_fields() {
    let data = snapshot(json!({
        "fluxa": {"api": FLUXA, "models": {"glm-5.2": {"limit": {"context": 1_000_000}}}}
    }));
    let without_snapshot = resolve_model_metadata(Codex, FLUXA, "glm-5.2", None).unwrap();
    let with_snapshot = resolve_model_metadata(Codex, FLUXA, "glm-5.2", Some(&data)).unwrap();
    assert_eq!(with_snapshot, without_snapshot);
    assert!(with_snapshot.metadata.provenance.values().all(|p| p.source == Preset));
    assert_eq!(with_snapshot.metadata.tool_calls, Unknown);
}

#[test]
fn catalog_metadata_endpoint_keeps_sibling_paths_ports_and_costs_separate() {
    let coding = "https://api.z.ai/api/coding/paas/v4";
    let standard = "https://api.z.ai/api/paas/v4";
    let data = snapshot(json!({
        "standard": {"api": standard, "models": {"model": {
            "limit": {"context": 200_000}, "cost": {"input": 1, "output": 2}
        }}},
        "coding": {"api": coding, "models": {"model": {
            "limit": {"context": 500_000}, "canonical_model_id": "standard/model"
        }}}
    }));
    let standard = resolve_model_metadata(Codex, standard, "model", Some(&data)).unwrap();
    assert_eq!(standard.metadata.cost.unwrap().input, 1.0);
    let coding = resolve_model_metadata(Codex, coding, "model", Some(&data)).unwrap();
    assert_eq!(coding.metadata.context_window, Some(500_000));
    assert!(coding.metadata.cost.is_none());
    assert!(!coding.metadata.provenance.contains_key(&Cost));
    for url in ["https://api.z.ai/api", "https://api.z.ai:8443/api/paas/v4"] {
        let fallback = resolve_model_metadata(Codex, url, "model", Some(&data)).unwrap().metadata;
        assert_eq!(fallback.context_window, Some(200_000));
        assert!(fallback.cost.is_none());
        assert!(!fallback.provenance.contains_key(&Cost));
    }
}

#[test]
fn catalog_metadata_endpoint_recognizes_aliases_and_request_suffixes() {
    let data = snapshot(json!({
        "go": {"api": "https://opencode.ai/inference/go/openai/v1", "models": {
            "vendor/model": {"limit": {"context": 1_000_000}}
        }},
        "zen": {"api": "https://opencode.ai/inference/openai/v1", "models": {
            "vendor/model": {"limit": {"context": 200_000}}
        }}
    }));
    for (url, window) in [
        ("https://opencode.ai/zen/go/v1beta/chat/completions/", 1_000_000),
        ("opencode.ai/zen/v1/responses", 200_000),
    ] {
        let found = resolve_model_metadata(OpenCode, url, " MODEL ", Some(&data)).unwrap();
        assert_eq!(found.metadata.context_window, Some(window));
    }
}

#[test]
fn catalog_metadata_endpoint_prefers_exact_model_before_normalized_ids() {
    let data = snapshot(json!({
        "vendor": {"api": "https://example.test", "models": {
            "relay/MODEL": {"limit": {"context": 10}},
            "Model": {"limit": {"context": 20}},
            "model": {"limit": {"context": 30}}
        }}
    }));
    let found =
        resolve_model_metadata(Codex, "https://example.test", "model", Some(&data)).unwrap();
    assert_eq!(found.metadata.context_window, Some(30));
    let folded =
        resolve_model_metadata(Codex, "https://example.test", "MODEL", Some(&data)).unwrap();
    assert_eq!(folded.metadata.context_window, Some(20));
}

#[test]
fn catalog_metadata_endpoint_rejects_invalid_and_empty_matches() {
    let data = snapshot(json!({
        "vendor": {"api": "https://example.test/v1", "models": {
            "empty": {"limit": {"context": 0}, "cost": {"input": 0}, "modalities": {"input": []}},
            "known": {"limit": {"context": 10}}
        }}
    }));
    for (url, model) in [
        ("https://example.test", "empty"),
        ("https://example.test", "  "),
        ("https://example.test", "missing"),
        ("", "known"),
        ("https://example.test?query=1", "known"),
        ("https://user:secret@example.test", "known"),
    ] {
        assert!(resolve_model_metadata(Codex, url, model, Some(&data)).is_none(), "{url}, {model}");
    }
}

#[test]
fn catalog_metadata_endpoint_preserves_explicit_tool_denial_and_unknown_fetch_time() {
    let mut data = snapshot(json!({
        "vendor": {"api": "https://example.test", "models": {
            "model": {"tool_call": false}
        }}
    }));
    data.fetched_at = None;
    let metadata = resolve_model_metadata(Codex, "https://example.test", "model", Some(&data))
        .unwrap()
        .metadata;
    assert_eq!(metadata.tool_calls, Unsupported);
    assert_eq!(metadata.provenance.len(), 1);
    assert_eq!(
        metadata.provenance[&ToolCalls],
        ModelMetadataProvenance { source: ModelsDev, fetched_at: None }
    );
}

#[test]
fn catalog_metadata_endpoint_projects_seven_app_presets_offline() {
    for (app, endpoint, model, window) in [
        (Codex, FLUXA, "glm-5.2", 500_000),
        (OpenCode, "https://api.moonshot.cn/v1", "kimi-k2.6", 262_144),
        (OpenClaw, "https://api.moonshot.cn/v1", "kimi-k3.1-pro", 1_048_576),
        (Hermes, "https://api.moonshot.cn/v1", "kimi-k3.1-pro", 1_048_576),
        (Mcode, "https://api.minimax.cn/v1", "MiniMax-M3.1-Flash", 1_000_000),
        (Pi, "https://api.aicoming.top", "claude-opus-5-5", 1_000_000),
        (Kimi, "https://api.moonshot.cn/v1", "kimi-k3.1-pro", 1_048_576),
    ] {
        let found = resolve_model_metadata(app, endpoint, model, None)
            .unwrap_or_else(|| panic!("missing {app:?}/{model}"));
        assert_eq!(found.metadata.context_window, Some(window), "{app:?}/{model}");
        assert_eq!(found.metadata.provenance[&ContextWindow].source, Preset);
    }
    for (app, window) in [(Codex, 1_000_000), (Mcode, 1_048_576)] {
        let resolved = resolve_model_metadata(app, FLUXA, "glm-5.3", None).unwrap();
        assert_eq!(resolved.metadata.context_window, Some(window));
    }
}

#[test]
fn catalog_metadata_endpoint_keeps_preset_price_as_one_field() {
    let url = "https://api.moonshot.cn/v1";
    let data = snapshot(json!({
        "moonshot": {"api": url, "models": {"kimi-k3": {
            "cost": {"input": 8, "output": 9, "cache_read": 0.2, "cache_write": 2}
        }}}
    }));
    let metadata = resolve_model_metadata(OpenClaw, url, "kimi-k3", Some(&data)).unwrap().metadata;
    assert_eq!(
        metadata.cost,
        Some(ModelTokenCost {
            input: 3.0,
            output: 15.0,
            cache_read: Some(0.3),
            cache_write: Some(0.0)
        })
    );
    assert_eq!(metadata.provenance[&Cost].source, Preset);
    assert!(
        resolve_model_metadata(
            OpenClaw,
            "https://api.moonshot.cn/coding/v1",
            "kimi-k3",
            Some(&data)
        )
        .is_none()
    );
}

#[test]
fn catalog_metadata_endpoint_keeps_pi_thinking_bound_to_reviewed_protocol() {
    let endpoint = "https://api.aicoming.top";
    let model = "claude-opus-5-5";
    let resolved = resolve_model_metadata(Pi, endpoint, model, None).unwrap();
    let thinking = resolved.pi_thinking.unwrap();
    assert_eq!(thinking.endpoint, endpoint);
    assert_eq!(thinking.protocol, PiPresetProtocol::AnthropicMessages);
    assert_eq!(thinking.levels[&PiThinkingLevel::Xhigh].as_deref(), Some("xhigh"));
    assert_eq!(thinking.compat.unwrap()["forceAdaptiveThinking"], PiPresetCompatValue::Flag(true));
    assert!(resolve_model_metadata(Pi, "https://api.aicoming.top/other", model, None).is_none());
}

#[test]
fn catalog_metadata_endpoint_rejects_ambiguous_deployments_sharing_an_address() {
    let endpoint = "https://api.minimax.io/anthropic/v1";
    let data = snapshot(json!({
        "minimax": {"api": endpoint, "models": {"model": {
            "limit": {"context": 200_000}, "cost": {"input": 0.3, "output": 1.2}
        }}},
        "minimax-coding-plan": {"api": endpoint, "models": {"model": {
            "limit": {"context": 100_000}, "cost": {"input": 0, "output": 0}
        }}}
    }));
    assert!(resolve_model_metadata(Codex, endpoint, "model", Some(&data)).is_none());
}
