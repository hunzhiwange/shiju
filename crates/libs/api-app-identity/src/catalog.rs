//! Application identities and runtime forms shared by the catalog API.
use serde::{Deserialize, Serialize};

/// Public application identity. CLI executable names are separate metadata;
/// in particular `grok` is not an alias for the public ID `grokbuild`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppId {
    Claude,
    #[serde(rename = "claude-desktop")]
    ClaudeDesktop,
    Codex,
    Gemini,
    GrokBuild,
    OpenCode,
    OpenClaw,
    Hermes,
    Pi,
    Mcode,
    Kimi,
    Dsh,
    Agy,
    Omp,
}

/// Distinct execution forms; support for one does not imply support for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRuntimeKind {
    NativeCli,
    AcpAdapter,
    ReviewRuntime,
    DesktopApp,
    EditorIntegration,
}

/// Catalog metadata, independent of installation state and operation support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCatalogEntry {
    pub id: AppId,
    pub name: String,
    /// Executable name, not an executable command or application identity.
    pub cli_command: Option<String>,
    pub runtime_kinds: Vec<AppRuntimeKind>,
}

/// Catalog response. Registration alone does not promise install/launch support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCatalogResponse {
    pub apps: Vec<AppCatalogEntry>,
}
