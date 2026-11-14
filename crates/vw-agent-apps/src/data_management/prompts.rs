//! Strict CC prompt decoding; importing catalog intent never publishes live files.
use super::{preview::PreviewError, source::{SourceCategory, SourceRecord}};
use crate::prompts::listing::PromptEntry;
use serde_json::Value;
use std::collections::BTreeSet;
use vw_api_types::agent_apps::{AppId, PromptResource};

pub struct MigrationPrompt {
    pub app: AppId,
    pub entry: PromptEntry,
    pub enabled: bool,
}

pub fn normalize(records: &[&SourceRecord]) -> Result<Vec<MigrationPrompt>, PreviewError> {
    let mut rows = Vec::new();
    for record in records {
        if record.category != SourceCategory::Rules { return Err(PreviewError::InvalidCandidate); }
        match record.origin.as_str() {
            "prompts" => rows.push(record.fields.clone()),
            "/prompts" => {
                for (app, manager) in record.fields.as_object().ok_or(PreviewError::InvalidCandidate)? {
                    let prompts = manager.get("prompts").and_then(Value::as_object).ok_or(PreviewError::InvalidCandidate)?;
                    for (id, prompt) in prompts {
                        if prompt.get("id").and_then(Value::as_str) != Some(id) { return Err(PreviewError::InvalidCandidate); }
                        let mut prompt = prompt.clone();
                        let object = prompt.as_object_mut().ok_or(PreviewError::InvalidCandidate)?;
                        object.insert("app_type".into(), Value::String(app.clone()));
                        if let Some(time) = object.remove("createdAt") { object.insert("created_at".into(), time); }
                        rows.push(prompt);
                    }
                }
            }
            _ => return Err(PreviewError::UnsupportedCategory),
        }
    }
    let mut identities = BTreeSet::new();
    let mut active = BTreeSet::new();
    rows.into_iter().map(|row| {
        let text = |key| row.get(key).and_then(Value::as_str).ok_or(PreviewError::InvalidCandidate);
        let app: AppId = serde_json::from_value(row.get("app_type").cloned().ok_or(PreviewError::InvalidCandidate)?)
            .map_err(|_| PreviewError::UnsupportedCategory)?;
        PromptResource::Application { app_id: app }.validate_application_access().map_err(|_| PreviewError::UnsupportedCategory)?;
        let id = text("id")?;
        crate::providers::validate_identity(id).map_err(|_| PreviewError::InvalidCandidate)?;
        let app_key = serde_json::to_string(&app).map_err(|_| PreviewError::InvalidCandidate)?;
        if !identities.insert((app_key.clone(), id.to_owned())) { return Err(PreviewError::InvalidCandidate); }
        let enabled = match row.get("enabled") {
            Some(Value::Bool(v)) => *v,
            Some(v) if v.as_i64() == Some(0) => false,
            Some(v) if v.as_i64() == Some(1) => true,
            None => false,
            _ => return Err(PreviewError::InvalidCandidate),
        };
        if enabled && !active.insert(app_key) { return Err(PreviewError::InvalidCandidate); }
        let name = text("name")?;
        if name.trim().is_empty() { return Err(PreviewError::InvalidCandidate); }
        let description = match row.get("description") {
            None | Some(Value::Null) => None,
            Some(Value::String(v)) => Some(v.clone()),
            _ => return Err(PreviewError::InvalidCandidate),
        };
        let created_at = match row.get("created_at") {
            None | Some(Value::Null) => None,
            Some(v) => Some(v.as_i64().ok_or(PreviewError::InvalidCandidate)?),
        };
        Ok(MigrationPrompt { app, enabled, entry: PromptEntry {
            id: id.into(), name: name.into(), content: text("content")?.into(), description, created_at,
        } })
    }).collect()
}
