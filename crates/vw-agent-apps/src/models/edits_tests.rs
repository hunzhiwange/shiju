use super::edits::{edit, read};
use serde_json::{Value, json};
use std::{fs, io::Write};
use tempfile::TempDir;
use vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{
        ModelCapabilityState, ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogIdentity,
        ModelCatalogReadResponse, ModelCatalogWriteResponse, ModelMetadataDto, ModelTokenCost,
    },
};

struct Fixture {
    directory: TempDir,
    tenant: TenantScope,
    deployment: ModelCatalogIdentity,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("provider.json"), br#"{"name":"Fixture"}"#).unwrap();
        Self {
            directory,
            tenant: TenantScope::new("platform", "tenant").unwrap(),
            deployment: ModelCatalogIdentity {
                target_host_id: "mac".into(),
                app_id: AppId::OpenCode,
                provider_id: "fixture".into(),
                endpoint: "https://fixture.test/inference/v1".into(),
            },
        }
    }

    fn reopen(&self) -> Value {
        serde_json::from_slice(&self.bytes()).unwrap()
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.directory.path().join("provider.json")).unwrap()
    }

    fn catalog(&self) -> ModelCatalogReadResponse {
        read(&self.tenant, &self.deployment, &self.reopen()).unwrap()
    }

    fn request(&self, edits: Vec<ModelCatalogEdit>) -> ModelCatalogEditRequest {
        ModelCatalogEditRequest {
            deployment: self.deployment.clone(),
            expected_revision: self.catalog().revision,
            edits,
            default_change: None,
        }
    }

    fn submit(
        &self,
        request: &ModelCatalogEditRequest,
    ) -> Result<ModelCatalogWriteResponse, String> {
        edit(&self.tenant, &self.reopen(), request, |updated| {
            let mut next = tempfile::NamedTempFile::new_in(self.directory.path()).unwrap();
            next.write_all(&serde_json::to_vec(updated).unwrap()).unwrap();
            next.persist(self.directory.path().join("provider.json")).unwrap();
            Ok(true)
        })
    }
}

fn upsert(id: &str, metadata: ModelMetadataDto) -> ModelCatalogEdit {
    ModelCatalogEdit::Upsert {
        model_id: id.into(),
        display_alias: None,
        metadata: Box::new(metadata),
    }
}

fn select(id: &str) -> ModelCatalogEdit {
    upsert(id, ModelMetadataDto::default())
}

#[test]
fn catalog_edits_rejects_invalid_batch_atomically() {
    let f = Fixture::new();
    f.submit(&f.request(vec![select("saved")])).unwrap();
    let before = f.bytes();
    let baseline = f.catalog();
    let result = f.submit(&f.request(vec![
        ModelCatalogEdit::Remove { model_id: "saved".into() },
        select("valid"),
        select("bad\nmodel"),
    ]));
    assert_eq!(result.unwrap_err(), "invalid model ID");
    assert_eq!(f.bytes(), before);
    assert_eq!(f.catalog(), baseline);
}

#[test]
fn catalog_edits_rejects_invalid_limits_and_modalities() {
    let f = Fixture::new();
    let before = f.bytes();
    for (metadata, error) in [
        (
            ModelMetadataDto { context_window: Some(0), ..Default::default() },
            "invalid model limits",
        ),
        (
            ModelMetadataDto { max_output_tokens: Some(0), ..Default::default() },
            "invalid model limits",
        ),
        (
            ModelMetadataDto {
                context_window: Some(1024),
                max_output_tokens: Some(1025),
                ..Default::default()
            },
            "invalid model limits",
        ),
        (
            ModelMetadataDto { context_window: Some(u64::MAX), ..Default::default() },
            "invalid model limits",
        ),
        (
            ModelMetadataDto { input_modalities: Some(vec!["txt".into()]), ..Default::default() },
            "invalid model modalities",
        ),
        (
            ModelMetadataDto {
                output_modalities: Some(vec!["text".into(), "text".into()]),
                ..Default::default()
            },
            "invalid model modalities",
        ),
        (
            ModelMetadataDto {
                cost: Some(ModelTokenCost {
                    input: f64::NAN,
                    output: 0.0,
                    cache_read: None,
                    cache_write: None,
                }),
                ..Default::default()
            },
            "invalid model cost",
        ),
    ] {
        assert_eq!(f.submit(&f.request(vec![upsert("manual", metadata)])).unwrap_err(), error);
        assert_eq!(f.bytes(), before);
    }
}

#[test]
fn catalog_edits_saves_manual_models_offline_with_unknown_capabilities() {
    let f = Fixture::new();
    let metadata = ModelMetadataDto {
        context_window: Some(32_768),
        max_output_tokens: Some(4096),
        input_modalities: Some(vec!["text".into(), "image".into()]),
        ..Default::default()
    };
    let result =
        f.submit(&f.request(vec![upsert("Vendor/Manual:free", metadata.clone())])).unwrap();
    assert!(result.changed);
    let catalog = f.catalog();
    assert_eq!(catalog.revision, result.revision);
    let item = &catalog.items[0];
    assert_eq!(item.identity.model_id.as_ref(), "Vendor/Manual:free");
    assert_eq!(item.identity.scope, f.tenant);
    assert_eq!(item.identity.deployment, f.deployment);
    assert_eq!(item.metadata, metadata);
    assert_eq!(item.metadata.reasoning, ModelCapabilityState::Unknown);
    assert_eq!(item.metadata.tool_calls, ModelCapabilityState::Unknown);
    assert!(item.metadata.output_modalities.is_none());
    assert_eq!(f.reopen()["name"], json!("Fixture"));
}

#[test]
fn catalog_edits_deduplicates_exact_selections_without_replacing_manual_fields() {
    let f = Fixture::new();
    let metadata = ModelMetadataDto {
        tool_calls: ModelCapabilityState::Unsupported,
        output_modalities: Some(vec![]),
        ..Default::default()
    };
    f.submit(&f.request(vec![upsert("Vendor/Model", metadata.clone())])).unwrap();
    let selection = vec![
        select("Vendor/Model"),
        select("vendor/model"),
        select("vendor/model"),
        select("Vendor/Model:free"),
    ];
    let saved = f.submit(&f.request(selection.clone())).unwrap();
    let catalog = f.catalog();
    assert_eq!(catalog.items.len(), 3);
    assert_eq!(catalog.items[0].metadata, metadata);
    assert_eq!(catalog.items[1].identity.model_id.as_ref(), "vendor/model");
    assert_eq!(catalog.items[2].identity.model_id.as_ref(), "Vendor/Model:free");
    let before = f.bytes();
    let repeated = f.submit(&f.request(selection)).unwrap();
    assert!(!repeated.changed);
    assert_eq!(repeated.revision, saved.revision);
    assert_eq!(f.bytes(), before);
}

#[test]
fn catalog_edits_rejects_stale_revision_and_conditional_commit_conflict() {
    let f = Fixture::new();
    let remove = f.request(vec![ModelCatalogEdit::Remove { model_id: "added".into() }]);
    f.submit(&f.request(vec![select("added")])).unwrap();
    let before = f.bytes();
    assert_eq!(f.submit(&remove).unwrap_err(), "model catalog changed");
    let remove = f.request(remove.edits);
    let mut commits = 0;
    assert_eq!(
        edit(&f.tenant, &f.reopen(), &remove, |_| {
            commits += 1;
            Ok(false)
        })
        .unwrap_err(),
        "model catalog changed"
    );
    assert_eq!(commits, 1);
    assert_eq!(f.bytes(), before);
    assert!(f.submit(&remove).unwrap().changed);
    assert!(f.catalog().items.is_empty());
    assert_ne!(f.catalog().revision, remove.expected_revision);
}

#[test]
fn catalog_edits_keeps_endpoint_and_host_catalogs_separate() {
    let mut f = Fixture::new();
    let first = f.deployment.clone();
    f.submit(&f.request(vec![select("shared")])).unwrap();
    let first_catalog = f.catalog();
    f.deployment.endpoint = "https://fixture.test/inference/go/v1".into();
    assert!(f.catalog().items.is_empty());
    assert_eq!(
        f.submit(&ModelCatalogEditRequest {
            expected_revision: first_catalog.revision.clone(),
            ..f.request(vec![select("shared")])
        })
        .unwrap_err(),
        "model catalog changed"
    );
    f.submit(&f.request(vec![select("shared")])).unwrap();
    f.deployment.target_host_id = "another-mac".into();
    assert!(f.catalog().items.is_empty());
    f.deployment = first;
    assert_eq!(f.catalog(), first_catalog);
}

#[test]
fn catalog_edits_rejects_conflicting_operations_for_one_id() {
    let f = Fixture::new();
    let before = f.bytes();
    let result = f.submit(
        &f.request(vec![select("same"), ModelCatalogEdit::Remove { model_id: "same".into() }]),
    );
    assert_eq!(result.unwrap_err(), "conflicting edits for one model");
    assert_eq!(f.bytes(), before);
}
