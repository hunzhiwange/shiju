//! Directory transaction helpers shared by provider handlers.
use axum::http::StatusCode;
use vw_api_transport::id::TenantScope;
use vw_agent_apps::providers::live::{write_guard::WriteError, commit::{ExecutionHostId, ProviderTransactions}};
use vw_agent_apps::providers::live::claude_directory::ClaudeDirectory;
pub fn directory_error(error: WriteError) -> StatusCode {
    match error {
        WriteError::Conflict => StatusCode::CONFLICT,
        WriteError::InvalidResource => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[cfg(target_os = "macos")]
pub fn transactions(
    root: &std::path::Path,
    host: &str,
) -> Result<ProviderTransactions, StatusCode> {
    let host = ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    ProviderTransactions::new(root, host).map_err(directory_error)
}

#[cfg(target_os = "macos")]
pub fn saved_claude_directory(
    root: Option<&std::path::Path>,
    host: &str,
    tenant: &TenantScope,
) -> Result<Option<ClaudeDirectory>, StatusCode> {
    let Some(root) = root else { return Ok(None) };
    let host = ExecutionHostId::new(host).map_err(|_| StatusCode::BAD_REQUEST)?;
    ProviderTransactions::open_existing(root, host)
        .and_then(|tx| tx.map(|tx| tx.claude_directory(tenant)).transpose())
        .map(Option::flatten)
        .map_err(directory_error)
}
