//! 领域测试使用真实会话存储；策略端口不调用模型服务。
use std::sync::Once;

pub(crate) fn install() {
    static INIT: Once = Once::new();
    INIT.call_once(|| crate::install_host(&TestHost).expect("install plan-run test host"));
}

struct TestHost;
impl crate::PlanRunHost for TestHost {
    fn recursive_orchestration_policy(&self) -> &'static str { "Plan-run fixture" }
    fn current_tenant_scope(&self) -> vw_api_transport::id::TenantScope { session_store::ui_store::current_tenant_scope() }
    fn reserve_pending_session(&self, tenant: &vw_api_transport::id::TenantScope, info: &session_info::Info) -> anyhow::Result<session_info::Info> { session_store::ui_store::reserve_pending_session(tenant, info) }
    fn load_agent_session_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session_id: &str, scope: Option<&str>) -> Option<session_info::Info> { assert_ne!(session_id, "cancelled-plan-run-plan", "cancelled planning must not load response history"); session_store::ui_store::load_agent_session_scoped_for_tenant(tenant, session_id, scope) }
    fn load_session_scoped_for_tenant_fallible(&self, tenant: &vw_api_transport::id::TenantScope, id: &str, scope: Option<&str>) -> anyhow::Result<Option<session_ui_types::ChatSession>> { session_store::ui_store::load_session_scoped_for_tenant_fallible(tenant, id, scope) }
    fn has_committed_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, session_id: &str) -> anyhow::Result<bool> { session_store::ui_store::has_committed_first_response(tenant, project_id, session_id) }
    fn commit_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, commit: session_store::FirstResponseCommit<'_>) -> anyhow::Result<bool> { session_store::ui_store::commit_first_response(tenant, project_id, commit) }
    fn save_session_ui_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session: &session_ui_types::ChatSession, scope: Option<&str>, preserve_calls: bool) -> anyhow::Result<std::path::PathBuf> { session_store::ui_store::save_session_ui_scoped_for_tenant(tenant, session, scope, preserve_calls) }
    fn replace_session_ui_history_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, expected: &session_ui_types::ChatSession, session: &session_ui_types::ChatSession, scope: Option<&str>) -> anyhow::Result<()> { session_store::ui_store::replace_session_ui_history_scoped_for_tenant(tenant, expected, session, scope) }
    fn now_ms(&self) -> u64 { 1 }
    fn create_slug(&self) -> String { "plan-run-fixture".to_string() }
}
