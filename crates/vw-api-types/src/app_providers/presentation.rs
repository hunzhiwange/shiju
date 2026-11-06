//! Catalog presentation only. PUT replaces all three fields; null/omitted clears a field.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderPresentation {
    pub sort_index: Option<u32>,
    pub category: Option<String>,
    pub notes: Option<String>,
}
/// Category supplies the grouping key; entries are ordered by index then identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderPresentationEntry {
    pub id: String,
    pub presentation: AppProviderPresentation,
}
