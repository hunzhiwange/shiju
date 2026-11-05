//! Read-only installation observations, never executable authorization.
use super::AppId;
use serde::{Deserialize, Serialize};

/// Opaque server-issued identity, resolved again in the authorized host context.
/// This value is not a path and does not itself grant permission to execute.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppInstallationId(pub String);

/// Environment on the target host. WSL observations always identify their distro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppInstallationEnvironment {
    Macos,
    Windows,
    Linux,
    Wsl { distribution: String },
}

/// Observed installation channel; unknown never implies a fallback command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppInstallationSource {
    Homebrew,
    Nvm,
    Volta,
    Fnm,
    Mise,
    Bun,
    Pnpm,
    Scoop,
    Pip,
    System,
    Native,
    Unknown,
}

/// Probe failure is distinct from a completed command's nonzero exit.
/// No raw stderr is exposed: it may contain credentials or sensitive payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AppInstallationRuntimeStatus {
    Runnable,
    ExitedWithError {
        #[serde(rename = "exitCode")]
        exit_code: i32,
    },
    ProbeFailed {
        reason: AppInstallationProbeFailure,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppInstallationProbeFailure {
    SpawnFailed,
    TimedOut,
    Cancelled,
    InvalidVersionOutput,
}

/// One independently discovered installation, including its execution context.
/// Paths are display-only observations from the host, not client command inputs.
/// Mutation/launch handlers must resolve the ID within authenticated host scope
/// and revalidate the executable rather than trust this deserializable DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInstallation {
    pub id: AppInstallationId,
    pub app_id: AppId,
    /// Host identity is metadata; authorization comes from the gateway context.
    pub target_host_id: String,
    pub environment: AppInstallationEnvironment,
    /// Entry path before symlink resolution, expressed in the target OS syntax.
    pub path: String,
    /// Resolved path when discovery could resolve it; never an authorization token.
    pub real_path: Option<String>,
    pub source: AppInstallationSource,
    pub is_path_default: bool,
    pub version: Option<String>,
    pub runtime_status: AppInstallationRuntimeStatus,
}

/// Discovery response. Conflict and action policy remain with the domain crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInstallationReport {
    pub installations: Vec<AppInstallation>,
}
