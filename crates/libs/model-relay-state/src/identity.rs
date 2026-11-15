//! Trusted identity supplied by the session owner.
use vw_api_transport::id::TenantScope;

/// Trusted identity supplied by the session owner, never reconstructed from HTTP headers.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayIdentity {
    pub tenant: TenantScope,
    pub project: String,
    pub session: String,
    pub provider: String,
    pub model: String,
    pub generation: u64,
}
