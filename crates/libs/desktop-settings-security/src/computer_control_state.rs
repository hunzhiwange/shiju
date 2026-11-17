//! Server-owned control settings; UI snapshots never grant execution authority.
use vw_api_computer_control::*;

#[derive(Clone, PartialEq, Eq)]
pub struct ControlIdentity {
    gateway: String,
    tenant: vw_api_transport::TenantScope,
    auth_fingerprint: [u8; 32],
}
impl ControlIdentity {
    pub fn from_endpoint(endpoint: &gateway_endpoint::GatewayEndpoint) -> Self {
        use sha2::{Digest, Sha256};
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
pub struct PairingCredential(pub String);
impl std::fmt::Debug for PairingCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("PairingCredential([redacted])") }
}
#[derive(Debug, Clone)]
pub struct OfficePairingDraft {
    pub feature: ControlSetting,
    pub document: String,
    pub session: String,
    pub workspace: Option<std::path::PathBuf>,
    pub credential: Option<PairingCredential>,
}
#[derive(Debug, Clone, Default)]
pub struct ComputerControlSettingsState {
    pub identity: Option<ControlIdentity>,
    pub generation: u64,
    pub loading: bool,
    pub saving: bool,
    pub stopping: bool,
    pub status: Option<ComputerControlStatus>,
    pub approvals: Vec<ControlGrantScope>,
    pub error: Option<String>,
    pub pairing: Option<OfficePairingDraft>,
    pub pairing_generation: u64,
    pub management: Option<ControlSetting>,
    pub native_permission_report: Option<NativePermissionReport>,
    pub requesting_permission: bool,
    pub native_service: Option<NativeServiceReport>,
    pub target_host: String,
}
impl ComputerControlSettingsState {
    pub fn busy(&self) -> bool { self.loading || self.saving || self.requesting_permission }
    pub fn feature(&self, feature: ControlSetting) -> Option<&ControlFeatureStatus> {
        self.status.as_ref().map(|s| match feature {
            ControlSetting::NativeApps => &s.native_apps,
            ControlSetting::Chrome => &s.chrome,
            ControlSetting::Excel => &s.excel,
            ControlSetting::Word => &s.word,
            ControlSetting::LockedUse => &s.locked_use.status,
        })
    }
    pub fn apply_loaded(&mut self, generation: u64, result: Result<(ComputerControlStatus, ControlApprovals), String>) -> bool {
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

impl ComputerControlSettingsState {
    pub fn reset_for_endpoint(&mut self, endpoint: &gateway_endpoint::GatewayEndpoint) {
    if self.identity.as_ref()
        != Some(&ControlIdentity::from_endpoint(endpoint))
    {
        let generation = self.generation + 1;
        let pairing_generation = self.pairing_generation + 1;
        *self = Default::default();
        self.generation = generation;
        self.pairing_generation = pairing_generation;
    }
    }
}
