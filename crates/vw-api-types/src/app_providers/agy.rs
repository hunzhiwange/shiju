//! Agy cards select a terminal profile, never a global current provider.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgyProfile {
    pub name: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}
impl std::fmt::Debug for AgyProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgyProfile([redacted])")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgyWrite {
    pub target_host_id: String,
    pub profile: AgyProfile,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgyTarget {
    pub target_host_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgySaved {
    pub catalog_saved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgyFormView {
    pub fields: super::basic::ProviderBasic,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub has_saved_key: bool,
    pub revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgyFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: super::create::ProviderSecretWrite,
    pub revision: Option<String>,
}
