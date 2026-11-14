//! Host-private CC profile preparation; storage remains with ProjectProfileStore.
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vw_api_types::project_profiles::*;

use super::{preview::{self, Decision, ImportPreview, PreviewCandidate, PreviewError}, source::*};

/// Only selected, successfully resolved source references belong in this map.
/// Keys are (category, serialized application scope, source ID). Targets must be
/// checked by the real domain readers in the same authorized project/host scope.
pub type ReferenceMap = BTreeMap<(String, String, String), String>;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SourcePayload {
    providers: ProjectProfilePerApp<Option<String>>,
    mcp: ProjectProfilePerApp<Option<Vec<String>>>,
    skills: ProjectProfilePerApp<Option<Vec<String>>>,
    prompts: ProjectProfilePerApp<Option<String>>,
}

pub struct ProfilePreview {
    preview: ImportPreview,
    rows: Vec<ProjectProfile>,
}

impl ProfilePreview {
    pub fn entries(&self) -> &[preview::PreviewEntry] { self.preview.entries() }

    /// Run within the owner's transaction and operation lock, after authorization
    /// and a durable rollback snapshot. Recheck dependencies before any insert.
    pub fn resolve(
        &self, scope: &SourceScope, source: &SourceSnapshot,
        existing: &[ProjectProfile], decisions: &[(Uuid, Decision)],
        mut exists: impl FnMut(ProjectProfileCategory, ProjectProfileScope, &str) -> bool,
    ) -> Result<Vec<ProjectProfile>, PreviewError> {
        let resolved = self.preview.resolve(scope, source, &revision(existing)?, decisions)?;
        let mut rows = Vec::new();
        for (entry, row) in self.preview.entries().iter().zip(&self.rows) {
            if resolved[&entry.id] == Decision::Skip { continue; }
            check_references(&row.payload, &mut exists)?;
            rows.push(row.clone());
        }
        Ok(rows)
    }
}

pub fn revision(rows: &[ProjectProfile]) -> Result<String, PreviewError> {
    let mut values = BTreeMap::new();
    for row in rows {
        if values.insert(&row.id.0, row).is_some() { return Err(PreviewError::InvalidCandidate); }
    }
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&values)
        .map_err(|_| PreviewError::InvalidCandidate)?)))
}

/// Parse every selected profile before offering writes. CC profiles are SQLite
/// rows; legacy JSON does not define this category. Current pointers are ignored.
pub fn prepare(
    scope: &SourceScope, source: &SourceSnapshot, existing: &[ProjectProfile],
    references: &ReferenceMap,
    mut exists: impl FnMut(ProjectProfileCategory, ProjectProfileScope, &str) -> bool,
) -> Result<ProfilePreview, PreviewError> {
    let mut rows = Vec::new();
    let mut valid = Vec::new();
    let mut ids = BTreeSet::new();
    for record in source.records(scope).map_err(|_| PreviewError::Forbidden)?
        .iter().filter(|r| r.category == SourceCategory::Profiles) {
        if record.origin != "profiles" { return Err(PreviewError::InvalidCandidate); }
        let text = |key: &str| record.fields.get(key).and_then(serde_json::Value::as_str)
            .ok_or(PreviewError::InvalidCandidate);
        let id = text("id")?;
        let name = text("name")?.trim();
        if id.trim().is_empty() || name.is_empty() || !ids.insert(id.to_owned()) {
            return Err(PreviewError::InvalidCandidate);
        }
        let raw: SourcePayload = serde_json::from_str(text("payload")?)
            .map_err(|_| PreviewError::InvalidCandidate)?;
        let mut payload = ProjectProfilePayload::default();
        let mut references_valid = true;
        for app in ProjectProfileScope::ALL {
            let app_key = serde_json::to_value(app).map_err(|_| PreviewError::InvalidCandidate)?;
            let app_key = app_key.as_str().ok_or(PreviewError::InvalidCandidate)?;
            let mut remap = |category: ProjectProfileCategory, old: &str| -> String {
                let key = match category {
                    ProjectProfileCategory::Provider => "provider",
                    ProjectProfileCategory::Mcp => "mcp",
                    ProjectProfileCategory::Skill => "skill",
                    ProjectProfileCategory::Prompt => "prompt",
                };
                match references.get(&(key.into(), app_key.into(), old.into())) {
                    Some(target) if !target.trim().is_empty() && exists(category, app, target) => target.clone(),
                    _ => { references_valid = false; old.to_owned() }
                }
            };
            *payload.providers.get_mut(app) = raw.providers.get(app).as_ref()
                .map(|id| remap(ProjectProfileCategory::Provider, id));
            *payload.mcp.get_mut(app) = raw.mcp.get(app).as_ref().map(|ids| ids.iter()
                .map(|id| remap(ProjectProfileCategory::Mcp, id)).collect());
            *payload.skills.get_mut(app) = raw.skills.get(app).as_ref().map(|ids| ids.iter()
                .map(|id| ProjectProfileSkillRef { installation_id: remap(ProjectProfileCategory::Skill, id) }).collect());
            *payload.prompts.get_mut(app) = raw.prompts.get(app).as_ref().map(|id| {
                if id.is_empty() { String::new() } else { remap(ProjectProfileCategory::Prompt, id) }
            });
        }
        rows.push(ProjectProfile { id: ProjectProfileId(id.into()), name: name.into(), version: 0,
            updated_at: None, payload });
        valid.push(references_valid);
    }
    let preview = preview::build_preview(scope, source, &[SourceCategory::Profiles], revision(existing)?, |_, _| {
        rows.iter().zip(&valid).map(|(row, valid)| {
            let previous = existing.iter().find(|p| p.id == row.id);
            // Version/time are target metadata, not imported source content.
            let value = |p: &ProjectProfile| serde_json::json!({"id":p.id,"name":p.name,"payload":p.payload});
            Ok(PreviewCandidate { incoming: value(row), existing: previous.map(value), references_valid: *valid })
        }).collect()
    })?;
    Ok(ProfilePreview { preview, rows })
}

fn check_references(
    payload: &ProjectProfilePayload,
    exists: &mut impl FnMut(ProjectProfileCategory, ProjectProfileScope, &str) -> bool,
) -> Result<(), PreviewError> {
    for app in ProjectProfileScope::ALL {
        let mut refs = Vec::new();
        if let Some(id) = payload.providers.get(app) { refs.push((ProjectProfileCategory::Provider, id.as_str())); }
        if let Some(ids) = payload.mcp.get(app) { refs.extend(ids.iter().map(|id| (ProjectProfileCategory::Mcp, id.as_str()))); }
        if let Some(ids) = payload.skills.get(app) { refs.extend(ids.iter().map(|id| (ProjectProfileCategory::Skill, id.installation_id.as_str()))); }
        if let Some(id) = payload.prompts.get(app) { if !id.is_empty() { refs.push((ProjectProfileCategory::Prompt, id.as_str())); } }
        for (category, id) in refs {
            if id.trim().is_empty() || !exists(category, app, id) { return Err(PreviewError::InvalidCandidate); }
        }
    }
    Ok(())
}
