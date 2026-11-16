//! 宿主会话处理器适配；计划编排由 task-plan-runner 持有。
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use std::path::Path;
pub use task_plan_runner::{SessionRunRequest, SessionRunner};
/// 默认实现：走 gateway/session processor。
#[derive(Debug, Default, Clone)]
pub struct GatewaySessionRunner {
    tenant: vw_api_transport::TenantScope,
    parent_session_id: Option<String>,
}

impl GatewaySessionRunner {
    pub fn new(tenant: vw_api_transport::TenantScope, parent_session_id: Option<String>) -> Self {
        Self { tenant, parent_session_id }
    }

    pub(super) async fn acp_full_access(
        &self,
        request: &SessionRunRequest,
    ) -> anyhow::Result<bool> {
        let (Some(parent), Some(backend)) = (
            self.parent_session_id.as_deref().map(str::trim).filter(|id| !id.is_empty()),
            selected_acp_agent(request.acp_agent.as_deref()),
        ) else {
            return Ok(false);
        };
        let root = Path::new(&request.root).canonicalize()?;
        let (project, _) = crate::project::from_directory(&self.tenant, &root).await?;
        let tenant = self.tenant.clone();
        let parent_id = parent.to_owned();
        let project_id = project.id.clone();
        let parent_matches = crate::global_database::run_isolated(move || {
            crate::session::ui_store::load_agent_session_scoped_for_tenant(
                &tenant,
                &parent_id,
                Some(&project_id),
            )
            .is_some_and(|info| {
                info.time.archived.is_none()
                    && Path::new(&info.directory).canonicalize().ok().as_ref() == Some(&root)
            })
        })
        .await?;
        if !parent_matches {
            return Ok(false);
        }
        let binding = crate::session::agent_binding::load_scoped(&project.id, parent).await?;
        if !binding.is_some_and(|binding| {
            binding.validate_selection(request.agent.as_deref(), Some(backend)).is_ok()
        }) {
            return Ok(false);
        }
        crate::session::agent_binding::confirmed_full_access(&project.id, parent).await
    }
}

/// 构建规划/执行子会话的 processor options。
pub(crate) fn gateway_session_options(
    request: &SessionRunRequest,
    acp_full_access: bool,
) -> Map<String, Value> {
    let acp_agent = selected_acp_agent(request.acp_agent.as_deref());
    let mut options = Map::new();
    options.insert("full_access".to_string(), json!(acp_agent.is_none() || acp_full_access));
    options.insert("cwd".to_string(), json!(request.root.as_str()));
    options.insert("goal_mode".to_string(), json!(true));
    options.insert("workflow_mode".to_string(), json!(true));

    if let Some(agent) = request.agent.as_ref().filter(|value| !value.trim().is_empty()) {
        options.insert("agent".to_string(), json!(agent));
    }

    let acp_enabled = acp_agent.is_some();
    options.insert("acp_test".to_string(), json!(acp_enabled));
    if let Some(acp_agent) = acp_agent {
        options.insert("acp_agent".to_string(), json!(acp_agent));
        options.insert(
            "acp_permission_mode".to_string(),
            json!(if acp_full_access { "approve-all" } else { "approve-reads" }),
        );
        options.insert("acp_non_interactive_permissions".to_string(), json!("deny"));
        options.insert("acp_force_new_session".to_string(), json!(true));
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        options.insert("acp_history_strategy".to_string(), json!("discard"));
        options.insert("acp_history_recent_count".to_string(), json!(1));
    }

    options
}

fn selected_acp_agent(agent: Option<&str>) -> Option<&str> {
    agent.map(str::trim).filter(|agent| !agent.is_empty())
}

/// 为需要持久化应用会话工件的 plan-run 轮次预分配 assistant 消息 ID。
pub(super) fn preallocate_assistant_message_id() -> Result<String, String> {
    crate::id::ascending(crate::id::Prefix::Message, None).map_err(|error| error.to_string())
}

#[async_trait]
impl SessionRunner for GatewaySessionRunner {
    async fn run_session(
        &self,
        request: SessionRunRequest,
        delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        let acp_full_access =
            self.acp_full_access(&request).await.map_err(|error| error.to_string())?;
        let options = gateway_session_options(&request, acp_full_access);
        let acp = selected_acp_agent(request.acp_agent.as_deref()).is_some();
        let reliability =
            crate::config::get_project_for_tenant(&self.tenant, Path::new(&request.root))
                .await
                .map_err(|error| error.to_string())?
                .reliability;
        let assistant_message_id = preallocate_assistant_message_id()?;

        let processor_request = crate::app::agent::session::processor::Request {
            stream: crate::app::agent::session::session::now_ms(),
            session: request.session_id,
            query: request.query,
            root: Some(request.root),
            model: request.model,
            reliability,
            options: Value::Object(options),
            approval: None,
            // Reuse the gateway's host workspace guard without an interactive approval channel.
            channel_name: acp.then(|| "desktop".to_string()),
            non_cli_approval_context: None,
            assistant_message_id: Some(assistant_message_id),
            history: Vec::new(),
            persist_app_session_artifacts: true,
            app_session_scope: None,
            abort_token: None,
            session_tools: crate::session::runtime::SessionToolset::default(),
            session_skills: Default::default(),
        };

        crate::channels::session::run_session_processor_for_channel(processor_request, delta_tx)
            .await
            .map_err(|error| error.to_string())
    }
}


pub async fn run_task_plan<R: SessionRunner + ?Sized, F: FnMut(vw_api_task::TaskPlanRunStreamEventDto) -> bool>(task_id: &str, request: &vw_api_task::TaskPlanRunRequest, runner: &R, on_event: F) -> Result<(), String> {
    ensure_host(); task_plan_runner::run_task_plan(task_id, request, runner, on_event).await
}
pub async fn run_task_plan_with_cancellation<R: SessionRunner + ?Sized, F: FnMut(vw_api_task::TaskPlanRunStreamEventDto) -> bool>(task_id: &str, request: &vw_api_task::TaskPlanRunRequest, runner: &R, cancellation: tokio_util::sync::CancellationToken, on_event: F) -> Result<(), String> {
    ensure_host(); task_plan_runner::run_task_plan_with_cancellation(task_id, request, runner, cancellation, on_event).await
}
#[cfg(test)]
pub(super) async fn run_task_plan_in_home<R: SessionRunner + ?Sized, F: FnMut(vw_api_task::TaskPlanRunStreamEventDto) -> bool>(task_id: &str, request: &vw_api_task::TaskPlanRunRequest, runner: &R, home: &Path, on_event: F) -> Result<(), String> {
    ensure_host(); task_plan_runner::runner::run_task_plan_in_home(task_id, request, runner, home, on_event).await
}
struct Host;
impl task_plan_runner::PlanRunHost for Host {
    fn recursive_orchestration_policy(&self) -> &'static str { crate::agent::prompt::multi_agent::recursive_orchestration_policy() }
    fn current_tenant_scope(&self) -> vw_api_transport::id::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn reserve_pending_session(&self, tenant: &vw_api_transport::id::TenantScope, info: &crate::session::session::Info) -> anyhow::Result<crate::session::session::Info> { crate::session::ui_store::reserve_pending_session(tenant, info) }
    fn load_agent_session_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session_id: &str, scope: Option<&str>) -> Option<crate::session::session::Info> { crate::session::ui_store::load_agent_session_scoped_for_tenant(tenant, session_id, scope) }
    fn load_session_scoped_for_tenant_fallible(&self, tenant: &vw_api_transport::id::TenantScope, id: &str, scope: Option<&str>) -> anyhow::Result<Option<crate::session::ui_types::ChatSession>> { crate::session::ui_store::load_session_scoped_for_tenant_fallible(tenant, id, scope) }
    fn has_committed_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, session_id: &str) -> anyhow::Result<bool> { crate::session::ui_store::has_committed_first_response(tenant, project_id, session_id) }
    fn commit_first_response(&self, tenant: &vw_api_transport::id::TenantScope, project_id: &str, commit: crate::session::store::FirstResponseCommit<'_>) -> anyhow::Result<bool> { crate::session::ui_store::commit_first_response(tenant, project_id, commit) }
    fn save_session_ui_scoped_for_tenant(&self, tenant: &vw_api_transport::id::TenantScope, session: &crate::session::ui_types::ChatSession, scope: Option<&str>, preserve_calls: bool) -> anyhow::Result<std::path::PathBuf> { crate::session::ui_store::save_session_ui_scoped_for_tenant(tenant, session, scope, preserve_calls) }
    fn now_ms(&self) -> u64 { crate::session::session::now_ms() }
    fn create_slug(&self) -> String { crate::session::session::create_slug() }

}
pub(super) fn ensure_host() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| task_plan_runner::install_host(&Host).expect("install task-plan host"));
}
