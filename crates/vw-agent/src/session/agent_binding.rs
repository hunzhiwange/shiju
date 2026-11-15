//! Server-owned session binding persistence and admission.
use anyhow::Result;
use config_agent_profile::AgentRuntime;
#[cfg(test)]
use config_agent::AgentDefinitionConfig;
#[cfg(test)]
use config_agent_profile::AgentDefaults;
pub(crate) use session_engine::agent_binding::AgentBinding;

pub(super) use session_engine::binding_store::storage_key;
pub(super) fn store() -> Result<app_settings_store::settings_store::GatewaySettingsStore> {
    let database = agent_database::GlobalDatabase::for_home(&crate::global::paths().home)?;
    app_settings_store::settings_store::GatewaySettingsStore::new(database)
}
fn scope() -> (std::path::PathBuf, vw_api_transport::TenantScope) {
    (crate::global::paths().home.clone(), crate::session::ui_store::current_tenant_scope())
}
pub(crate) async fn load_scoped(project: &str, session: &str) -> Result<Option<AgentBinding>> {
    let (home, tenant) = scope();
    session_engine::binding_store::load_scoped(home, tenant, project, session).await
}
pub(crate) async fn save_scoped(project: &str, session: &str, binding: &AgentBinding) -> Result<()> {
    let (home, tenant) = scope();
    session_engine::binding_store::save_scoped(home, tenant, project, session, binding).await
}
pub(crate) struct SessionSettings(session_engine::binding_store::SessionSettings);
impl std::ops::Deref for SessionSettings {
    type Target = session_engine::binding_store::SessionSettings;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl SessionSettings {
    pub(crate) async fn load(project: &str, session: &str) -> Result<Self> {
        let (home, tenant) = scope();
        session_engine::binding_store::SessionSettings::load(home, tenant, project, session).await.map(Self)
    }
    pub(super) async fn save(&self, project: &str, session: &str) -> Result<()> {
        let (home, tenant) = scope();
        self.0.save(home, tenant, project, session).await
    }
    pub(super) async fn remove(&self, project: &str, session: &str) -> Result<()> {
        let (home, tenant) = scope();
        self.0.remove(home, tenant, project, session).await
    }
}
pub(crate) async fn confirmed_full_access(project: &str, session: &str) -> Result<bool> {
    let (home, tenant) = scope();
    session_engine::binding_store::confirmed_full_access(home, tenant, project, session).await
}
pub(crate) async fn resolve_full_access_scoped(project: &str, session: &str, explicit: Option<bool>, initial: bool) -> Result<bool> {
    let (home, tenant) = scope();
    session_engine::binding_store::resolve_full_access_scoped(home, tenant, project, session, explicit, initial).await
}

pub(crate) async fn resolve_full_access(
    session: &str,
    explicit: Option<bool>,
    initial: bool,
) -> Result<bool> {
    if session.is_empty() {
        return Ok(explicit.unwrap_or(initial));
    }
    let project = crate::project::instance::project()
        .ok_or_else(|| anyhow::anyhow!("session permissions require a project"))?
        .id;
    resolve_full_access_scoped(&project, session, explicit, initial).await
}

pub(crate) async fn load(session: &str) -> Result<Option<AgentBinding>> {
    let project = crate::project::instance::project()
        .ok_or_else(|| anyhow::anyhow!("session binding requires a project"))?
        .id;
    load_scoped(&project, session).await
}

/// Reload from server storage at the execution boundary, never from request options.
pub(crate) async fn apply_to_config(
    session: &str,
    config: &mut crate::config::Config,
) -> Result<()> {
    session_engine::admission::apply_to_config::<AdmissionHost>(session, config).await
}

#[cfg(test)]
#[path = "agent_binding_tests.rs"]
mod tests;

pub(crate) async fn prepare(session: &str, selected_agent: Option<&str>, backend: Option<&str>, explicit_model: Option<&str>, workspace: &std::path::Path) -> Result<(AgentBinding, crate::config::Config)> {
    session_engine::admission::prepare::<AdmissionHost>(session, selected_agent, backend, explicit_model, workspace).await
}

pub(crate) struct AdmissionHost;
impl session_engine::admission_host::AdmissionHost for AdmissionHost {
    async fn config() -> Result<crate::config::Config> { Ok(crate::config::get_current().await?) }
    fn project() -> Option<String> { crate::project::instance::project().map(|p| p.id) }
    async fn fork_selectable(project: &str, session: &str) -> Result<bool> { super::fork_execution::is_selectable(project, session).await }
    async fn load_binding(session: &str) -> Result<Option<AgentBinding>> { load(session).await }
    async fn save_binding(project: &str, session: &str, binding: &AgentBinding) -> Result<()> { save_scoped(project, session, binding).await }
    async fn history_agents(session: &str) -> Result<Vec<String>> {
        Ok(super::message::messages(session, Some(1)).await?.into_iter().map(|entry| match entry.info {
            super::message::Info::Assistant(a) => a.agent,
            super::message::Info::User(u) => u.agent,
        }).collect())
    }
    async fn agent(key: &str) -> Option<config_agent::AgentDefinitionConfig> { crate::agent::get(key).await }
    fn skills(workspace: &std::path::Path, config: &crate::config::Config) -> std::collections::HashMap<String, bool> {
        crate::skills::load_skills_with_config(workspace, config).iter().map(|s| (s.invocation_name().to_string(), true)).collect()
    }
    async fn mcp(workspace: &std::path::Path, runtime: &AgentRuntime) -> Result<std::collections::HashMap<String, bool>> {
        let mcp = match runtime {
            AgentRuntime::Native => crate::tools::mcp_common::load_resolved_runtime_mcp_config_from(workspace).await?,
            AgentRuntime::Acp { .. } => crate::tools::mcp_common::load_resolved_acp_config_from(workspace).await?,
        };
        Ok(mcp.mcp_servers.iter().map(|s| (crate::tools::mcp_common::server_name(s).to_string(), crate::tools::mcp_common::management_for_server(&mcp, s).enabled)).collect())
    }
    async fn permission_ceiling(key: &str) -> Result<permission_runtime::next::Ruleset> { crate::agent::native_permission_ceiling(key).await.map_err(anyhow::Error::msg) }
    async fn default_model() -> Result<config_agent_profile::AgentModelReference> {
        let model = crate::provider::provider::default_model().await.map_err(anyhow::Error::msg)?;
        Ok(config_agent_profile::AgentModelReference { provider: model.provider_id, model: model.model_id })
    }
}
