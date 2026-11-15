//! Frozen common snippets. Saving never applies a snapshot to live configuration.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnippetRead {
    pub target_host_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnippetWrite {
    pub target_host_id: String,
    pub content: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SnippetSource {
    Draft { settings: Value },
    Saved { provider_id: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnippetExtract {
    pub target_host_id: String,
    pub source: SnippetSource,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSnippet {
    pub content: Option<String>,
}
macro_rules! concealed { ($($t:ty),*) => { $(impl std::fmt::Debug for $t {
 fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("ConfigSnippet([redacted])") }
})* }; }
concealed!(SnippetRead, SnippetWrite, SnippetSource, SnippetExtract, ConfigSnippet);
