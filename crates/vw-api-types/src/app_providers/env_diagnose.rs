//! Static shell definitions and read-only inherited names. Values never leave the executor.
use crate::agent_apps::AppId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvDiagnoseRequest {
    pub target_host_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserShellFile {
    Bashrc,
    BashProfile,
    Zshrc,
    Zprofile,
    Profile,
}
impl UserShellFile {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Bashrc => ".bashrc",
            Self::BashProfile => ".bash_profile",
            Self::Zshrc => ".zshrc",
            Self::Zprofile => ".zprofile",
            Self::Profile => ".profile",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemShellFile {
    Profile,
    Bashrc,
}
impl SystemShellFile {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Bashrc => "bashrc",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnvSource {
    UserShell { file: UserShellFile, line: usize },
    SystemShell { file: SystemShellFile, line: usize },
    ProcessInherited,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvDiagnostic {
    pub variable: String,
    pub source: EnvSource,
    /// Eligibility only; cleanup must revalidate a server-issued diagnostic and base.
    pub cleanup_available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvDiagnosis {
    pub app_id: AppId,
    /// Shell entries are static definitions, not evaluated effective environment values.
    pub entries: Vec<EnvDiagnostic>,
}
