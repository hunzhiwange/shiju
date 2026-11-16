use super::*;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionHostId(pub(super) String);
impl ExecutionHostId {
    pub fn new(id: &str) -> Result<Self, WriteError> {
        crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
        Ok(Self(id.into()))
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Direct,
    Proxy,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_login_operation: Option<String>,
    pub version: u32,
    pub key: String,
    pub exclusive: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeState {
    pub mode: Mode,
    pub attached: bool,
    pub proxy_route: Option<String>,
    pub contract: Option<Contract>,
}
/// Private executor proof of the last accepted aggregation projection. It can
/// contain a listener credential, so it deliberately has no Debug implementation.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregationRecovery {
    pub principal: String,
    pub published: serde_json::Value,
    /// A previously confirmed takeover applies only to this exact external pointer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_external_catalog: Option<String>,
}

/// None preserves that part of state; direct pointer and proxy route are independent.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregation_recovery: Option<AggregationRecovery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omo: Option<OmoSelection>,
    /// Set only by the executor after authorizing the physical project slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_scope: Option<vw_api_transport::ProjectScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_aggregation_settings:
        Option<vw_api_app_aggregation::CodexSettingsValues>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<crate::providers::mode_state::StackState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kimi_binding: Option<KimiBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_managed_login: Option<ClaudeManagedLogin>,
    pub pointer: Option<String>,
    pub mode: Option<ModeState>,
}
/// Scoped reference to the account whose login occupies the default Claude slot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeManagedLogin {
    pub tenant: TenantScope,
    pub principal: String,
    pub host: String,
    pub account_id: String,
}
/// Reference only. Account credentials remain with the managed and native login owners.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KimiBinding {
    pub(crate) provider_id: String,
    pub(crate) principal: String,
    pub(crate) account_id: String,
    pub(crate) region: AuthRegion,
    pub(crate) slot: String,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopedProvider {
    pub tenant: TenantScope,
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", content = "provider", rename_all = "snake_case", deny_unknown_fields)]
pub enum OmoSelection {
    Standard(String),
    Slim(String),
    Disabled,
}
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregation_recovery: Option<AggregationRecovery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omo: Option<OmoSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_scope: Option<vw_api_transport::ProjectScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_aggregation_settings:
        Option<vw_api_app_aggregation::CodexSettingsValues>,
    #[serde(default, skip_serializing_if = "crate::providers::mode_state::StackState::is_empty")]
    pub stack: crate::providers::mode_state::StackState,
    #[serde(default)]
    pub stack_tenant: Option<TenantScope>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub kimi_bindings: std::collections::BTreeMap<String, KimiBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_managed_login: Option<ClaudeManagedLogin>,
    pub pointer: Option<ScopedProvider>,
    pub mode: Option<ModeState>,
    pub mode_tenant: Option<TenantScope>,
}
impl AppState {
    pub fn uses_kimi_account(&self, tenant: &TenantScope, principal: &str, account: &str) -> bool {
        self.mode_tenant.as_ref() == Some(tenant)
            && self
                .kimi_bindings
                .values()
                .any(|binding| binding.principal == principal && binding.account_id == account)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingFile {
    /// Index into the executor registration; legacy intents use the primary directory.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub directory: usize,
    pub name: Vec<u8>,
    pub staged: Vec<u8>,
    pub pre: Option<String>,
    // Required field: a missing digest must never silently become a deletion.
    #[serde(deserialize_with = "Option::<String>::deserialize")]
    pub planned: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Pending {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_record: Option<super::super::record_publication::ProviderRecordChange>,
    #[serde(default)]
    pub project_preimage: Option<vw_api_transport::ProjectScope>,
    pub version: u32,
    pub host: ExecutionHostId,
    pub app: AppId,
    pub tenant: TenantScope,
    pub directory_device: u64,
    pub directory_inode: u64,
    pub target: PendingTarget,
    pub files: Vec<PendingFile>,
    pub published: bool,
}

fn is_zero(index: &usize) -> bool {
    *index == 0
}
