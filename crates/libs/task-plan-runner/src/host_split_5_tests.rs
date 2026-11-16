//! 根取消令牌与终止事件的领域契约。

use crate::{SessionRunRequest, SessionRunner, run_task_plan_with_cancellation};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use vw_api_task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

#[derive(Clone)]
struct CancellationGatedSessionRunner {
    started: Arc<Semaphore>,
}

impl Default for CancellationGatedSessionRunner {
    fn default() -> Self {
        Self { started: Arc::new(Semaphore::new(0)) }
    }
}

#[async_trait]
impl SessionRunner for CancellationGatedSessionRunner {
    async fn run_session(
        &self,
        _request: SessionRunRequest,
        _delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String, String> {
        self.started.add_permits(1);
        std::future::pending().await
    }
}

#[tokio::test]
async fn task_plan_run_root_cancellation_publishes_cancelled_once() {
    let temp = tempfile::tempdir().expect("tempdir");
    crate::install_host(&CancellationHost).expect("install cancellation fixture host");
    let runner = CancellationGatedSessionRunner::default();
    let started = Arc::clone(&runner.started);
    let cancellation = CancellationToken::new();
    let cancellation_for_run = cancellation.clone();
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected_events = Arc::clone(&events);
    let request = TaskPlanRunRequest {
        query: "run until stopped".to_string(),
        root: temp.path().to_string_lossy().to_string(),
        model: None,
        agent: None,
        acp_agent: Some("fixture".to_string()),
        parent_session_id: Some("main-session".to_string()),
    };

    let run = tokio::spawn(async move {
        run_task_plan_with_cancellation(
            "cancelled-plan-run",
            &request,
            &runner,
            cancellation_for_run,
            |event| {
                collected_events.lock().expect("events lock").push(event);
                true
            },
        )
        .await
    });

    tokio::time::timeout(std::time::Duration::from_millis(200), started.acquire())
        .await
        .expect("planning session should start")
        .expect("started semaphore should remain open")
        .forget();
    cancellation.cancel();
    tokio::time::timeout(std::time::Duration::from_millis(200), run)
        .await
        .expect("root cancellation should stop planning")
        .expect("plan-run task should join")
        .expect("cancellation is a terminal outcome");

    let events = events.lock().expect("events lock");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, TaskPlanRunStreamEventDto::Cancelled { .. }))
            .count(),
        1
    );
    assert!(matches!(
        events.last(),
        Some(TaskPlanRunStreamEventDto::Cancelled {
            task_id,
            session_id: None,
        }) if task_id == "cancelled-plan-run"
    ));
}


// 只替代宿主端口；取消发生前尚无成功回复，不允许访问回复存储。
struct CancellationHost;

impl crate::PlanRunHost for CancellationHost {
    fn recursive_orchestration_policy(&self) -> &'static str { "Cancellation fixture" }
    fn current_tenant_scope(&self) -> vw_api_transport::id::TenantScope {
        vw_api_transport::id::TenantScope::new("cancel-fixture", "cancel-fixture").expect("fixture tenant")
    }
    fn reserve_pending_session(&self, _tenant: &vw_api_transport::id::TenantScope, info: &session_info::Info) -> anyhow::Result<session_info::Info> {
        Ok(info.clone())
    }
    fn load_agent_session_scoped_for_tenant(&self, _tenant: &vw_api_transport::id::TenantScope, _session_id: &str, _scope: Option<&str>) -> Option<session_info::Info> {
        panic!("cancelled planning must not load response history")
    }
    fn load_session_scoped_for_tenant_fallible(&self, _tenant: &vw_api_transport::id::TenantScope, _id: &str, _scope: Option<&str>) -> anyhow::Result<Option<session_ui_types::ChatSession>> {
        anyhow::bail!("cancelled planning must not load response history")
    }
    fn has_committed_first_response(&self, _tenant: &vw_api_transport::id::TenantScope, _project_id: &str, _session_id: &str) -> anyhow::Result<bool> {
        anyhow::bail!("cancelled planning must not inspect response commits")
    }
    fn commit_first_response(&self, _tenant: &vw_api_transport::id::TenantScope, _project_id: &str, _commit: session_store::FirstResponseCommit<'_>) -> anyhow::Result<bool> {
        anyhow::bail!("cancelled planning must not commit a response")
    }
    fn save_session_ui_scoped_for_tenant(&self, _tenant: &vw_api_transport::id::TenantScope, _session: &session_ui_types::ChatSession, _scope: Option<&str>, _preserve_calls: bool) -> anyhow::Result<std::path::PathBuf> {
        anyhow::bail!("cancelled planning must not save response history")
    }
    fn now_ms(&self) -> u64 { 1 }
    fn create_slug(&self) -> String { "cancellation-fixture".to_string() }
}
