//! Native Claude routing transaction types.
use super::{
    claude::ClaudeFile,
    live::{commit::*, write_guard::WriteError},
};
use {vw_api_app_identity::AppId, vw_api_transport::id::TenantScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeApplyError {
    InvalidInput,
    Ownership,
    Write(WriteError),
}
impl From<WriteError> for ModeApplyError {
    fn from(e: WriteError) -> Self {
        Self::Write(e)
    }
}
/// Scope is executor-authorized. It is never accepted from a runtime or HTTP body.
pub struct RouteScope<'a> {
    pub tenant: &'a TenantScope,
    pub host: &'a str,
    pub app: AppId,
}
pub(super) fn validate_client_endpoint(url: &str) -> Result<(), ModeApplyError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ModeApplyError::InvalidInput)?;
    if parsed.scheme() != "http"
        || !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        || parsed.port().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(ModeApplyError::InvalidInput);
    }
    Ok(())
}

pub struct ClaudeMode<'a> {
    pub file: &'a mut ClaudeFile,
    pub transactions: &'a ProviderTransactions,
    pub scope: RouteScope<'a>,
}
