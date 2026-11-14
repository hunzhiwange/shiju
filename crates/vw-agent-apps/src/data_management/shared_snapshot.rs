//! Portable device configuration. No host identity, credentials or arbitrary maps.
use serde::{Deserialize, Serialize};
use vw_api_types::{agent_apps::AppId, app_providers::ProviderProtocol};

pub const DATA_COMPAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedSnapshot {
    pub data_compat_version: u32,
    pub providers: Vec<SharedProvider>,
    pub mcp: Vec<SharedMcp>,
    pub prompts: Vec<SharedPrompt>,
    pub preferences: Option<vw_api_types::agent_apps::AppUserPreferences>,
    pub skills: Vec<SharedSkill>,
    pub repositories: Vec<SharedRepository>,
    pub profiles: Vec<SharedProfile>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedProvider {
    pub id: String,
    pub app_id: AppId,
    pub name: String,
    pub protocol: Option<ProviderProtocol>,
    pub model: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedMcp {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedPrompt {
    pub id: String,
    pub app_id: AppId,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedSkill {
    pub id: String,
    pub directory: String,
    pub sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedRepository {
    pub owner: String,
    pub name: String,
    pub branch: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedProfile {
    pub id: String,
    pub name: String,
    pub groups: Vec<SharedProfileGroup>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedProfileGroup {
    pub app_id: AppId,
    pub provider: Option<String>,
    pub mcp: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub prompt: Option<String>,
}
