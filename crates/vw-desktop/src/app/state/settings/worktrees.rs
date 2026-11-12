//! Gateway-owned worktree policy and its unsubmitted editor draft.
use vw_config_types::worktree::WorktreeSettings;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorktreesTab {
    #[default]
    General,
    Environment,
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorktreeSettingsIdentity {
    pub(crate) gateway: String,
    pub(crate) tenant: vw_gateway_client::vw_api_types::id::TenantScope,
    pub(crate) auth_fingerprint: [u8; 32],
    pub(crate) project: Option<String>,
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
    pub(crate) fn current(app: &crate::app::App) -> Self {
        use sha2::{Digest, Sha256};
        let endpoint = crate::app::gateway_client_endpoint();
        Self {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            auth_fingerprint: Sha256::digest(
                endpoint
                    .auth
                    .as_ref()
                    .and_then(|auth| auth.skey.as_deref())
                    .unwrap_or("")
                    .as_bytes(),
            )
            .into(),
            project: app.project_path.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreePolicyDraft {
    pub(crate) policy: WorktreeSettings,
    pub(crate) keep_count: String,
}
impl From<WorktreeSettings> for WorktreePolicyDraft {
    fn from(policy: WorktreeSettings) -> Self {
        Self { keep_count: policy.keep_count.to_string(), policy }
    }
}
impl WorktreePolicyDraft {
    pub(crate) fn validated(&self) -> Result<WorktreeSettings, &'static str> {
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
pub(crate) struct WorktreeSettingsState {
    pub(crate) active_tab: WorktreesTab,
    pub(crate) saved: Option<WorktreeSettings>,
    pub(crate) default_root: String,
    pub(crate) draft: Option<WorktreePolicyDraft>,
    pub(crate) loading: bool,
    pub(crate) saving: bool,
    pub(crate) save_generation: Option<u64>,
    pub(crate) generation: u64,
    pub(crate) identity: Option<WorktreeSettingsIdentity>,
    pub(crate) error: Option<String>,
}
