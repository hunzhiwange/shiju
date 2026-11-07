use crate::{TimestampMs, agent_apps::AppId, id::TenantScope, model_catalog::*};
use serde_json::{from_value, json, to_value};

#[test]
fn catalog_contract_roundtrip_preserves_unknown() {
    let metadata: ModelMetadataDto = from_value(json!({
        "reasoning": "unknown",
        "contextWindow": null
    }))
    .unwrap();
    let wire = to_value(&metadata).unwrap();
    assert_eq!(wire["reasoning"], "unknown");
    let decoded: ModelMetadataDto = from_value(wire).unwrap();
    assert_eq!(decoded.reasoning, ModelCapabilityState::Unknown);
    assert_eq!(decoded.context_window, None);

    let omitted: ModelMetadataDto = from_value(json!({})).unwrap();
    assert_eq!(omitted.reasoning, ModelCapabilityState::Unknown);
    assert_eq!(omitted.context_window, None);
}

fn identity() -> ScopedModelCatalogIdentity {
    ScopedModelCatalogIdentity {
        scope: TenantScope::new("platform", "company").unwrap(),
        deployment: ModelCatalogIdentity {
            target_host_id: "host-a".into(),
            app_id: AppId::Codex,
            provider_id: "provider".into(),
            endpoint: "https://example.invalid/plan/v1".into(),
        },
        model_id: "vendor/Exact-ID".into(),
    }
}

#[test]
fn catalog_contract_identity_uses_every_scope_component_and_excludes_alias() {
    let original = ModelCatalogEntry {
        identity: identity(),
        display_alias: Some("Friendly".into()),
        metadata: ModelMetadataDto::default(),
    };
    let wire = to_value(&original).unwrap();
    assert_eq!(wire["identity"]["modelId"], "vendor/Exact-ID");
    assert_eq!(wire["displayAlias"], "Friendly");

    let mut renamed = wire.clone();
    renamed["displayAlias"] = json!("New name");
    let renamed: ModelCatalogEntry = from_value(renamed).unwrap();
    assert_eq!(renamed.identity, original.identity);
    assert_ne!(renamed.display_alias, original.display_alias);

    for (path, value) in [
        ("/identity/scope/platform_id", "other-platform"),
        ("/identity/scope/company_id", "other-company"),
        ("/identity/deployment/targetHostId", "host-b"),
        ("/identity/deployment/appId", "pi"),
        ("/identity/deployment/providerId", "other-provider"),
        ("/identity/deployment/endpoint", "https://example.invalid/v1"),
        ("/identity/modelId", "vendor/exact-id"),
    ] {
        let mut changed = wire.clone();
        *changed.pointer_mut(path).unwrap() = json!(value);
        let changed: ModelCatalogEntry = from_value(changed).unwrap();
        assert_ne!(changed.identity, original.identity, "{path}");
    }
}

#[test]
fn catalog_contract_retains_known_values_and_field_provenance() {
    let metadata: ModelMetadataDto = from_value(json!({
        "contextWindow": 0,
        "reasoning": "unsupported",
        "toolCalls": "supported",
        "inputModalities": [],
        "provenance": {
            "contextWindow": {"source": "preset"},
            "toolCalls": {"source": "models-dev", "fetchedAt": 1730000000000_i64}
        }
    }))
    .unwrap();
    let decoded: ModelMetadataDto = from_value(to_value(metadata).unwrap()).unwrap();
    assert_eq!(decoded.context_window, Some(0));
    assert_eq!(decoded.max_output_tokens, None);
    assert_eq!(decoded.input_modalities, Some(vec![]));
    assert_eq!(decoded.output_modalities, None);
    assert_eq!(decoded.reasoning, ModelCapabilityState::Unsupported);
    assert_eq!(decoded.tool_calls, ModelCapabilityState::Supported);
    assert_eq!(
        decoded.provenance[&ModelMetadataField::ContextWindow].source,
        ModelMetadataSource::Preset
    );
    assert_eq!(
        decoded.provenance[&ModelMetadataField::ToolCalls],
        ModelMetadataProvenance {
            source: ModelMetadataSource::ModelsDev,
            fetched_at: Some(TimestampMs(1_730_000_000_000)),
        }
    );
}

#[test]
fn catalog_contract_fill_commit_requires_revision_for_exact_identity() {
    let preview = ModelCatalogFillPreviewResponse {
        identity: identity(),
        revision: "revision-5".into(),
        proposed: ModelMetadataDto { context_window: Some(65_536), ..Default::default() },
    };
    let preview: ModelCatalogFillPreviewResponse = from_value(to_value(preview).unwrap()).unwrap();
    let commit = ModelCatalogFillCommitRequest {
        identity: preview.identity,
        expected_revision: preview.revision,
    };
    let mut wire = to_value(&commit).unwrap();
    assert_eq!(from_value::<ModelCatalogFillCommitRequest>(wire.clone()).unwrap(), commit);
    assert!(wire.get("proposed").is_none());
    wire.as_object_mut().unwrap().remove("expectedRevision");
    assert!(
        from_value::<ModelCatalogFillCommitRequest>(wire)
            .unwrap_err()
            .to_string()
            .contains("expectedRevision")
    );
}

#[test]
fn catalog_contract_discovery_references_credentials_and_rejects_secrets() {
    let request = ModelDiscoveryRequest {
        deployment: identity().deployment,
        method: ModelDiscoveryMethod::Http,
        account_id: Some("saved-account".into()),
    };
    let wire = to_value(&request).unwrap();
    assert_eq!(wire["accountId"], "saved-account");
    assert_eq!(from_value::<ModelDiscoveryRequest>(wire.clone()).unwrap(), request);
    for field in ["apiKey", "token", "command"] {
        let mut injected = wire.clone();
        injected[field] = json!("unexpected");
        let error = from_value::<ModelDiscoveryRequest>(injected).unwrap_err();
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    let status = ModelDiscoveryStatus {
        task_id: "discovery-1".into(),
        deployment: request.deployment,
        state: ModelDiscoveryState::Failed { reason: ModelDiscoveryFailure::TimedOut },
    };
    let mut wire = to_value(&status).unwrap();
    assert_eq!(wire["state"]["phase"], "failed");
    assert_eq!(from_value::<ModelDiscoveryStatus>(wire.clone()).unwrap(), status);
    wire["state"]["stderr"] = json!("unexpected");
    assert!(from_value::<ModelDiscoveryStatus>(wire).is_err());
}

#[test]
fn catalog_contract_saved_codex_deployment_key_remains_compatible() {
    // ModelCatalogStore keys saved revisions by the serialized (deployment, revision) tuple.
    let key = r#"[{"targetHostId":"host-a","appId":"codex","providerId":"provider","endpoint":"https://example.invalid/plan/v1"},"revision-5"]"#;
    let (deployment, revision): (ModelCatalogIdentity, String) = serde_json::from_str(key).unwrap();
    assert_eq!(deployment, identity().deployment);
    assert_eq!(serde_json::to_string(&(deployment, revision)).unwrap(), key);
}

#[test]
fn catalog_contract_edit_batch_carries_default_change_and_model_id() {
    let edit = ModelCatalogEditRequest {
        deployment: identity().deployment,
        expected_revision: "revision-5".into(),
        edits: vec![
            ModelCatalogEdit::Remove { model_id: "vendor/Exact-ID".into() },
            ModelCatalogEdit::Upsert {
                model_id: "replacement".into(),
                display_alias: Some("Replacement".into()),
                metadata: Box::new(ModelMetadataDto {
                    context_window: Some(65_536),
                    ..Default::default()
                }),
            },
        ],
        default_change: Some(ModelCatalogDefaultChange::Set { model_id: "replacement".into() }),
    };
    let wire = to_value(&edit).unwrap();
    assert_eq!(wire["edits"][0]["modelId"], "vendor/Exact-ID");
    assert_eq!(wire["edits"][1]["metadata"]["contextWindow"], 65_536);
    assert_eq!(wire["defaultChange"]["modelId"], "replacement");
    assert_eq!(from_value::<ModelCatalogEditRequest>(wire).unwrap(), edit);
}
