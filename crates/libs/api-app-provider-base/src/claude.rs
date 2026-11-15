//! Claude-only configuration. No client paths to configuration roots or login material.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaudeConfig {
    Official {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<super::claude_login::ClaudeLoginBinding>,
    },
    ThirdParty {
        base_url: String,
        api_key: String,
        #[serde(default)]
        api_key_header: bool,
    },
    Bedrock {
        region: String,
        auth: BedrockAuth,
    },
    Vertex {
        project: String,
        region: String,
        #[serde(default)]
        credentials_file: Option<String>,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BedrockAuth {
    DefaultChain {},
    Bearer {
        token: String,
    },
    Keys {
        access_key_id: String,
        secret_access_key: String,
        #[serde(default)]
        session_token: Option<String>,
    },
    Profile {
        name: String,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeProfile {
    pub name: String,
    pub config: ClaudeConfig,
    #[serde(default)]
    pub model: Option<String>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeActivation {
    #[default]
    CatalogOnly,
    First,
    IfCurrent,
    Switch,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeWrite {
    pub target_host_id: String,
    pub claude: ClaudeProfile,
    #[serde(default)]
    pub activation: ClaudeActivation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeTarget {
    pub target_host_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeImport {
    pub target_host_id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeLiveResult {
    NotAttempted,
    Applied,
    Failed,
    RecoveryRequired,
}
/// Directory writes survive a later live failure. A caller must inspect both outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeResult {
    pub catalog_saved: bool,
    pub live: ClaudeLiveResult,
}
macro_rules! concealed {
    ($($t:ty),*) => {$(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("ClaudeConfig([redacted])") }
    })*};
}
concealed!(ClaudeConfig, BedrockAuth, ClaudeProfile, ClaudeWrite);

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeOr<T> {
    Claude(ClaudeWrite),
    Legacy(T),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeSync {
    pub target_host_id: String,
    #[serde(default)]
    pub activation: ClaudeActivation,
}

/// Only the fields editable by the Claude Code form. Stored credentials and account bindings
/// are never present in this response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeFormView {
    pub fields: super::basic::ProviderBasic,
    pub config: ClaudeFormKind,
    pub model: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ClaudeFormKind {
    Official {},
    ThirdParty { base_url: String, api_key_header: bool },
    Unsupported {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeFormWrite {
    pub target_host_id: String,
    pub fields: super::basic::ProviderBasic,
    pub config: ClaudeFormConfig,
    pub model: Option<String>,
    /// First selection on create, or reapply only if current on edit.
    pub apply: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ClaudeFormConfig {
    Official {},
    ThirdParty {
        base_url: String,
        api_key_header: bool,
        api_key: super::create::ProviderSecretWrite,
    },
}
concealed!(ClaudeFormConfig);
