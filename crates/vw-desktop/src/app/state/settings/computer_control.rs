//! Server-owned control settings; UI snapshots never grant execution authority.
use vw_gateway_client::vw_api_types::computer_control::*;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ControlIdentity {
    gateway: String,
    tenant: vw_gateway_client::vw_api_types::id::TenantScope,
    auth_fingerprint: [u8; 32],
}
impl ControlIdentity {
    pub(crate) fn current() -> Self {
        use sha2::{Digest, Sha256};
        let endpoint = crate::app::gateway_client_endpoint();
        Self {
            gateway: endpoint.base_url(),
            tenant: endpoint.tenant_scope().clone(),
            auth_fingerprint: Sha256::digest(endpoint.auth.as_ref().and_then(|a| a.skey.as_deref()).unwrap_or("").as_bytes()).into(),
        }
    }
}
impl std::fmt::Debug for ControlIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlIdentity").field("gateway", &self.gateway).finish_non_exhaustive()
    }
}

/// One-use credential kept only in memory while its pairing modal is open.
#[derive(Clone)]
pub(crate) struct PairingCredential(pub(crate) String);
impl std::fmt::Debug for PairingCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("PairingCredential([redacted])") }
}
#[derive(Debug, Clone)]
pub(crate) struct OfficePairingDraft {
    pub(crate) feature: ControlSetting,
    pub(crate) document: String,
    pub(crate) session: String,
    pub(crate) workspace: Option<std::path::PathBuf>,
    pub(crate) credential: Option<PairingCredential>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct ComputerControlSettingsState {
    pub(crate) identity: Option<ControlIdentity>,
    pub(crate) generation: u64,
    pub(crate) loading: bool,
    pub(crate) saving: bool,
    pub(crate) stopping: bool,
    pub(crate) status: Option<ComputerControlStatus>,
    pub(crate) approvals: Vec<ControlGrantScope>,
    pub(crate) error: Option<String>,
    pub(crate) pairing: Option<OfficePairingDraft>,
    pub(crate) pairing_generation: u64,
}
impl ComputerControlSettingsState {
    pub(crate) fn busy(&self) -> bool { self.loading || self.saving }
    pub(crate) fn feature(&self, feature: ControlSetting) -> Option<&ControlFeatureStatus> {
        self.status.as_ref().map(|s| match feature {
            ControlSetting::NativeApps => &s.native_apps,
            ControlSetting::Chrome => &s.chrome,
            ControlSetting::Excel => &s.excel,
            ControlSetting::Word => &s.word,
            ControlSetting::LockedUse => &s.locked_use.status,
        })
    }
    pub(crate) fn apply_loaded(&mut self, generation: u64, result: Result<(ComputerControlStatus, ControlApprovals), String>) -> bool {
        if self.generation != generation || !self.loading { return false; }
        self.loading = false;
        self.status = None;
        self.approvals.clear();
        match result {
            Ok((status, approvals)) if status.permission_version == approvals.permission_version => {
                self.status = Some(status);
                self.approvals = approvals.approvals;
                self.error = None;
            }
            Ok(_) => self.error = Some("授权版本已变化，请刷新后重试".into()),
            Err(error) => self.error = Some(error),
        }
        true
    }
}
