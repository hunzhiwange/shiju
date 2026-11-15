//! Credential classification contract shared with routing.
use vw_api_app_provider_base::AppProviderSummary;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderAuth {
    ThirdParty,
    OfficialAccount,
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProviderSnapshot {
    pub provider: AppProviderSummary,
    pub auth: ProviderAuth,
}
