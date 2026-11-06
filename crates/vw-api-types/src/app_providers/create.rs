//! Explicit create-only input; live activation belongs to application adapters.
use super::ProviderProtocol;
use serde::{Deserialize, Serialize};

/// Secret intent is explicit. A new required credential accepts Replace only.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderSecretWrite {
    Keep,
    Replace { value: String },
    Clear,
}
impl std::fmt::Debug for ProviderSecretWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderSecretWrite([redacted])")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderCreate {
    pub name: String,
    pub protocol: ProviderProtocol,
    pub api_key: ProviderSecretWrite,
    pub base_url: String,
    pub model: String,
    /// Required, with no default. Only false is supported by this create-only operation.
    pub add_to_live: bool,
}
