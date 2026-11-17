use std::sync::OnceLock;

pub trait PlanRunHost: Send + Sync {
    fn recursive_orchestration_policy(&self) -> &'static str;
    fn current_tenant_scope(&self) -> vw_api_transport::id::TenantScope;
    fn reserve_pending_session(&self, tenant: &vw_api_transport::id::TenantScope, info: &session_info::Info) -> anyhow::Result<session_info::Info>;
    fn load_agent_session_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session_id: &str, scope: Option<&str>) -> Option<session_info::Info>;
    fn load_session_scoped_for_tenant_fallible(&self, tenant: &vw_api_transport::id::TenantScope, id: &str, scope: Option<&str>) -> anyhow::Result<Option<session_ui_types::ChatSession>>;
    fn has_committed_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, session_id: &str) -> anyhow::Result<bool>;
    fn commit_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, commit: session_store::FirstResponseCommit<'_>) -> anyhow::Result<bool>;
    fn save_session_ui_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session: &session_ui_types::ChatSession, scope: Option<&str>, preserve_calls: bool) -> anyhow::Result<std::path::PathBuf>;
    fn replace_session_ui_history_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, expected: &session_ui_types::ChatSession, session: &session_ui_types::ChatSession, scope: Option<&str>) -> anyhow::Result<()>;
    fn now_ms(&self) -> u64;
    fn create_slug(&self) -> String;
}
static HOST: OnceLock<&'static dyn PlanRunHost> = OnceLock::new();
pub fn install_host(host: &'static dyn PlanRunHost) -> Result<(), &'static str> {
    HOST.set(host).map_err(|_| "task-plan-runner host already installed")
}
pub(crate) fn host() -> &'static dyn PlanRunHost {
    *HOST.get().expect("task-plan-runner host must be installed before use")
}
pub(crate) fn recursive_orchestration_policy() -> &'static str {
    host().recursive_orchestration_policy()
}
