//! 两组执行器契约共用的宿主端口；计划文件和调度仍使用真实领域实现。
use std::collections::HashMap;
use std::sync::{Mutex, Once, OnceLock};

pub(crate) fn install() {
    static INIT: Once = Once::new();
    INIT.call_once(|| crate::install_host(&TestHost).expect("install plan-run test host"));
}

fn sessions() -> &'static Mutex<HashMap<String, session_info::Info>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, session_info::Info>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

struct TestHost;
impl crate::PlanRunHost for TestHost {
    fn recursive_orchestration_policy(&self) -> &'static str { "Plan-run fixture" }
    fn current_tenant_scope(&self) -> vw_api_transport::id::TenantScope {
        vw_api_transport::id::TenantScope::new("plan-fixture", "plan-fixture").expect("fixture tenant")
    }
    fn reserve_pending_session(&self, _tenant: &vw_api_transport::id::TenantScope, info: &session_info::Info) -> anyhow::Result<session_info::Info> {
        Ok(sessions().lock().expect("sessions lock").entry(info.id.clone()).or_insert_with(|| info.clone()).clone())
    }
    fn load_agent_session_scoped_for_tenant(&self, _tenant: &vw_api_transport::id::TenantScope, session_id: &str, scope: Option<&str>) -> Option<session_info::Info> {
        assert_ne!(session_id, "cancelled-plan-run-plan", "cancelled planning must not load response history");
        sessions().lock().expect("sessions lock").get(session_id).filter(|info| scope.is_none_or(|scope| scope == info.project_id)).cloned()
    }
    fn load_session_scoped_for_tenant_fallible(&self, _tenant: &vw_api_transport::id::TenantScope, _id: &str, _scope: Option<&str>) -> anyhow::Result<Option<session_ui_types::ChatSession>> {
        Ok(None)
    }
    fn has_committed_first_response(&self, _tenant: &vw_api_transport::id::TenantScope, _project_id: &str, _session_id: &str) -> anyhow::Result<bool> {
        Ok(false)
    }
    fn commit_first_response(&self, _tenant: &vw_api_transport::id::TenantScope, project_id: &str, commit: session_store::FirstResponseCommit<'_>) -> anyhow::Result<bool> {
        let sessions = sessions().lock().expect("sessions lock");
        anyhow::ensure!(sessions.get(&commit.snapshot.id).is_some_and(|info| info.project_id == project_id), "response requires a reserved session");
        Ok(true)
    }
    fn save_session_ui_scoped_for_tenant(&self, _tenant: &vw_api_transport::id::TenantScope, _session: &session_ui_types::ChatSession, _scope: Option<&str>, _preserve_calls: bool) -> anyhow::Result<std::path::PathBuf> {
        anyhow::bail!("fixture expects a pending first response")
    }
    fn now_ms(&self) -> u64 { 1 }
    fn create_slug(&self) -> String { "plan-run-fixture".to_string() }
}
