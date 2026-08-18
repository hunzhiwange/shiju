use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};
use vw_api_types::task::{
    TaskPlanRunRequest, TaskPlanRunStatus, TaskPlanRunStreamEventDto, TaskPoolMatrixActorDto,
    TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto, TaskPoolMatrixPublishRequest,
    TaskPoolStatus, TaskPoolWorktreeAcquireRequest, TaskPoolWorktreeMaintenanceRequest,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeUnlockRequest,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeReleaseRequest,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotStateDto,
    TaskPoolWorktreeSnapshotRequest,
};

use crate::client::test_support;
use crate::{GatewayStreamAttemptResult, GatewayStreamEvent, GatewayStreamResume};

#[tokio::test]
async fn task_plan_run_control_client_uses_status_and_stop_routes() {
    let tenant_scope =
        TenantScope::new("platform-control", "company-control").expect("tenant scope");
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({
                    "task_id": "task-control",
                    "status": "waiting",
                    "current_session_id": "task-control-1"
                }),
            ),
            (
                200,
                json!({
                    "task_id": "task-control",
                    "status": "cancelled",
                    "current_session_id": "task-control-1"
                }),
            ),
        ],
        tenant_scope,
    );

    let status = server.client().task_plan_run_status("task-control").await.expect("status");
    let stopped = server.client().task_plan_run_stop("task-control").await.expect("stop");

    assert_eq!(status.status, TaskPlanRunStatus::Waiting);
    assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
    let status_request = server.take_request();
    let stop_request = server.take_request();
    assert_eq!(status_request.method, "GET");
    assert_eq!(status_request.path, "/v1/task-plan-run/task-control/status");
    assert_eq!(stop_request.method, "POST");
    assert_eq!(stop_request.path, "/v1/task-plan-run/task-control/stop");
    assert_eq!(stop_request.body, json!({}));
    for request in [&status_request, &stop_request] {
        assert_eq!(request.header(PLATFORM_ID_HEADER), Some("platform-control"));
        assert_eq!(request.header(COMPANY_ID_HEADER), Some("company-control"));
    }
    server.join();
}

#[tokio::test]
async fn task_plan_run_stream_stop_uses_request_scoped_route() {
    let server = test_support::server(vec![(200, json!({ "cancelled": true }))]);

    let cancelled = server
        .client()
        .task_plan_run_stream_stop("desktop-plan-request")
        .await
        .expect("plan stream stop");

    assert!(cancelled);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/task-plan-run/stream/desktop-plan-request/stop");
    assert_eq!(request.body, json!({}));
    server.join();
}

#[tokio::test]
async fn task_plan_run_control_stream_treats_cancelled_as_terminal() {
    let cancelled = TaskPlanRunStreamEventDto::Cancelled {
        task_id: "task-control".to_string(),
        session_id: Some("task-control-1".to_string()),
    };
    let finished = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-control".to_string(),
        plan_dir: "/repo/.plans/task-control".to_string(),
        status: "completed".to_string(),
        output: "must not be delivered".to_string(),
        session_id: Some("task-control-1".to_string()),
    };
    let body = format!(
        "data: {}\n\ndata: {}\n\n",
        serde_json::to_string(&cancelled).expect("cancelled"),
        serde_json::to_string(&finished).expect("finished")
    );
    let server = test_support::server_raw(vec![(200, body)]);
    let request = TaskPlanRunRequest {
        query: "run plan".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let mut events = Vec::new();

    super::GatewayClient::stream_task_plan_run(server.client().endpoint(), &request, |event| {
        events.push(event);
        true
    })
    .await
    .expect("cancelled task plan stream");

    assert_eq!(events, vec![cancelled]);
    server.join();
}

#[tokio::test]
async fn task_plan_run_stream_treats_blocked_as_terminal() {
    let blocked = TaskPlanRunStreamEventDto::Blocked {
        task_id: "task-1".to_string(),
        batch: Some(1),
        subtask_id: Some("sub-1".to_string()),
        reason: "subtask boom".to_string(),
        session_id: "task-1-sub-1".to_string(),
    };
    let finished = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-1".to_string(),
        plan_dir: "/repo/.plans/task-1".to_string(),
        status: "completed".to_string(),
        output: "should not be delivered".to_string(),
        session_id: Some("task-1-plan".to_string()),
    };
    let body = format!(
        "data: {}\n\ndata: {}\n\n",
        serde_json::to_string(&blocked).expect("blocked"),
        serde_json::to_string(&finished).expect("finished")
    );
    let server = test_support::server_raw(vec![(200, body)]);
    let request = TaskPlanRunRequest {
        query: "run plan".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let mut events = Vec::new();

    super::GatewayClient::stream_task_plan_run(server.client().endpoint(), &request, |event| {
        events.push(event);
        true
    })
    .await
    .expect("task plan stream");

    assert_eq!(events, vec![blocked]);
    server.join();
}

#[tokio::test]
async fn task_plan_run_stream_sends_selected_tenant_headers() {
    let event = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-1".to_string(),
        plan_dir: "/repo/.plans/task-1".to_string(),
        status: "completed".to_string(),
        output: "done".to_string(),
        session_id: Some("task-1-plan".to_string()),
    };
    let tenant_scope = TenantScope::new("platform-task", "company-task").expect("tenant");
    let server = test_support::server_raw_with_tenant_scope(
        vec![(200, format!("data: {}\n\n", serde_json::to_string(&event).expect("event")))],
        tenant_scope,
    );
    let request = TaskPlanRunRequest {
        query: "run plan".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };
    let mut events = Vec::new();

    super::GatewayClient::stream_task_plan_run(server.client().endpoint(), &request, |event| {
        events.push(event);
        true
    })
    .await
    .expect("task plan stream");

    assert_eq!(events, vec![event]);
    let recorded = server.take_request();
    assert_eq!(recorded.path, "/v1/task-plan-run/stream");
    assert_eq!(recorded.header(PLATFORM_ID_HEADER), Some("platform-task"));
    assert_eq!(recorded.header(COMPANY_ID_HEADER), Some("company-task"));
    assert_eq!(recorded.body["parent_session_id"], "main-session");
    server.join();
}

#[tokio::test]
async fn task_plan_run_request_omits_removed_mode_field() {
    let event = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-goal".to_string(),
        plan_dir: "/repo/.plans/task-goal".to_string(),
        status: "completed".to_string(),
        output: "done".to_string(),
        session_id: Some("task-goal-goal-2".to_string()),
    };
    let server = test_support::server_raw(vec![(
        200,
        format!("data: {}\n\n", serde_json::to_string(&event).expect("event")),
    )]);
    let request = TaskPlanRunRequest {
        query: "reach goal".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };

    super::GatewayClient::stream_task_plan_run(server.client().endpoint(), &request, |_| true)
        .await
        .expect("task plan stream");

    let recorded = server.take_request();
    assert!(
        recorded.body.get("mode").is_none(),
        "serialized plan-run request must omit removed mode field"
    );
    assert_eq!(recorded.body["query"], "reach goal");
    assert_eq!(recorded.body["parent_session_id"], "main-session");
    server.join();
}

#[tokio::test]
async fn task_plan_resumable_stream_reports_incomplete_eof_and_terminal_event() {
    let progress = TaskPlanRunStreamEventDto::SessionDelta {
        task_id: "task-1".to_string(),
        session_id: "task-1-plan".to_string(),
        delta: "working".to_string(),
    };
    let finished = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-1".to_string(),
        plan_dir: "/repo/.plans/task-1".to_string(),
        status: "completed".to_string(),
        output: "done".to_string(),
        session_id: Some("task-1-plan".to_string()),
    };
    let server = test_support::server_raw(vec![
        (200, format!("id: 12\ndata: {}\n\n", serde_json::to_string(&progress).expect("progress"))),
        (200, format!("id: 13\ndata: {}\n\n", serde_json::to_string(&finished).expect("finished"))),
    ]);
    let request = TaskPlanRunRequest {
        query: "run plan".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let first_resume =
        GatewayStreamResume { request_id: "plan-request-1".to_string(), last_event_id: Some(11) };
    let mut progress_events = Vec::new();

    let incomplete = super::GatewayClient::stream_task_plan_run_resumable(
        server.client().endpoint(),
        &request,
        &first_resume,
        |event| {
            progress_events.push(event);
            true
        },
    )
    .await
    .expect("incomplete plan attempt");

    assert_eq!(incomplete, GatewayStreamAttemptResult::Incomplete);
    assert_eq!(progress_events, vec![GatewayStreamEvent { id: 12, event: progress }]);
    let first_request = server.take_request();
    assert_eq!(first_request.header("x-vw-stream-request-id"), Some("plan-request-1"));
    assert_eq!(first_request.header("last-event-id"), Some("11"));

    let second_resume =
        GatewayStreamResume { request_id: "plan-request-1".to_string(), last_event_id: Some(12) };
    let mut terminal_events = Vec::new();
    let terminated = super::GatewayClient::stream_task_plan_run_resumable(
        server.client().endpoint(),
        &request,
        &second_resume,
        |event| {
            terminal_events.push(event);
            true
        },
    )
    .await
    .expect("terminal plan attempt");

    assert_eq!(terminated, GatewayStreamAttemptResult::Terminated);
    assert_eq!(terminal_events, vec![GatewayStreamEvent { id: 13, event: finished }]);
    let second_request = server.take_request();
    assert_eq!(second_request.header("x-vw-stream-request-id"), Some("plan-request-1"));
    assert_eq!(second_request.header("last-event-id"), Some("12"));
    server.join();
}

#[tokio::test]
async fn task_plan_resumable_stream_reports_caller_stop() {
    let progress = TaskPlanRunStreamEventDto::SessionDelta {
        task_id: "task-1".to_string(),
        session_id: "task-1-plan".to_string(),
        delta: "working".to_string(),
    };
    let server = test_support::server_raw(vec![(
        200,
        format!("id: 1\ndata: {}\n\n", serde_json::to_string(&progress).expect("progress")),
    )]);
    let request = TaskPlanRunRequest {
        query: "run plan".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: None,
    };
    let resume =
        GatewayStreamResume { request_id: "plan-request-stop".to_string(), last_event_id: None };

    let stopped = super::GatewayClient::stream_task_plan_run_resumable(
        server.client().endpoint(),
        &request,
        &resume,
        |_| false,
    )
    .await
    .expect("caller-stopped plan attempt");

    assert_eq!(stopped, GatewayStreamAttemptResult::CallerStopped);
    server.join();
}

#[tokio::test]
async fn task_pool_matrix_publish_posts_event_and_events_get_uses_cursor() {
    let server = test_support::server(vec![
        (503, json!({"message": "warming up"})),
        (200, json!({"accepted": true, "matrix_event_id": "$matrix-event"})),
        (
            200,
            json!({
                "cursor": 7,
                "events": [{
                    "version": 1,
                    "event_id": "$event-1",
                    "task_id": "task-a",
                    "actor": {
                        "matrix_user_id": "@agent:example.test"
                    },
                    "occurred_at_ms": 1_716_000_000_000u64,
                    "kind": {
                        "type": "task_status_changed",
                        "from_status": "pool",
                        "to_status": "running"
                    }
                }]
            }),
        ),
    ]);
    let event = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$event-1".to_string(),
        task_id: "task-a".to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@agent:example.test".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1_716_000_000_000,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pool),
            to_status: TaskPoolStatus::Running,
        },
    };
    let request = TaskPoolMatrixPublishRequest {
        event: event.clone(),
        matrix_instance_id: Some("planner".to_string()),
    };

    let publish =
        server.client().task_pool_matrix_publish(&request).await.expect("publish matrix event");
    let events = server.client().task_pool_matrix_events(Some(6)).await.expect("get matrix events");

    assert!(publish.accepted);
    assert_eq!(publish.matrix_event_id, Some("$matrix-event".to_string()));
    assert_eq!(events.cursor, 7);
    assert_eq!(events.events, vec![event]);

    let first_publish = server.take_request();
    let second_publish = server.take_request();
    let list_events = server.take_request();
    assert_eq!(first_publish.method, "POST");
    assert_eq!(second_publish.method, "POST");
    assert_eq!(first_publish.path, "/v1/task-pool/matrix/events");
    assert_eq!(second_publish.path, "/v1/task-pool/matrix/events");
    assert_eq!(first_publish.body, second_publish.body);
    assert_eq!(second_publish.body["event"]["task_id"], "task-a");
    assert_eq!(second_publish.body["matrix_instance_id"], "planner");
    assert_eq!(list_events.method, "GET");
    assert_eq!(list_events.path, "/v1/task-pool/matrix/events?after_cursor=6");
    assert_eq!(list_events.body, serde_json::Value::Null);
    server.join();
}

#[tokio::test]
async fn task_pool_matrix_events_omits_cursor_when_not_provided() {
    let server = test_support::server(vec![(200, json!({"cursor": 0, "events": []}))]);

    let response = server
        .client()
        .task_pool_matrix_events(None)
        .await
        .expect("get matrix events without cursor");

    assert_eq!(response.cursor, 0);
    assert!(response.events.is_empty());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/task-pool/matrix/events");
    server.join();
}

#[tokio::test]
async fn task_pool_worktree_client_posts_expected_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "project_id": "project-1",
                "repo_root": "/repo",
                "pool_root": "/repo/.vibewindow/task-worktrees",
                "base_branch": "main",
                "idle_count": 1,
                "busy_count": 0,
                "tainted_count": 0,
                "recycling_count": 0,
                "dead_count": 0,
                "merge_target_locks": [],
                "slots": [{
                    "id": "slot-1",
                    "path": "/repo/.vibewindow/task-worktrees/slot-1",
                    "branch": "main",
                    "state": "idle"
                }]
            }),
        ),
        (200, json!({"idle_count": 2, "target_idle_count": 2})),
        (
            200,
            json!({
                "leased": true,
                "slot": {
                    "id": "slot-1",
                    "path": "/repo/.vibewindow/task-worktrees/slot-1",
                    "branch": "vw/task/task-1",
                    "state": "busy",
                    "leased_task_id": "task-1"
                }
            }),
        ),
        (200, json!({"released": true})),
        (200, json!({"recycled": true})),
        (200, json!({"recycled": true})),
        (200, json!({"locked": false, "holder_task_id": "task-2"})),
        (200, json!({"released": true})),
    ]);
    let snapshot_request = TaskPoolWorktreeSnapshotRequest { project_id: "project-1".into() };
    let maintenance_request =
        TaskPoolWorktreeMaintenanceRequest { project_id: "project-1".into(), desired_capacity: 2 };
    let acquire_request =
        TaskPoolWorktreeAcquireRequest { project_id: "project-1".into(), task_id: "task-1".into() };
    let release_request =
        TaskPoolWorktreeReleaseRequest { project_id: "project-1".into(), task_id: "task-1".into() };
    let recycle_request = TaskPoolWorktreeRecycleRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        taint_reason: Some("dirty worktree".to_string()),
    };
    let slot_recycle_request = TaskPoolWorktreeSlotRecycleRequest {
        project_id: "project-1".into(),
        slot_id: "slot-1".to_string(),
    };
    let merge_lock_request = TaskPoolWorktreeMergeLockRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        target_branch: "main".to_string(),
    };
    let merge_unlock_request = TaskPoolWorktreeMergeUnlockRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        target_branch: "main".to_string(),
    };

    let snapshot =
        server.client().task_pool_worktree_snapshot(&snapshot_request).await.expect("snapshot");
    let maintenance =
        server.client().task_pool_worktree_maintain(&maintenance_request).await.expect("maintain");
    let acquire =
        server.client().task_pool_worktree_acquire(&acquire_request).await.expect("acquire");
    let release =
        server.client().task_pool_worktree_release(&release_request).await.expect("release");
    let recycle =
        server.client().task_pool_worktree_recycle(&recycle_request).await.expect("recycle");
    let slot_recycle = server
        .client()
        .task_pool_worktree_slot_recycle(&slot_recycle_request)
        .await
        .expect("slot recycle");
    let merge_lock = server
        .client()
        .task_pool_worktree_merge_lock(&merge_lock_request)
        .await
        .expect("merge lock");
    let merge_unlock = server
        .client()
        .task_pool_worktree_merge_unlock(&merge_unlock_request)
        .await
        .expect("merge unlock");

    assert_eq!(snapshot.project_id.as_ref(), "project-1");
    assert_eq!(snapshot.slots[0].state, TaskPoolWorktreeSlotStateDto::Idle);
    assert_eq!(maintenance.target_idle_count, 2);
    assert!(acquire.leased);
    assert_eq!(acquire.slot.expect("leased slot").leased_task_id.expect("task").as_ref(), "task-1");
    assert!(release.released);
    assert!(recycle.recycled);
    assert!(slot_recycle.recycled);
    assert!(!merge_lock.locked);
    assert_eq!(merge_lock.holder_task_id.expect("holder").as_ref(), "task-2");
    assert!(merge_unlock.released);

    let recorded = [
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
        server.take_request(),
    ];
    let paths = recorded.iter().map(|request| request.path.as_str()).collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "/v1/task-pool/worktrees/snapshot",
            "/v1/task-pool/worktrees/maintain",
            "/v1/task-pool/worktrees/acquire",
            "/v1/task-pool/worktrees/release",
            "/v1/task-pool/worktrees/recycle",
            "/v1/task-pool/worktrees/slots/recycle",
            "/v1/task-pool/worktrees/merge-lock",
            "/v1/task-pool/worktrees/merge-unlock",
        ]
    );
    assert!(recorded.iter().all(|request| request.method == "POST"));
    assert_eq!(recorded[0].body["project_id"], "project-1");
    assert_eq!(recorded[1].body["desired_capacity"], 2);
    assert_eq!(recorded[2].body["task_id"], "task-1");
    assert_eq!(recorded[4].body["taint_reason"], "dirty worktree");
    assert_eq!(recorded[5].body["slot_id"], "slot-1");
    assert_eq!(recorded[6].body["target_branch"], "main");
    assert_eq!(recorded[7].body["target_branch"], "main");
    server.join();
}
