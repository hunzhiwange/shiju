//! Shared project configuration snapshots. References are resolved on the authorized
//! execution host; this payload must never contain credentials or prompt contents.
use serde::{Deserialize, Serialize};

use crate::agent_apps::catalog::AppId;

/// Only these three applications participate in project profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectProfileScope {
    Claude,
    #[serde(rename = "claude-desktop")]
    ClaudeDesktop,
    Codex,
}

impl ProjectProfileScope {
    pub const ALL: [Self; 3] = [Self::Claude, Self::ClaudeDesktop, Self::Codex];

    pub fn app_id(self) -> AppId {
        match self {
            Self::Claude => AppId::Claude,
            Self::ClaudeDesktop => AppId::ClaudeDesktop,
            Self::Codex => AppId::Codex,
        }
    }

    pub fn for_app(app: AppId) -> Option<Self> {
        match app {
            AppId::Claude => Some(Self::Claude),
            AppId::ClaudeDesktop => Some(Self::ClaudeDesktop),
            AppId::Codex => Some(Self::Codex),
            _ => None,
        }
    }
}

/// Application slots are independent; unknown application keys are rejected.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectProfilePerApp<T: Default> {
    pub claude: T,
    #[serde(rename = "claude-desktop")]
    pub claude_desktop: T,
    pub codex: T,
}

impl<T: Default> ProjectProfilePerApp<T> {
    pub fn get(&self, scope: ProjectProfileScope) -> &T {
        match scope {
            ProjectProfileScope::Claude => &self.claude,
            ProjectProfileScope::ClaudeDesktop => &self.claude_desktop,
            ProjectProfileScope::Codex => &self.codex,
        }
    }

    pub fn get_mut(&mut self, scope: ProjectProfileScope) -> &mut T {
        match scope {
            ProjectProfileScope::Claude => &mut self.claude,
            ProjectProfileScope::ClaudeDesktop => &mut self.claude_desktop,
            ProjectProfileScope::Codex => &mut self.codex,
        }
    }
}

/// Empty prompt ID explicitly deactivates the currently active prompt.
pub const PROJECT_PROFILE_NO_ACTIVE_PROMPT: &str = "";

/// Installation identity is scoped to the containing application slot. A directory
/// name alone is not an installation ID. Profiles never trigger implicit downloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectProfileSkillRef {
    pub installation_id: String,
}

/// None (including JSON null or an omitted slot) means leave that category alone.
/// Some([]) means restore the empty enabled set; Some("") for prompts means stop
/// activation. Nonempty IDs refer to existing records, never embedded secrets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectProfilePayload {
    pub providers: ProjectProfilePerApp<Option<String>>,
    pub mcp: ProjectProfilePerApp<Option<Vec<String>>>,
    pub skills: ProjectProfilePerApp<Option<Vec<ProjectProfileSkillRef>>>,
    pub prompts: ProjectProfilePerApp<Option<String>>,
}

impl ProjectProfilePayload {
    pub fn scope_captured(&self, scope: ProjectProfileScope) -> bool {
        self.providers.get(scope).is_some()
            || self.mcp.get(scope).is_some()
            || self.skills.get(scope).is_some()
            || self.prompts.get(scope).is_some()
    }

    /// Resnapshot only the requested group, preserving the other applications.
    pub fn merge_scope_from(&mut self, other: &Self, scope: ProjectProfileScope) {
        *self.providers.get_mut(scope) = other.providers.get(scope).clone();
        *self.mcp.get_mut(scope) = other.mcp.get(scope).clone();
        *self.skills.get_mut(scope) = other.skills.get(scope).clone();
        *self.prompts.get_mut(scope) = other.prompts.get(scope).clone();
    }
}

/// A separate domain identity, never an AgentProfile ID or a filesystem ProjectId.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectProfileId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectProfile {
    pub id: ProjectProfileId,
    /// Owner validates a trimmed, nonempty name before storage.
    pub name: String,
    /// Optimistic concurrency token, rechecked before executing a preview.
    pub version: u64,
    pub payload: ProjectProfilePayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProfileCategory {
    Provider,
    Mcp,
    Skill,
    Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProfileItemStatus {
    Succeeded,
    Missing,
    Conflict,
    Denied,
    Failed,
    Cancelled,
    Unsupported,
}

/// A committed item remains successful if a later item fails or is cancelled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectProfileItemResult {
    pub category: ProjectProfileCategory,
    pub reference_id: Option<String>,
    pub status: ProjectProfileItemStatus,
    /// Stable, sanitized reason code; never native payloads or credentials.
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProfileApplyStatus {
    Applied,
    Partial,
    Uncaptured,
    Cancelled,
    Failed,
}

/// Selection is separate from application. Applied is reserved for confirmation
/// of every required category; uncaptured/partial/cancelled never imply applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectProfileApplyResult {
    pub profile_id: ProjectProfileId,
    pub group: ProjectProfileScope,
    pub selected: bool,
    pub status: ProjectProfileApplyStatus,
    pub items: Vec<ProjectProfileItemResult>,
}
