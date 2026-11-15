//! Gateway-owned worktree policy and its unsubmitted editor draft.
use config_worktree::WorktreeSettings;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorktreesTab {
    #[default]
    General,
    Environment,
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorktreeSettingsIdentity {
    pub gateway: String,
    pub tenant: vw_api_transport::TenantScope,
    pub auth_fingerprint: [u8; 32],
    pub project: Option<String>,
}
impl std::fmt::Debug for WorktreeSettingsIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorktreeSettingsIdentity")
            .field("gateway", &self.gateway)
            .field("tenant", &self.tenant)
            .field("project", &self.project)
            .finish_non_exhaustive()
    }
}
impl WorktreeSettingsIdentity {
    pub fn from_endpoint(endpoint: &gateway_endpoint::GatewayEndpoint, project: Option<String>) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            auth_fingerprint: Sha256::digest(endpoint.auth.as_ref().and_then(|auth| auth.skey.as_deref()).unwrap_or("").as_bytes()).into(),
            project,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePolicyDraft {
    pub policy: WorktreeSettings,
    pub keep_count: String,
}
impl From<WorktreeSettings> for WorktreePolicyDraft {
    fn from(policy: WorktreeSettings) -> Self {
        Self { keep_count: policy.keep_count.to_string(), policy }
    }
}
impl WorktreePolicyDraft {
    pub fn validated(&self) -> Result<WorktreeSettings, &'static str> {
        let mut policy = self.policy.clone();
        policy.keep_count = self
            .keep_count
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|v| *v > 0)
            .ok_or("保留数量必须为大于零的整数。")?;
        policy.root = policy.root.trim().to_string();
        if policy.validate().is_err() {
            return Err("根目录必须为空或绝对路径。");
        }
        Ok(policy)
    }
}
#[derive(Debug, Clone, Default)]
pub struct WorktreeSettingsState {
    pub active_tab: WorktreesTab,
    pub managed: ManagedWorktreesState,
    pub recovery: WorktreeRecoveryState,
    pub environment: WorktreeEnvironmentState,
    pub saved: Option<WorktreeSettings>,
    pub default_root: String,
    pub draft: Option<WorktreePolicyDraft>,
    pub loading: bool,
    pub saving: bool,
    pub save_generation: Option<u64>,
    pub generation: u64,
    pub identity: Option<WorktreeSettingsIdentity>,
    pub error: Option<String>,
}

use vw_api_project::ProjectDto;
use vw_api_worktree::{ManagedWorktreeDto, WorktreeDeletionPreflight};

#[derive(Debug, Clone)]
pub struct ManagedWorktreeGroup {
    pub project: ProjectDto,
    pub items: Vec<ManagedWorktreeDto>,
    pub collapsed: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct ManagedWorktreesState {
    pub groups: Vec<ManagedWorktreeGroup>,
    pub generation: u64,
    pub loading: bool,
    pub identity: Option<WorktreeSettingsIdentity>,
    pub error: Option<String>,
    pub opening: Option<(String, String)>,
    pub retrying: Option<(String, String)>,
    pub preflight_target: Option<(String, String)>,
    pub preflight: Option<WorktreeDeletionPreflight>,
}
impl ManagedWorktreesState {
    pub fn replace_groups(&mut self, mut incoming: Vec<ManagedWorktreeGroup>) {
        let mut groups = Vec::new();
        for mut old in std::mem::take(&mut self.groups) {
            if let Some(index) = incoming.iter().position(|g| g.project.id == old.project.id) {
                let mut next = incoming.remove(index);
                next.collapsed = old.collapsed;
                // A failed project refresh retains its last successful rows for context,
                // but the error disables actions until a successful refresh.
                if next.error.is_some() {
                    next.items = std::mem::take(&mut old.items);
                }
                groups.push(next);
            }
        }
        groups.extend(incoming);
        self.groups = groups;
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorktreeEnvironmentState {
    pub projects: Vec<ProjectDto>,
    pub identity: Option<WorktreeSettingsIdentity>,
    pub list_generation: u64,
    pub projects_loading: bool,
    pub list_error: Option<String>,
    pub adding: bool,
    pub loaded: bool,
    pub selected: Option<ProjectDto>,
    pub editor: iced::widget::text_editor::Content,
    pub generation: u64,
    pub loading: bool,
    pub saving: bool,
    pub error: Option<String>,
}

/// Results are scoped to the same view epoch as the managed checkout list.
#[derive(Debug, Clone, Default)]
pub struct WorktreeRecoveryState {
    pub generation: u64,
    pub identity: Option<WorktreeSettingsIdentity>,
    pub loading: bool,
    pub loaded: bool,
    pub projects: Vec<WorktreeRecoveryProject>,
    pub error: Option<String>,
    pub ignored_acknowledged: bool,
    pub pending: Option<(String, String)>,
    pub outcomes: Vec<(String, String, Result<String, String>)>,
    pub restored: Option<(String, String, String)>,
}
#[derive(Debug, Clone)]
pub struct WorktreeRecoveryProject {
    pub project: ProjectDto,
    pub snapshots:
        Result<Vec<vw_api_worktree::WorktreeSnapshotSummary>, String>,
    pub proposal:
        Result<vw_api_worktree::WorktreeCleanupProposal, String>,
}
