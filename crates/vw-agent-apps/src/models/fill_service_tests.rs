use super::{edits, fill_service};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    time::{Duration, UNIX_EPOCH},
};
use tempfile::TempDir;
use vw_api_types::{
    agent_apps::AppId,
    common::TimestampMs,
    id::TenantScope,
    model_catalog::{
        ModelCapabilityState, ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogEntry,
        ModelCatalogFillCommitRequest, ModelCatalogFillPreviewRequest,
        ModelCatalogFillPreviewResponse, ModelCatalogIdentity, ModelCatalogReadResponse,
        ModelCatalogWriteResponse, ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance,
        ModelMetadataSource, ScopedModelCatalogIdentity,
    },
};
use vw_provider_resolver::models::ModelsSnapshot;

const ENDPOINT: &str = "https://api.baiduqianfan.ai/v2/tokenplan/team";
const MODEL: &str = "glm-5.2";

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
                app_id: AppId::Codex,
                provider_id: "fixture".into(),
                endpoint: ENDPOINT.into(),
            },
        }
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.directory.path().join("provider.json")).unwrap()
    }

    fn read(&self) -> Value {
        serde_json::from_slice(&self.bytes()).unwrap()
    }

    fn write(&self, before: &Value, updated: &Value) -> Result<bool, String> {
        if self.read() != *before {
            return Ok(false);
        }
        let mut next = tempfile::NamedTempFile::new_in(self.directory.path()).unwrap();
        next.write_all(&serde_json::to_vec(updated).unwrap()).unwrap();
        next.persist(self.directory.path().join("provider.json")).unwrap();
        Ok(true)
    }

    fn catalog(&self) -> ModelCatalogReadResponse {
        edits::read(&self.tenant, &self.deployment, &self.read()).unwrap()
    }

    fn edit(&self, changes: Vec<ModelCatalogEdit>) {
        let before = self.read();
        let request = ModelCatalogEditRequest {
            deployment: self.deployment.clone(),
            expected_revision: self.catalog().revision,
            edits: changes,
            default_change: None,
        };
        edits::edit(&self.tenant, &before, &request, |updated| self.write(&before, updated))
            .unwrap();
    }

    fn identity(&self, id: &str) -> ScopedModelCatalogIdentity {
        ScopedModelCatalogIdentity {
            scope: self.tenant.clone(),
            deployment: self.deployment.clone(),
            model_id: id.into(),
        }
    }

    fn preview(
        &self,
        id: &str,
        snapshot: Option<&ModelsSnapshot>,
    ) -> Result<ModelCatalogFillPreviewResponse, String> {
        fill_service::preview(
            &self.tenant,
            &self.deployment,
            &self.read(),
            &ModelCatalogFillPreviewRequest { identity: self.identity(id) },
            snapshot,
        )
    }

    fn commit(
        &self,
        request: &ModelCatalogFillCommitRequest,
        snapshot: Option<&ModelsSnapshot>,
    ) -> Result<ModelCatalogWriteResponse, String> {
        let before = self.read();
        fill_service::commit(
            &self.tenant,
            &self.deployment,
            &before,
            request,
            snapshot,
            |updated| self.write(&before, updated),
        )
    }

    fn row(&self, id: &str) -> ModelCatalogEntry {
        self.catalog().items.into_iter().find(|row| row.identity.model_id.as_ref() == id).unwrap()
    }
}

fn upsert(id: &str, metadata: ModelMetadataDto) -> ModelCatalogEdit {
    ModelCatalogEdit::Upsert {
        model_id: id.into(),
        display_alias: Some("展示别名".into()),
        metadata: Box::new(metadata),
    }
}

fn select(id: &str) -> ModelCatalogEdit {
    upsert(id, ModelMetadataDto::default())
}

fn request(preview: &ModelCatalogFillPreviewResponse) -> ModelCatalogFillCommitRequest {
    ModelCatalogFillCommitRequest {
        identity: preview.identity.clone(),
        expected_revision: preview.revision.clone(),
    }
}

fn snapshot(providers: Value) -> ModelsSnapshot {
    ModelsSnapshot {
        providers: serde_json::from_value(providers).unwrap(),
        fetched_at: Some(UNIX_EPOCH + Duration::from_millis(1_234)),
    }
}

fn delayed_snapshot() -> ModelsSnapshot {
    snapshot(json!({
        "fixture": {"api": ENDPOINT, "models": { (MODEL): {
            "limit": {"context": 1_000_000, "output": 98_304},
            "reasoning": true, "tool_call": true, "modalities": {"output": ["text"]}
        }}}
    }))
}

#[test]
fn catalog_fill_revision_rejects_stale_revision() {
    let f = Fixture::new();
    f.edit(vec![select(MODEL)]);
    let old = ModelCatalogFillCommitRequest {
        identity: f.identity(MODEL),
        expected_revision: f.catalog().revision,
    };
    f.edit(vec![ModelCatalogEdit::Remove { model_id: MODEL.into() }, select("replacement")]);
    let before = f.bytes();
    assert_eq!(f.commit(&old, Some(&delayed_snapshot())).unwrap_err(), "model catalog changed");
    assert_eq!(f.bytes(), before);
    assert_eq!(f.row("replacement").metadata, ModelMetadataDto::default());
}

#[test]
fn catalog_fill_revision_previews_offline_without_writing() {
    let f = Fixture::new();
    f.edit(vec![select(MODEL)]);
    let before = f.bytes();
    let preview = f.preview(MODEL, None).unwrap();
    assert_eq!(preview.identity, f.identity(MODEL));
    assert_eq!(preview.revision, f.catalog().revision);
    assert_eq!(preview.proposed.context_window, Some(500_000));
    assert_eq!(
        preview.proposed.provenance[&ModelMetadataField::ContextWindow].source,
        ModelMetadataSource::Preset
    );
    assert_eq!(preview.proposed.max_output_tokens, None);
    assert_eq!(f.bytes(), before);
}

#[test]
fn catalog_fill_revision_late_data_fills_only_blank_fields() {
    let f = Fixture::new();
    let manual = ModelMetadataProvenance { source: ModelMetadataSource::Manual, fetched_at: None };
    f.edit(vec![upsert(
        MODEL,
        ModelMetadataDto {
            context_window: Some(200_000),
            reasoning: ModelCapabilityState::Unsupported,
            input_modalities: Some(vec![]),
            provenance: [(ModelMetadataField::ContextWindow, manual.clone())].into(),
            ..Default::default()
        },
    )]);
    let offline = f.preview(MODEL, None).unwrap();
    assert_eq!(offline.proposed.context_window, None);
    assert!(!offline.proposed.provenance.contains_key(&ModelMetadataField::ContextWindow));
    let offline_write = f.commit(&request(&offline), None).unwrap();
    assert_eq!(offline_write.changed, offline.proposed != ModelMetadataDto::default());
    let snapshot = delayed_snapshot();
    let late = f.preview(MODEL, Some(&snapshot)).unwrap();
    assert_eq!(late.proposed.max_output_tokens, Some(98_304));
    assert_eq!(late.proposed.reasoning, ModelCapabilityState::Unknown);
    assert_eq!(late.proposed.input_modalities, None);
    assert_eq!(
        late.proposed.provenance[&ModelMetadataField::MaxOutputTokens],
        ModelMetadataProvenance {
            source: ModelMetadataSource::ModelsDev,
            fetched_at: Some(TimestampMs(1_234))
        }
    );
    assert!(f.commit(&request(&late), Some(&snapshot)).unwrap().changed);
    let row = f.row(MODEL);
    assert_eq!(row.display_alias.as_deref(), Some("展示别名"));
    assert_eq!(row.metadata.context_window, Some(200_000));
    assert_eq!(row.metadata.reasoning, ModelCapabilityState::Unsupported);
    assert_eq!(row.metadata.input_modalities, Some(vec![]));
    assert_eq!(row.metadata.max_output_tokens, Some(98_304));
    assert_eq!(row.metadata.provenance[&ModelMetadataField::ContextWindow], manual);
}

#[test]
fn catalog_fill_revision_uses_exact_saved_row_and_its_endpoint() {
    let mut f = Fixture::new();
    f.edit(vec![select(MODEL)]);
    f.deployment.endpoint = "https://fixture.test/model-override".into();
    f.edit(vec![select(MODEL)]);
    let snapshot = snapshot(json!({
        "provider": {"api": ENDPOINT, "models": { (MODEL): {"limit": {"context": 200_000}} }},
        "override": {"api": f.deployment.endpoint, "models": { (MODEL): {"limit": {"context": 300_000}} }}
    }));
    let preview = f.preview(MODEL, Some(&snapshot)).unwrap();
    assert_eq!(preview.proposed.context_window, Some(300_000));
    assert_eq!(f.preview("GLM-5.2", Some(&snapshot)).unwrap_err(), "model not found in deployment");
    assert!(f.commit(&request(&preview), Some(&snapshot)).unwrap().changed);
    assert_eq!(f.row(MODEL).metadata.context_window, Some(300_000));
    f.deployment.endpoint = ENDPOINT.into();
    assert_eq!(f.row(MODEL).metadata.context_window, None);
}

#[test]
fn catalog_fill_revision_rejects_old_endpoint_host_and_tenant() {
    let mut f = Fixture::new();
    f.edit(vec![select(MODEL)]);
    let old = request(&f.preview(MODEL, None).unwrap());
    for (host, endpoint) in [("mac", "https://fixture.test/v2"), ("other-mac", ENDPOINT)] {
        f.deployment.target_host_id = host.into();
        f.deployment.endpoint = endpoint.into();
        f.edit(vec![select(MODEL)]);
        let before = f.bytes();
        assert_eq!(f.commit(&old, None).unwrap_err(), "model fill target changed");
        let mut retargeted = old.clone();
        retargeted.identity = f.identity(MODEL);
        assert_eq!(f.commit(&retargeted, None).unwrap_err(), "model catalog changed");
        assert_eq!(f.bytes(), before);
    }
    f.deployment = old.identity.deployment.clone();
    f.tenant = TenantScope::new("platform", "other").unwrap();
    let before = f.bytes();
    assert_eq!(f.commit(&old, None).unwrap_err(), "model fill target changed");
    assert_eq!(f.bytes(), before);
}

#[test]
fn catalog_fill_revision_replay_and_no_change_do_not_write() {
    let f = Fixture::new();
    f.edit(vec![select(MODEL), select("unknown")]);
    let priced = snapshot(json!({
        "fixture": {"api": ENDPOINT, "models": { (MODEL): {
            "cost": {"input": 51.248178375505404, "output": 97.45365320034685}
        }}}
    }));
    let preview = f.preview(MODEL, Some(&priced)).unwrap();
    assert!(preview.proposed.cost.is_some());
    let first = request(&preview);
    let changed = f.commit(&first, Some(&priced)).unwrap();
    assert!(changed.changed);
    let after = f.bytes();
    let replay = f.commit(&first, Some(&delayed_snapshot())).unwrap();
    assert!(!replay.changed);
    assert_eq!(replay.revision, changed.revision);
    let other_row =
        ModelCatalogFillCommitRequest { identity: f.identity("unknown"), ..first.clone() };
    assert_eq!(f.commit(&other_row, None).unwrap_err(), "model catalog changed");
    let no_change = f.preview(MODEL, None).unwrap();
    assert_eq!(no_change.proposed, ModelMetadataDto::default());
    for preview in [no_change, f.preview("unknown", None).unwrap()] {
        let result = f.commit(&request(&preview), None).unwrap();
        assert!(!result.changed);
        assert_eq!(result.revision, changed.revision);
    }
    assert_eq!(f.bytes(), after);
    f.edit(vec![select("later-edit")]);
    assert_eq!(f.commit(&first, None).unwrap_err(), "model catalog changed");
}

#[test]
fn catalog_fill_revision_conditional_write_preserves_competing_edit() {
    let f = Fixture::new();
    f.edit(vec![select(MODEL)]);
    let fill = request(&f.preview(MODEL, None).unwrap());
    let before = f.read();
    let competing = ModelMetadataDto { context_window: Some(100_000), ..Default::default() };
    let result = fill_service::commit(&f.tenant, &f.deployment, &before, &fill, None, |updated| {
        f.edit(vec![upsert(MODEL, competing.clone())]);
        f.write(&before, updated)
    });
    assert_eq!(result.unwrap_err(), "model catalog changed");
    assert_eq!(f.row(MODEL).metadata, competing);
}

#[test]
fn catalog_fill_revision_skips_invalid_source_fields_without_losing_valid_ones() {
    let f = Fixture::new();
    f.edit(vec![upsert(
        MODEL,
        ModelMetadataDto { context_window: Some(10_000), ..Default::default() },
    )]);
    let snapshot = snapshot(json!({
        "fixture": {"api": ENDPOINT, "models": { (MODEL): {
            "limit": {"context": 1_000_000, "output": 98_304},
            "reasoning": true, "modalities": {"output": ["unsupported-modality"]}
        }}}
    }));
    let preview = f.preview(MODEL, Some(&snapshot)).unwrap();
    assert_eq!(preview.proposed.max_output_tokens, None);
    assert_eq!(preview.proposed.output_modalities, None);
    assert!(!preview.proposed.provenance.contains_key(&ModelMetadataField::MaxOutputTokens));
    assert!(!preview.proposed.provenance.contains_key(&ModelMetadataField::OutputModalities));
    assert_eq!(preview.proposed.reasoning, ModelCapabilityState::Supported);
    assert!(f.commit(&request(&preview), Some(&snapshot)).unwrap().changed);
    let row = f.row(MODEL);
    assert_eq!(row.metadata.context_window, Some(10_000));
    assert_eq!(row.metadata.reasoning, ModelCapabilityState::Supported);
    assert_eq!(row.metadata.max_output_tokens, None);
}
