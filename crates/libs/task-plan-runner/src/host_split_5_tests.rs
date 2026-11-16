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
    crate::test_host::install();
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


