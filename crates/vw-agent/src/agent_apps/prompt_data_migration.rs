//! Preview-bound prompt catalog migration; no native file IO or activation.
use super::{PromptCatalog, PromptScope, PromptsStore, NAMESPACE};
use anyhow::{Result, ensure};
use crate::gateway::settings_store::GatewaySetting;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use std::{collections::BTreeMap, path::Path};
use vw_agent_apps::data_management::{
    prompts::normalize,
    preview::{build_preview, Decision, ImportPreview, PreviewCandidate, PreviewError},
    source::{SourceCategory, SourceScope, SourceSnapshot},
    safety_snapshot::{with_safety_snapshot, OwnerPreimage, PresentPreimage, SafetySnapshotReceipt,
        SafetySnapshotScope, SnapshotCategory, SnapshotTarget},
};

pub struct PromptMigration {
    scope: SourceScope,
    preview: ImportPreview,
    revision: String,
    observed: Vec<GatewaySetting>,
    incoming: Vec<GatewaySetting>,
}
impl PromptMigration {
    pub fn preview(&self) -> &ImportPreview { &self.preview }
}
pub struct PromptMigrationCommit {
    pub snapshot: Option<SafetySnapshotReceipt>,
    pub outcome: Result<usize>,
}
impl PromptsStore {
    /// Caller authorizes source reading and all affected application catalogs.
    /// Decisions apply to each application's complete merged catalog so its
    /// active selection and entries cannot be committed independently.
    pub fn preview_data_migration(&self, scope: &SourceScope, source: &SourceSnapshot) -> Result<PromptMigration> {
        let rows = source.records(scope)?;
        let selected = rows.iter().filter(|r| r.category == SourceCategory::Rules).collect::<Vec<_>>();
        let normalized = normalize(&selected).map_err(|_| anyhow::anyhow!("invalid prompt migration source"))?;
        let observed = self.0.list(scope.project().tenant_scope(), NAMESPACE)?;
        let mut catalogs = BTreeMap::new();
        for row in normalized {
            let key = PromptScope { project: scope.project(), host: scope.host_id(), app: row.app }.key()?;
            if !catalogs.contains_key(&key) {
                let old = observed.iter().find(|v| v.config_key == key);
                let catalog = Self::decode(old.map(|v| v.payload_json.as_str()))?;
                catalogs.insert(key.clone(), catalog);
            }
            let catalog = catalogs.get_mut(&key).unwrap();
            if row.enabled { catalog.active_id = Some(row.entry.id.clone()); }
            else if catalog.active_id.as_deref() == Some(row.entry.id.as_str()) { catalog.active_id = None; }
            catalog.entries.retain(|v| v.id != row.entry.id);
            catalog.entries.push(row.entry);
        }
        let mut incoming = Vec::new();
        for (key, mut catalog) in catalogs {
            catalog.entries = vw_agent_apps::prompts::listing::list_entries(catalog.entries, "");
            incoming.push(GatewaySetting { config_key: key, payload_json: serde_json::to_string(&catalog)? });
        }
        let revision = digest(&observed.iter().map(|v| (&v.config_key, &v.payload_json)).collect::<Vec<_>>())?;
        let preview = build_preview(scope, source, &[SourceCategory::Rules], revision.clone(), |_, _| {
            incoming.iter().map(|row| Ok(PreviewCandidate {
                incoming: comparable(&row.payload_json)?,
                existing: observed.iter().find(|v| v.config_key == row.config_key)
                    .map(|v| comparable(&v.payload_json)).transpose()?,
                references_valid: true,
            })).collect()
        }).map_err(|_| anyhow::anyhow!("prompt migration preview failed"))?;
        Ok(PromptMigration { scope: scope.clone(), preview, revision, observed, incoming })
    }

    /// Invoke after reauthorizing every application. SQLite commits the entire
    /// category using one CAS; unsupported backends fail without partial writes.
    pub fn commit_data_migration(&self, scope: &SourceScope, source: &SourceSnapshot,
        prepared: &PromptMigration, decisions: &[(Uuid, Decision)], snapshot_root: &Path,
    ) -> Result<PromptMigrationCommit> {
        ensure!(scope == &prepared.scope, "prompt migration scope mismatch");
        let resolved = prepared.preview.resolve(scope, source, &prepared.revision, decisions)
            .map_err(|_| anyhow::anyhow!("stale or unresolved prompt migration"))?;
        let mut replacements = Vec::new();
        for (entry, row) in prepared.preview.entries().iter().zip(&prepared.incoming) {
            if resolved[&entry.id] == Decision::Skip { continue; }
            let mut catalog = Self::decode(Some(&row.payload_json))?;
            catalog.revision = catalog.revision.checked_add(1).ok_or_else(|| anyhow::anyhow!("prompt revision exhausted"))?;
            replacements.push(GatewaySetting { config_key: row.config_key.clone(), payload_json: serde_json::to_string(&catalog)? });
        }
        let apply = || -> Result<usize> {
            ensure!(self.0.apply_catalog_migration(scope.project().tenant_scope(), NAMESPACE,
                &prepared.observed, &replacements)?, "prompt migration revision conflict");
            Ok(replacements.len())
        };
        if replacements.is_empty() { return Ok(PromptMigrationCommit { snapshot: None, outcome: apply() }); }
        let snapshot_scope = SafetySnapshotScope { project: scope.project().clone(),
            principal_id: scope.principal_id().into(), host_id: scope.host_id().into() };
        let targets = replacements.iter().map(|row| SnapshotTarget {
            category: SnapshotCategory::Prompt, object_id: row.config_key.clone(),
        }).collect::<Vec<_>>();
        let preimages = targets.iter().map(|target| OwnerPreimage { target: target.clone(),
            state: prepared.observed.iter().find(|v| v.config_key == target.object_id).map(|v| PresentPreimage {
                revision: prepared.revision.clone(), bytes: v.payload_json.as_bytes().to_vec(),
            }),
        }).collect();
        let (receipt, outcome) = with_safety_snapshot(snapshot_root, &snapshot_scope, &targets,
            || Ok(preimages), |_| apply())?;
        Ok(PromptMigrationCommit { snapshot: Some(receipt), outcome })
    }
}
fn digest(value: &impl serde::Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
fn comparable(raw: &str) -> Result<serde_json::Value, PreviewError> {
    let mut catalog: PromptCatalog = serde_json::from_str(raw).map_err(|_| PreviewError::InvalidCandidate)?;
    catalog.revision = 0;
    catalog.bootstrap_complete = false;
    Ok(serde_json::json!({"content": digest(&catalog).map_err(|_| PreviewError::InvalidCandidate)?}))
}
