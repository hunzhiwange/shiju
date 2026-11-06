//! Partial catalog update. Missing fields are preserved; explicit null is invalid.
use super::{ProviderProtocol, create::ProviderSecretWrite};
use serde::{Deserialize, Deserializer, Serialize};
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}
fn keep() -> ProviderSecretWrite {
    ProviderSecretWrite::Keep
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppProviderUpdate {
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ProviderProtocol>,
    /// Missing means Keep. Replace rejects redaction markers ([redacted], <redacted>,
    /// and strings of '*' or '•'); Clear is rejected while the credential is required.
    #[serde(default = "keep")]
    pub api_key: ProviderSecretWrite,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
impl Default for AppProviderUpdate {
    fn default() -> Self {
        Self { name: None, protocol: None, api_key: keep(), base_url: None, model: None }
    }
}
