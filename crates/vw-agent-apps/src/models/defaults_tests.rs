use super::edits::{edit, read};
use crate::providers::codex;
use serde_json::{Value, json};
use std::{fs, io::Write};
use tempfile::NamedTempFile;
use vw_api_types::{
    agent_apps::AppId,
    id::TenantScope,
    model_catalog::{
        ModelCatalogDefaultChange::{self, Clear, Set},
        ModelCatalogEdit::{self, Remove, Upsert},
        ModelCatalogEditRequest, ModelCatalogIdentity, ModelCatalogReadResponse,
        ModelCatalogWriteResponse,
    },
};

const MODEL: &str = "Vendor/Model:free";

struct Catalog {
    file: NamedTempFile,
    tenant: TenantScope,
    deployment: ModelCatalogIdentity,
}

impl Catalog {
    fn new() -> Self {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"{}").unwrap();
        Self {
            file,
            tenant: TenantScope::new("platform", "company").unwrap(),
            deployment: ModelCatalogIdentity {
                target_host_id: "mac".into(),
                app_id: AppId::Codex,
                provider_id: "fixture".into(),
                endpoint: "https://fixture.test/v1".into(),
            },
        }
    }

    fn load(&self) -> Value {
        serde_json::from_slice(&fs::read(self.file.path()).unwrap()).unwrap()
    }

    fn read(&self) -> ModelCatalogReadResponse {
        read(&self.tenant, &self.deployment, &self.load()).unwrap()
    }

    fn edit(
        &self,
        edits: Vec<ModelCatalogEdit>,
        default_change: Option<ModelCatalogDefaultChange>,
    ) -> Result<ModelCatalogWriteResponse, String> {
        edit(
            &self.tenant,
            &self.load(),
            &ModelCatalogEditRequest {
                deployment: self.deployment.clone(),
                expected_revision: self.read().revision,
                edits,
                default_change,
            },
            |updated| {
                let mut file = NamedTempFile::new_in(self.file.path().parent().unwrap()).unwrap();
                serde_json::to_writer(&mut file, updated).unwrap();
                file.persist(self.file.path()).unwrap();
                Ok(true)
            },
        )
    }
}

fn upsert(id: &str, alias: Option<&str>) -> ModelCatalogEdit {
    Upsert {
        model_id: id.into(),
        display_alias: alias.map(str::to_owned),
        metadata: Default::default(),
    }
}

fn native_model(catalog: &ModelCatalogReadResponse) -> Value {
    let default = catalog.default_model_id.as_ref().unwrap();
    let identity =
        &catalog.items.iter().find(|row| row.identity.model_id == *default).unwrap().identity;
    let settings = json!({
        "auth": {"OPENAI_API_KEY": "test-key"},
        "config": format!(
            "model_provider='fixture'\nmodel='{}'\n[model_providers.fixture]\nbase_url='{}'\nwire_api='responses'\n",
            identity.model_id.as_ref(),
            identity.deployment.endpoint,
        ),
    });
    let projected = codex::project("", &settings, None, false).unwrap();
    let config: Value = toml::from_str(projected.config()).unwrap();
    config["model"].clone()
}

#[test]
fn catalog_defaults_alias_keeps_request_identity() {
    let f = Catalog::new();
    f.edit(vec![upsert(MODEL, None)], Some(Set { model_id: MODEL.into() })).unwrap();
    let before = f.read();
    let saved = f.edit(vec![upsert(MODEL, Some("展示名"))], None).unwrap();
    let reopened = f.read();
    assert_eq!(reopened.revision, saved.revision);
    assert_eq!(reopened.default_model_id, before.default_model_id);
    assert_eq!(reopened.items[0].identity, before.items[0].identity);
    assert_eq!(reopened.items[0].identity.deployment, f.deployment);
    assert_eq!(reopened.items[0].display_alias.as_deref(), Some("展示名"));
    assert_eq!(native_model(&before), MODEL);
    assert_eq!(native_model(&reopened), MODEL);
}

#[test]
fn catalog_defaults_rejects_foreign_deployment_reference() {
    let mut f = Catalog::new();
    f.edit(vec![upsert(MODEL, Some("Primary"))], Some(Set { model_id: MODEL.into() })).unwrap();
    let original = f.deployment.clone();
    let before = f.read();
    for deployment in [
        ModelCatalogIdentity { endpoint: format!("{}/", original.endpoint), ..original.clone() },
        ModelCatalogIdentity { target_host_id: "other-mac".into(), ..original.clone() },
    ] {
        f.deployment = deployment;
        assert_eq!(
            f.edit(vec![], Some(Set { model_id: MODEL.into() })).unwrap_err(),
            "default model not found in deployment",
        );
        f.edit(vec![upsert(MODEL, Some("Secondary"))], None).unwrap();
        assert!(f.read().default_model_id.is_none());
        assert_eq!(f.read().items[0].display_alias.as_deref(), Some("Secondary"));
    }
    f.deployment = original;
    assert_eq!(f.read(), before);
}

#[test]
fn catalog_defaults_rejects_missing_or_inexact_replacement_atomically() {
    let f = Catalog::new();
    f.edit(vec![upsert(MODEL, None)], Some(Set { model_id: MODEL.into() })).unwrap();
    let before = f.load();
    for model in ["missing", "vendor/model:free", "Vendor/Model"] {
        assert_eq!(
            f.edit(vec![upsert("available", None)], Some(Set { model_id: model.into() }))
                .unwrap_err(),
            "default model not found in deployment",
        );
        assert_eq!(f.load(), before);
    }
    assert_eq!(
        f.edit(vec![Remove { model_id: MODEL.into() }], Some(Set { model_id: MODEL.into() }))
            .unwrap_err(),
        "default model not found in deployment",
    );
    assert_eq!(f.load(), before);
}

#[test]
fn catalog_defaults_removes_default_only_with_explicit_replacement_or_clear() {
    let f = Catalog::new();
    f.edit(vec![upsert("other", None), upsert(MODEL, None)], Some(Set { model_id: MODEL.into() }))
        .unwrap();
    let before = f.load();
    let remove = |model: &str| Remove { model_id: model.into() };
    assert_eq!(f.edit(vec![remove(MODEL)], None).unwrap_err(), "cannot remove default model");
    assert_eq!(f.load(), before);
    f.edit(
        vec![remove(MODEL), upsert("replacement", None)],
        Some(Set { model_id: "replacement".into() }),
    )
    .unwrap();
    assert_eq!(f.read().default_model_id, Some("replacement".into()));
    f.edit(vec![remove("replacement")], Some(Clear)).unwrap();
    let reopened = f.read();
    assert_eq!(reopened.default_model_id, None);
    assert_eq!(reopened.items[0].identity.model_id.as_ref(), "other");
}

#[test]
fn catalog_defaults_preserves_alias_on_selection_and_allows_explicit_clear() {
    let f = Catalog::new();
    f.edit(vec![upsert(MODEL, Some("Friendly"))], Some(Set { model_id: MODEL.into() })).unwrap();
    let before = f.read();
    let repeated = f.edit(vec![upsert(MODEL, None)], Some(Set { model_id: MODEL.into() })).unwrap();
    assert!(!repeated.changed);
    assert_eq!(f.read(), before);
    f.edit(vec![upsert(MODEL, Some(""))], None).unwrap();
    let reopened = f.read();
    assert!(reopened.items[0].display_alias.is_none());
    assert_eq!(reopened.default_model_id, before.default_model_id);
}

#[test]
fn catalog_defaults_rejects_invalid_alias_before_commit() {
    let f = Catalog::new();
    let before = f.load();
    for alias in ["bad\nname", &"a".repeat(1025)] {
        assert_eq!(
            f.edit(vec![upsert(MODEL, Some(alias))], None).unwrap_err(),
            "invalid display alias",
        );
        assert_eq!(f.load(), before);
    }
}
