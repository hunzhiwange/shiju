use anyhow::Result;

use super::controller::{
    ControllerAdapter, EnsureWorkerReadyRequest, FakeControllerAdapter, WorkerControllerSnapshot,
    WorkerLifecycleRequest, WorkerLifecycleState, WorkerStatusRequest,
};
use crate::agent_scheduler::domain::{TeamId, WorkerName};

#[tokio::test]
async fn agent_controller_adapter_fake_records_worker_lifecycle_requests() -> Result<()> {
    let adapter = FakeControllerAdapter::new();
    adapter.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("worker-api")?,
        desired_state: WorkerLifecycleState::Running,
        runtime: Some("copaw".to_string()),
        team_id: Some(TeamId::new("team-alpha")?),
        worker_role: Some("worker".to_string()),
        room_id: Some("!worker-api-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let ready = adapter
        .ensure_worker_ready(EnsureWorkerReadyRequest {
            worker_name: WorkerName::new("worker-api")?,
            runtime: Some("copaw".to_string()),
            team_id: Some(TeamId::new("team-alpha")?),
            worker_role: Some("worker".to_string()),
        })
        .await?;
    assert_eq!(ready.room_id.as_deref(), Some("!worker-api-room:example.test"));
    assert_eq!(ready.container_state.as_deref(), Some("ready"));

    adapter
        .request_worker_sleep(WorkerLifecycleRequest {
            worker_name: WorkerName::new("worker-api")?,
            desired_state: WorkerLifecycleState::Sleeping,
            runtime: Some("copaw".to_string()),
            team_id: Some(TeamId::new("team-alpha")?),
            worker_role: Some("worker".to_string()),
        })
        .await?;
    adapter
        .request_worker_stop(WorkerLifecycleRequest {
            worker_name: WorkerName::new("worker-api")?,
            desired_state: WorkerLifecycleState::Stopped,
            runtime: Some("copaw".to_string()),
            team_id: Some(TeamId::new("team-alpha")?),
            worker_role: Some("worker".to_string()),
        })
        .await?;

    let status = adapter
        .worker_status(WorkerStatusRequest { worker_name: WorkerName::new("worker-api")? })
        .await?
        .expect("worker status");
    assert_eq!(status.room_id.as_deref(), Some("!worker-api-room:example.test"));

    let requests = adapter.recorded_lifecycle_requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].worker_name.as_str(), "worker-api");
    assert_eq!(requests[0].desired_state, WorkerLifecycleState::Running);
    assert_eq!(requests[0].runtime.as_deref(), Some("copaw"));
    assert_eq!(requests[0].team_id.as_ref().map(TeamId::as_str), Some("team-alpha"));
    assert_eq!(requests[1].desired_state, WorkerLifecycleState::Sleeping);
    assert_eq!(requests[2].desired_state, WorkerLifecycleState::Stopped);
    assert_eq!(requests[2].worker_role.as_deref(), Some("worker"));

    Ok(())
}

#[tokio::test]
async fn fake_controller_rejects_mismatched_lifecycle_states() -> Result<()> {
    let adapter = FakeControllerAdapter::new();

    let sleep_error = adapter
        .request_worker_sleep(WorkerLifecycleRequest {
            worker_name: WorkerName::new("worker-api")?,
            desired_state: WorkerLifecycleState::Stopped,
            runtime: None,
            team_id: None,
            worker_role: None,
        })
        .await
        .expect_err("sleep request must require Sleeping state");
    assert!(
        sleep_error
            .to_string()
            .contains("request worker sleep requires desired state Sleeping, got Stopped")
    );

    let stop_error = adapter
        .request_worker_stop(WorkerLifecycleRequest {
            worker_name: WorkerName::new("worker-api")?,
            desired_state: WorkerLifecycleState::Running,
            runtime: None,
            team_id: None,
            worker_role: None,
        })
        .await
        .expect_err("stop request must require Stopped state");
    assert!(
        stop_error
            .to_string()
            .contains("request worker stop requires desired state Stopped, got Running")
    );
    assert!(adapter.recorded_lifecycle_requests().is_empty());

    Ok(())
}

#[tokio::test]
async fn fake_controller_reports_missing_worker_and_records_status_request() -> Result<()> {
    let adapter = FakeControllerAdapter::new();

    let status = adapter
        .worker_status(WorkerStatusRequest { worker_name: WorkerName::new("missing-worker")? })
        .await?;

    assert_eq!(status, None);
    let requests = adapter.recorded_status_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].worker_name.as_str(), "missing-worker");

    Ok(())
}

#[tokio::test]
async fn fake_controller_builds_snapshot_from_lifecycle_request_when_worker_is_missing()
-> Result<()> {
    let adapter = FakeControllerAdapter::new();

    let snapshot = adapter
        .ensure_worker_ready(EnsureWorkerReadyRequest {
            worker_name: WorkerName::new("new-worker")?,
            runtime: Some("local".to_string()),
            team_id: Some(TeamId::new("team-beta")?),
            worker_role: Some("reviewer".to_string()),
        })
        .await?;

    assert_eq!(snapshot.worker_name.as_str(), "new-worker");
    assert_eq!(snapshot.desired_state, WorkerLifecycleState::Running);
    assert_eq!(snapshot.runtime.as_deref(), Some("local"));
    assert_eq!(snapshot.team_id.as_ref().map(TeamId::as_str), Some("team-beta"));
    assert_eq!(snapshot.worker_role.as_deref(), Some("reviewer"));
    assert_eq!(snapshot.room_id, None);
    assert_eq!(snapshot.container_state, None);

    Ok(())
}

#[tokio::test]
async fn fake_controller_fills_missing_snapshot_metadata_from_lifecycle_request() -> Result<()> {
    let adapter = FakeControllerAdapter::new();
    adapter.set_snapshot(WorkerControllerSnapshot {
        worker_name: WorkerName::new("partial-worker")?,
        desired_state: WorkerLifecycleState::Stopped,
        runtime: None,
        team_id: None,
        worker_role: None,
        room_id: Some("!partial-worker-room:example.test".to_string()),
        container_state: Some("ready".to_string()),
    });

    let snapshot = adapter
        .request_worker_sleep(WorkerLifecycleRequest {
            worker_name: WorkerName::new("partial-worker")?,
            desired_state: WorkerLifecycleState::Sleeping,
            runtime: Some("copaw".to_string()),
            team_id: Some(TeamId::new("team-gamma")?),
            worker_role: Some("implementer".to_string()),
        })
        .await?;

    assert_eq!(snapshot.desired_state, WorkerLifecycleState::Sleeping);
    assert_eq!(snapshot.runtime.as_deref(), Some("copaw"));
    assert_eq!(snapshot.team_id.as_ref().map(TeamId::as_str), Some("team-gamma"));
    assert_eq!(snapshot.worker_role.as_deref(), Some("implementer"));
    assert_eq!(snapshot.room_id.as_deref(), Some("!partial-worker-room:example.test"));
    assert_eq!(snapshot.container_state.as_deref(), Some("ready"));

    Ok(())
}
