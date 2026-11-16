//! Host scope injection and session projection for fork execution state.
use anyhow::Result;
pub(crate) use session_engine::fork_execution::ForkExecution;
fn scope() -> (std::path::PathBuf, vw_api_transport::TenantScope) {
    (crate::global::paths().home.clone(), super::ui_store::current_tenant_scope())
}
pub(crate) async fn load_scoped(project: &str, session: &str) -> Result<Option<ForkExecution>> {
    let (home, tenant) = scope();
    session_engine::fork_execution::load_scoped(home, tenant, project, session).await
}
pub(crate) async fn create(project: &str, session: &str, inherited_message_ids: Vec<String>) -> Result<()> {
    let (home, tenant) = scope();
    session_engine::fork_execution::create(home, tenant, project, session, inherited_message_ids).await
}
pub(crate) async fn remove(project: &str, session: &str) -> Result<()> {
    let (home, tenant) = scope();
    session_engine::fork_execution::remove(home, tenant, project, session).await
}
pub(crate) async fn start(project: &str, session: &str) -> Result<()> {
    let (home, tenant) = scope();
    session_engine::fork_execution::start(home, tenant, project, session).await
}
pub(crate) async fn is_selectable(project: &str, session: &str) -> Result<bool> {
    let (home, tenant) = scope();
    session_engine::fork_execution::is_selectable(home, tenant, project, session).await
}

/// Ignore any serialized projection and reload the authoritative server setting.
pub(crate) async fn project(info: &mut super::session::Info) -> Result<()> {
    let (home, tenant) = scope();
    session_engine::fork_execution::project(home, tenant, info).await
}

#[cfg(test)]
#[path = "fork_execution_tests.rs"]
mod tests;
