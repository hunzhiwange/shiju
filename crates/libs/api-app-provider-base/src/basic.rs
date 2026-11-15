//! Basic catalog editor. Credentials are write-only; application configuration stays on the executor.
use super::create::{AppProviderCreate, ProviderSecretWrite};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderBasic {
    pub name: String,
    pub category: Option<String>,
    pub notes: Option<String>,
    pub website_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BasicOperation {
    Create { draft: Box<AppProviderCreate> },
    Update,
    Copy { source_id: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderBasicWrite {
    pub fields: ProviderBasic,
    pub api_key: ProviderSecretWrite,
    pub action: BasicOperation,
}
