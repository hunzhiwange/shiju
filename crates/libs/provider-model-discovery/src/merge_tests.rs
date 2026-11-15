use std::{collections::{HashMap, HashSet}, time::Duration};
use serde_json::json;
use crate::{merge_remote_models, test_support::*};

#[test]
fn remote_models_merge_preserves_explicit_status_and_static_metadata() {
    let mut provider = test_provider();
    let explicit = HashSet::from(["explicit-disabled".to_string()]);

    let merged = merge_remote_models(
        &mut provider,
        "https://connected.example/v1",
        &explicit,
        vec!["remote-new".to_string(), "static-model".to_string(), "explicit-disabled".to_string()],
        &HashMap::new(),
    );

    assert_eq!(merged, 2);
    assert_eq!(provider.models["explicit-disabled"].status, "disabled");
    assert_eq!(provider.models["static-model"].status, "active");
    assert_eq!(provider.models["static-model"].name, "Static static-model");
    assert!(provider.models["static-model"].capabilities.reasoning);
    let discovered = &provider.models["remote-new"];
    assert_eq!(discovered.status, "active");
    assert_eq!(discovered.api.url, "https://connected.example/v1");
    assert!(discovered.capabilities.input.text);
    assert!(discovered.capabilities.output.text);
    assert!(!discovered.capabilities.attachment);
    assert_eq!(discovered.cost.input, 0.0);
}

#[test]
fn shiju_discovery_inherits_catalog_metadata_without_foreign_routing_or_prices() {
    let catalog = serde_json::from_value(json!({
        "alibaba-cn": {"id":"alibaba-cn", "api":"https://foreign.example/v1",
            "models":{"known": {
                "id":"known", "name":"Known vision model", "family":"qwen",
                "reasoning":true, "reasoning_options":[{"type":"toggle"}],
                "attachment":true, "tool_call":false, "temperature":true,
                "modalities":{"input":["text","image","video"],"output":["text"]},
                "limit":{"context":1_000_000,"input":900_000,"output":100_000},
                "headers":{"Authorization":"foreign-secret"},
                "options":{"foreign_option":true},
                "cost":{"input":99,"output":99}, "status":"deprecated"
            }}}
    }))
    .unwrap();
    let mut provider = test_provider();
    provider.id = "shiju".into();
    let original = provider.models["static-model"].clone();
    let changed = merge_remote_models(
        &mut provider,
        "https://router.shiju.cloud/v1",
        &HashSet::from(["explicit-disabled".into()]),
        vec![
            "known".into(),
            "unknown".into(),
            "alias/known".into(),
            "static-model".into(),
            "explicit-disabled".into(),
        ],
        &catalog,
    );
    assert_eq!(changed, 4);
    let model = &provider.models["known"];
    assert_eq!(model.provider_id, "shiju");
    assert_eq!(model.api.id, "known");
    assert_eq!(model.api.url, "https://router.shiju.cloud/v1");
    assert_eq!(model.api.adapter, "openai-compatible");
    assert_eq!(model.name, "Known vision model");
    assert!(model.capabilities.reasoning);
    assert!(model.capabilities.input.image);
    assert!(model.capabilities.input.video);
    assert!(!model.capabilities.toolcall);
    assert_eq!(model.capabilities.reasoning_options.as_ref().unwrap().len(), 1);
    assert_eq!(model.limit.context, 1_000_000);
    assert_eq!(model.limit.input, Some(900_000));
    assert_eq!(model.limit.output, 100_000);
    assert!(model.headers.is_empty());
    assert!(model.options.is_empty());
    assert_eq!(model.cost.input, 0.0);
    assert_eq!(model.status, "active");
    assert_eq!(provider.models["explicit-disabled"].status, "disabled");
    assert_eq!(provider.models["static-model"].name, original.name);
    for id in ["unknown", "alias/known"] {
        assert!(!provider.models[id].capabilities.reasoning);
        assert_eq!(provider.models[id].limit.context, 0);
    }
}

#[test]
fn shiju_discovery_prefers_own_catalog_and_does_not_enrich_unrelated_providers() {
    let catalog = serde_json::from_value(json!({
        "shiju":{"models":{"known":{"id":"known","name":"Shiju metadata", "reasoning":false}}},
        "alibaba-cn":{"models":{"known":{"id":"known","name":"Other metadata", "reasoning":true}}}
    }))
    .unwrap();
    let mut provider = test_provider();
    merge_remote_models(
        &mut provider,
        "https://connected.example/v1",
        &HashSet::new(),
        vec!["known".into()],
        &catalog,
    );
    assert_eq!(provider.models["known"].name, "known");
    provider.id = "shiju".into();
    provider.models.remove("known");
    merge_remote_models(
        &mut provider,
        "https://router.shiju.cloud/v1",
        &HashSet::new(),
        vec!["known".into()],
        &catalog,
    );
    assert_eq!(provider.models["known"].name, "Shiju metadata");
    assert!(!provider.models["known"].capabilities.reasoning);
}

#[test]
fn model_protocol_010_discovery_requires_unambiguous_deployment() {
    let raw: provider_model_catalog::Provider = serde_json::from_value(json!({
        "id":"fixture", "api":"https://one.example/v1", "models":{
            "one":{"id":"one"},"two":{"id":"two", "provider":{"api":"https://two.example/v1"}}
        }
    }))
    .unwrap();
    let provider = vw_shared::provider::state::from_models_dev_provider(raw);
    assert_eq!(crate::resolve_api_base(&provider, None), None);
    assert_eq!(crate::resolve_api_base(&provider, Some("")), None);
    assert_eq!(
        crate::resolve_api_base(&provider, Some("https://explicit.example/v1/")),
        Some("https://explicit.example/v1".into())
    );
}

#[tokio::test]
async fn model_protocol_010_remote_discovery_preserves_explicit_protocols_and_status() {
    let server = serve_once(
        "200 OK",
        &[("Content-Type", "application/json")],
        r#"{"data":[{"id":"m"},{"id":"new"}]}"#,
        Duration::ZERO,
    );
    let raw: provider_model_catalog::Provider = serde_json::from_value(json!({
        "id":"fixture", "api":server.api_base, "npm":"@ai-sdk/openai-compatible",
        "models":{"m":{"id":"m","protocols":[]}}
    }))
    .unwrap();
    let mut provider = vw_shared::provider::state::from_models_dev_provider(raw);
    provider.models.get_mut("m").unwrap().status = "disabled".into();
    let explicit = HashSet::from(["m".to_string()]);
    assert_eq!(
        crate::discover_and_merge_provider(
            &mut provider,
            Some(&server.api_base),
            &explicit,
            "fixture-key"
        )
        .await
        .unwrap(),
        1
    );
    let request = server.request.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(request.starts_with("GET /models HTTP/1.1"));
    assert!(request.to_ascii_lowercase().contains("authorization: bearer fixture-key"));
    assert_eq!(provider.models["m"].status, "disabled");
    assert_eq!(provider.models["m"].api.protocols, Some(vec![]));
    assert_eq!(provider.models["new"].api.url, server.api_base);
    assert_eq!(provider.models["new"].api.protocols, None);
}

#[tokio::test]
async fn model_protocol_010_dedicated_sdk_skips_bearer_discovery() {
    let raw: provider_model_catalog::Provider = serde_json::from_value(json!({
        "id":"fixture", "npm":"@ai-sdk/amazon-bedrock",
        "api":"https://never-request.example", "models":{"m":{"id":"m"}}
    }))
    .unwrap();
    let mut provider = vw_shared::provider::state::from_models_dev_provider(raw);
    assert_eq!(
        crate::discover_and_merge_provider(
            &mut provider,
            None,
            &HashSet::new(),
            "fixture-key"
        )
        .await
        .unwrap(),
        0
    );
}
