//! Bind the runtime session owner to trusted host scope and fixtures.
#[cfg(test)]
use super::*;
#[cfg(test)]
use std::{path::{Path, PathBuf}};
#[cfg(test)]
use crate::config;
use llm_runtime::acp::session as runtime;
pub(crate) use runtime::{acp_session_name, missing_session_error};
#[cfg(test)]
pub(super) use runtime::{bind_desktop_access};
#[cfg(test)]
pub(super) fn desktop_workspace(context: &crate::tools::ToolUseContext, session: &str, cwd: Option<&Path>) -> Result<(PathBuf, PathBuf), Error> {
    runtime::desktop_workspace(&super::request::desktop_context(context), session, cwd)
}
#[cfg(test)]
pub(super) fn get_cached_acp_client(agent: &str, cfg: &config::schema::AcpAgentConfig, cwd: &Path, parsed: &ParsedAcpOptions) -> CachedAcpClientEntry {
    runtime::get_cached_acp_client::<super::request::RuntimeHost>(agent, cfg, cwd, parsed)
}
#[cfg(test)]
pub(super) async fn stop_incompatible_cached_clients(agent: &str, cfg: &config::schema::AcpAgentConfig, cwd: &Path, parsed: &ParsedAcpOptions) -> Result<(), Error> {
    runtime::stop_incompatible_cached_clients::<super::request::RuntimeHost>(agent, cfg, cwd, parsed).await
}

#[cfg(test)]
fn acp_client_cache_key(agent: &str, cfg: &config::schema::AcpAgentConfig, cwd: &Path, parsed: &ParsedAcpOptions) -> String {
    runtime::acp_client_cache_key::<super::request::RuntimeHost>(agent, cfg, cwd, parsed)
}
#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
