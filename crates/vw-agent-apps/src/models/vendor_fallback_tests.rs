use super::metadata::resolve_model_metadata;
use serde_json::{Value, json};
use std::time::{Duration, UNIX_EPOCH};
use vw_api_types::{
    agent_apps::AppId::Codex,
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelMetadataDto,
        ModelMetadataField::{ContextWindow, Cost, MaxOutputTokens},
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
    },
};
use vw_provider_resolver::models::ModelsSnapshot;

const RELAY: &str = "https://relay.example.test/v1";
const VENDOR: &str = "https://api.vendor.example.test/v1";

fn snapshot(providers: Value) -> ModelsSnapshot {
    ModelsSnapshot {
        providers: serde_json::from_value(providers).unwrap(),
        fetched_at: Some(UNIX_EPOCH + Duration::from_secs(42)),
    }
}

fn resolve(data: &ModelsSnapshot, endpoint: &str, id: &str) -> Option<ModelMetadataDto> {
    resolve_model_metadata(Codex, endpoint, id, Some(data)).map(|result| result.metadata)
}

fn gpt_catalog() -> Value {
    json!({
        "openai": {"api": VENDOR, "models": {
            "gpt-6-astra": {
                "limit": {"context": 1_050_000, "output": 128_000},
                "reasoning": true, "tool_call": true,
                "reasoning_options": [{"type": "effort", "values": [" HIGH ", "high", "low"]}],
                "modalities": {"input": ["text", "image"], "output": ["text"]},
                "cost": {"input": 2, "output": 8, "cache_read": 1, "cache_write": 3}
            },
            "gpt-4o": {"limit": {"context": 128_000}},
            "o9-big": {"limit": {"context": 1_050_000}},
            "agpt-large": {"limit": {"context": 1_050_000}}
        }},
        "relay": {"api": RELAY, "models": {
            "openai/gpt-6-astra": {"canonical_model_id": "openai/gpt-6-astra"}
        }}
    })
}

#[test]
fn catalog_metadata_vendor_omits_vendor_price() {
    let data = snapshot(gpt_catalog());
    let metadata = resolve(&data, "https://other.example.test", "openai/GPT-6-Astra:relay")
        .unwrap_or_default();
    assert_eq!((metadata.cost, metadata.context_window), (None, Some(272_000)));
    assert!(!metadata.provenance.contains_key(&Cost));
    assert_eq!(metadata.max_output_tokens, Some(128_000));
    assert_eq!(metadata.reasoning, Supported);
    assert_eq!(metadata.tool_calls, Supported);
    assert_eq!(metadata.reasoning_efforts.unwrap(), ["high", "low"]);
    assert_eq!(metadata.input_modalities.unwrap(), ["text", "image"]);
    assert_eq!(metadata.output_modalities.unwrap(), ["text"]);
    assert_eq!(
        metadata.provenance[&ContextWindow],
        ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(42_000)) }
    );
}

#[test]
fn catalog_metadata_vendor_fills_missing_fields_and_preserves_same_endpoint_values() {
    let mut data = gpt_catalog();
    data["relay"]["models"]["openai/gpt-6-astra"]["limit"] = json!({"context": 800_000});
    data["relay"]["models"]["openai/gpt-6-astra"]["tool_call"] = json!(false);
    data["relay"]["models"]["openai/gpt-6-astra"]["cost"] = json!({"input": 0, "output": 1});
    let metadata = resolve(&snapshot(data), RELAY, "gpt-6-astra").unwrap();
    assert_eq!(metadata.context_window, Some(800_000));
    assert_eq!(metadata.max_output_tokens, Some(128_000));
    assert_eq!(metadata.tool_calls, Unsupported);
    let cost = metadata.cost.unwrap();
    assert_eq!(
        (cost.input, cost.output, cost.cache_read, cost.cache_write),
        (0.0, 1.0, None, None)
    );
    assert_eq!(metadata.provenance[&Cost].source, ModelsDev);
}

#[test]
fn catalog_metadata_vendor_caps_a_borrowed_window_on_a_known_endpoint() {
    let data = snapshot(gpt_catalog());
    let metadata = resolve(&data, RELAY, "gpt-6-astra").unwrap_or_default();
    assert_eq!(metadata.context_window, Some(272_000));
    assert!(metadata.cost.is_none());
}

#[test]
fn catalog_metadata_vendor_keeps_the_full_same_endpoint_window_and_price() {
    let metadata = resolve(&snapshot(gpt_catalog()), VENDOR, "gpt-6-astra").unwrap();
    assert_eq!(metadata.context_window, Some(1_050_000));
    assert_eq!(metadata.cost.unwrap().input, 2.0);
}

#[test]
fn catalog_metadata_vendor_keeps_the_same_endpoint_preset_window() {
    let data = snapshot(json!({
        "openai": {"models": {"gpt-6.1-sol": {
            "limit": {"context": 1_050_000, "output": 128_000}
        }}},
        "relay": {"models": {"openai-gpt-6-1-sol": {"canonical_model_id": "openai/gpt-6.1-sol"}}}
    }));
    let metadata = resolve(&data, "https://inference.do-ai.run/v1", "openai-gpt-6-1-sol").unwrap();
    assert_eq!(metadata.context_window, Some(1_050_000));
    assert_eq!(metadata.provenance[&ContextWindow].source, Preset);
    assert_eq!(metadata.max_output_tokens, Some(128_000));
    assert_eq!(metadata.provenance[&MaxOutputTokens].source, ModelsDev);
    assert_eq!(resolve(&data, RELAY, "openai-gpt-6-1-sol").unwrap().context_window, Some(272_000));
}

#[test]
fn catalog_metadata_vendor_keeps_smaller_and_non_gpt_borrowed_windows() {
    let data = snapshot(gpt_catalog());
    for (id, expected) in [("gpt-4o", 128_000), ("o9-big", 1_050_000), ("agpt-large", 1_050_000)] {
        assert_eq!(resolve(&data, RELAY, id).unwrap_or_default().context_window, Some(expected));
    }
}

#[test]
fn catalog_metadata_vendor_prefers_exact_canonical_targets_over_owned_names() {
    let data = snapshot(json!({
        "relay": {"models": {
            "vendor/model": {"canonical_model_id": "vendor/series/model-v2"},
            "other/model": {"canonical_model_id": "vendor/series/model-v2"},
            "other-model": {"canonical_model_id": "other/other-model"}
        }},
        "vendor": {"models": {
            "series/model-v2": {"limit": {"context": 100}},
            "model": {"limit": {"context": 200}}
        }},
        "other": {"models": {"model": {"limit": {"context": 300}}}}
    }));
    assert_eq!(resolve(&data, RELAY, "model").unwrap_or_default().context_window, Some(100));
}

#[test]
fn catalog_metadata_vendor_rejects_conflicting_canonical_targets() {
    let data = snapshot(json!({
        "relay": {"models": {
            "one/model": {"canonical_model_id": "one/model"},
            "two/model": {"canonical_model_id": "two/model"}
        }},
        "one": {"models": {"model": {"limit": {"context": 100}}}},
        "two": {"models": {"model": {"limit": {"context": 200}}}}
    }));
    assert!(resolve(&data, RELAY, "model").is_none());
}

#[test]
fn catalog_metadata_vendor_rejects_conflicting_owned_candidates() {
    let data = snapshot(json!({
        "relay": {"models": {
            "one/marker": {"canonical_model_id": "one/marker"},
            "two/marker": {"canonical_model_id": "two/marker"}
        }},
        "one": {"models": {"model": {"limit": {"context": 100}}}},
        "two": {"models": {"model": {"limit": {"context": 200}}}}
    }));
    assert!(resolve(&data, RELAY, "model").is_none());
}

#[test]
fn catalog_metadata_vendor_ignores_hosted_rows_with_dangling_pointers() {
    let data = snapshot(json!({
        "relay": {"models": {
            "vendor/model": {"canonical_model_id": "vendor/model"},
            "host/marker": {"canonical_model_id": "host/marker"}
        }},
        "vendor": {"models": {"MODEL": {"limit": {"context": 100}}}},
        "host": {"models": {"model": {
            "canonical_model_id": "vendor/model", "limit": {"context": 200}
        }}}
    }));
    assert_eq!(resolve(&data, RELAY, "model").unwrap_or_default().context_window, Some(100));
    let data = snapshot(json!({
        "relay": {"models": {"host/marker": {"canonical_model_id": "host/marker"}}},
        "host": {"models": {"model": {
            "canonical_model_id": "vendor/model", "limit": {"context": 200}
        }}}
    }));
    assert!(resolve(&data, RELAY, "model").is_none());
}

#[test]
fn catalog_metadata_vendor_uses_an_owned_row_with_a_dangling_self_pointer() {
    let data = snapshot(json!({
        "vendor": {"models": {"model": {
            "canonical_model_id": "vendor/model-v2", "limit": {"context": 100}
        }}}
    }));
    assert_eq!(resolve(&data, RELAY, "model").unwrap_or_default().context_window, Some(100));
}

#[test]
fn catalog_metadata_vendor_excludes_the_local_overlay_and_price_only_results() {
    let data = snapshot(json!({
        "relay": {"models": {
            "shiju/model": {"canonical_model_id": "shiju/model"},
            "vendor/price-only": {"canonical_model_id": "vendor/price-only"}
        }},
        "shiju": {"models": {
            "model": {"limit": {"context": 100}},
            "vendor/unverified": {"canonical_model_id": "vendor/unverified"}
        }},
        "vendor": {"models": {"price-only": {"cost": {"input": 1, "output": 2}}}},
        "unverified": {"models": {"model": {"limit": {"context": 200}}}}
    }));
    assert!(resolve(&data, RELAY, "model").is_none());
    assert!(resolve(&data, RELAY, "price-only").is_none());

    let data = snapshot(json!({
        "shiju": {"models": {"vendor/model": {"canonical_model_id": "vendor/model"}}},
        "vendor": {"models": {"model": {"limit": {"context": 100}}}}
    }));
    assert!(resolve(&data, RELAY, "model").is_none());
}
