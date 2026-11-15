//! Ordering and text retrieval for disabled prompt catalogs, after scope authorization.
use serde::{Deserialize, Serialize};

/// No enabled field: this store cannot activate prompts or grant native file ownership.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptEntry {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    /// Unix milliseconds; absent timestamps sort before known timestamps.
    pub created_at: Option<i64>,
}

/// Match name, description and body with trimmed, Unicode lowercase text.
/// The caller must load only its authorized catalog before calling this function.
pub fn list_entries(mut entries: Vec<PromptEntry>, query: &str) -> Vec<PromptEntry> {
    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        entries.retain(|entry| {
            [Some(entry.name.as_str()), entry.description.as_deref(), Some(entry.content.as_str())]
                .into_iter()
                .flatten()
                .any(|value| value.to_lowercase().contains(&query))
        });
    }
    entries.sort_by(|left, right| {
        left.created_at.cmp(&right.created_at).then_with(|| left.id.cmp(&right.id))
    });
    entries
}
