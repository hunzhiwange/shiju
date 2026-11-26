use crate::{
    CommandTaskSpecDto, CreateTaskRequest, ListTasksRequest, TaskKind, TaskPlanRunStatus,
    TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto, TaskPoolStatus,
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorktreeAcquireRequest,
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeMaintenanceRequest,
    TaskPoolWorktreeMaintenanceResponse, TaskPoolWorktreeMergeLockDto,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeMergeUnlockRequest, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseRequest, TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotRecycleResponse,
    TaskPoolWorktreeSlotStateDto, TaskPoolWorktreeSnapshotRequest,
    TaskPoolWorktreeSnapshotResponse, TaskStatus,
};
use serde_json::json;

#[test]
fn task_requests_default_stream_and_filters() {
    let create: CreateTaskRequest = serde_json::from_value(json!({
        "project_id": "project-1",
        "kind": "command"
    }))
    .expect("valid create");
    assert!(!create.stream);
    assert_eq!(create.command, None);

    let list: ListTasksRequest = serde_json::from_value(json!({})).expect("valid list");
    assert_eq!(list.project_id, None);
    assert_eq!(list.status, None);

    let command = CommandTaskSpecDto {
        argv: vec!["echo".to_string()],
        cwd: ".".to_string(),
        env: Default::default(),
    };
    assert!(command.env.is_empty());
    assert_eq!(serde_json::to_value(TaskKind::Command).expect("serialize"), json!("command"));
    assert_eq!(serde_json::to_value(TaskStatus::Cancelled).expect("serialize"), json!("cancelled"));
}

#[test]
fn task_pool_status_keys_round_trip_for_all_builtin_statuses() {
    let statuses = [
        TaskPoolStatus::Pool,
        TaskPoolStatus::Pending,
        TaskPoolStatus::Planning,
        TaskPoolStatus::Running,
        TaskPoolStatus::Failed,
        TaskPoolStatus::Paused,
        TaskPoolStatus::CodeComplete,
        TaskPoolStatus::PrSubmitted,
        TaskPoolStatus::Completed,
        TaskPoolStatus::Archived,
    ];

    for status in statuses {
        assert_eq!(TaskPoolStatus::from_status_key(status.status_key()), Some(status));
    }
    assert_eq!(TaskPoolStatus::from_status_key("code_review"), Some(TaskPoolStatus::CodeComplete));
    assert_eq!(
        TaskPoolStatus::canonical_status_key("code_review", Some("dev_lifecycle")),
        "code_complete"
    );
    assert_eq!(
        TaskPoolStatus::canonical_status_key("code_review", Some("custom_workflow")),
        "code_review"
    );
    assert_eq!(TaskPoolStatus::from_status_key("custom_review"), None);
}

#[test]
fn task_worktree_pool_types_round_trip() {
    let slot = TaskPoolWorktreeSlotDto {
        agent_id: None,
        run_id: None,
        id: "slot-1".to_string(),
        path: "/repo/.vibewindow/task-worktrees/slot-1".to_string(),
        branch: "vw/task/task-1".to_string(),
        state: TaskPoolWorktreeSlotStateDto::Busy,
        leased_task_id: Some("task-1".into()),
        taint_reason: None,
    };
    let snapshot_request = TaskPoolWorktreeSnapshotRequest { project_id: "project-1".into() };
    let snapshot = TaskPoolWorktreeSnapshotResponse {
        project_id: "project-1".into(),
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.vibewindow/task-worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 1,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: vec![TaskPoolWorktreeMergeLockDto {
            target_branch: "main".to_string(),
            task_id: "task-1".into(),
            age_ms: 0,
        }],
        slots: vec![slot.clone()],
    };

    assert_eq!(
        serde_json::to_value(&snapshot_request).expect("serialize snapshot request"),
        json!({"project_id": "project-1"})
    );
    let encoded_snapshot = serde_json::to_value(&snapshot).expect("serialize snapshot");
    assert_eq!(encoded_snapshot["slots"][0]["state"], "busy");
    assert_eq!(encoded_snapshot["merge_target_locks"][0]["task_id"], "task-1");
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeSnapshotResponse>(encoded_snapshot)
            .expect("deserialize snapshot"),
        snapshot
    );

    let minimal_snapshot: TaskPoolWorktreeSnapshotResponse = serde_json::from_value(json!({
        "project_id": "project-1",
        "repo_root": "/repo",
        "pool_root": "/repo/.vibewindow/task-worktrees",
        "base_branch": "main"
    }))
    .expect("deserialize minimal snapshot");
    assert_eq!(minimal_snapshot.idle_count, 0);
    assert!(minimal_snapshot.merge_target_locks.is_empty());
    assert!(minimal_snapshot.slots.is_empty());

    let maintenance =
        TaskPoolWorktreeMaintenanceRequest { project_id: "project-1".into(), desired_capacity: 2 };
    let maintenance_response = TaskPoolWorktreeMaintenanceResponse {
        idle_count: 2,
        target_idle_count: 2,
        snapshot: Some(snapshot.clone()),
    };
    assert_eq!(
        serde_json::to_value(&maintenance).expect("serialize maintenance request"),
        json!({"project_id": "project-1", "desired_capacity": 2})
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeMaintenanceResponse>(json!({}))
            .expect("deserialize default maintenance response"),
        TaskPoolWorktreeMaintenanceResponse { idle_count: 0, target_idle_count: 0, snapshot: None }
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeMaintenanceResponse>(
            serde_json::to_value(&maintenance_response).expect("serialize maintenance response")
        )
        .expect("deserialize maintenance response"),
        maintenance_response
    );

    let acquire =
        TaskPoolWorktreeAcquireRequest { project_id: "project-1".into(), task_id: "task-1".into() };
    let acquire_response = TaskPoolWorktreeAcquireResponse { leased: true, slot: Some(slot) };
    assert_eq!(
        serde_json::to_value(&acquire).expect("serialize acquire request"),
        json!({"project_id": "project-1", "task_id": "task-1"})
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeAcquireResponse>(
            serde_json::to_value(&acquire_response).expect("serialize acquire response")
        )
        .expect("deserialize acquire response"),
        acquire_response
    );

    let release =
        TaskPoolWorktreeReleaseRequest { project_id: "project-1".into(), task_id: "task-1".into() };
    let recycle = TaskPoolWorktreeRecycleRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        taint_reason: Some("dirty worktree".to_string()),
    };
    let slot_recycle = TaskPoolWorktreeSlotRecycleRequest {
        project_id: "project-1".into(),
        slot_id: "slot-1".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&release).expect("serialize release request"),
        json!({"project_id": "project-1", "task_id": "task-1"})
    );
    assert_eq!(
        serde_json::to_value(&recycle).expect("serialize recycle request"),
        json!({
            "project_id": "project-1",
            "task_id": "task-1",
            "taint_reason": "dirty worktree"
        })
    );
    assert_eq!(
        serde_json::to_value(&slot_recycle).expect("serialize slot recycle request"),
        json!({"project_id": "project-1", "slot_id": "slot-1"})
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeReleaseResponse>(json!({"released": true}))
            .expect("deserialize release response"),
        TaskPoolWorktreeReleaseResponse { released: true }
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeRecycleResponse>(json!({"recycled": true}))
            .expect("deserialize recycle response"),
        TaskPoolWorktreeRecycleResponse { recycled: true }
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeSlotRecycleResponse>(json!({"recycled": true}))
            .expect("deserialize slot recycle response"),
        TaskPoolWorktreeSlotRecycleResponse { recycled: true }
    );

    let merge_lock = TaskPoolWorktreeMergeLockRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        target_branch: "main".to_string(),
    };
    let merge_unlock = TaskPoolWorktreeMergeUnlockRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        target_branch: "main".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&merge_lock).expect("serialize merge lock request"),
        json!({"project_id": "project-1", "task_id": "task-1", "target_branch": "main"})
    );
    assert_eq!(
        serde_json::to_value(&merge_unlock).expect("serialize merge unlock request"),
        json!({"project_id": "project-1", "task_id": "task-1", "target_branch": "main"})
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeMergeLockResponse>(
            json!({"locked": false, "holder_task_id": "task-2"})
        )
        .expect("deserialize merge lock response"),
        TaskPoolWorktreeMergeLockResponse { locked: false, holder_task_id: Some("task-2".into()) }
    );
    assert_eq!(
        serde_json::from_value::<TaskPoolWorktreeMergeUnlockResponse>(json!({"released": true}))
            .expect("deserialize merge unlock response"),
        TaskPoolWorktreeMergeUnlockResponse { released: true }
    );
}

#[test]
fn task_pool_workflow_snapshot_preserves_step_rules() {
    let dto: vw_api_task_lifecycle::TaskLifecycleWorkflowDto = serde_json::from_value(json!({
        "id": "content", "label": "Content", "built_in": false,
        "initial_status_key": "draft",
        "statuses": [
            {"key":"draft", "label":"Draft", "category":"unstarted"},
            {"key":"editing", "label":"Editing", "category":"started"},
            {"key":"published", "label":"Published", "category":"done"}
        ],
        "steps": [
            {"status_key":"draft", "handoff":{"kind":"member", "member_id":"alice"}},
            {"status_key":"editing", "handoff":{"kind":"agent", "agent_id":"editor", "instructions":"Review the draft", "return_status_key":"draft"}},
            {"status_key":"published", "handoff":{"kind":"keep_assignee"}}
        ]
    })).unwrap();
    let snapshot = TaskPoolWorkflowSnapshotDto::from(dto.clone());
    snapshot.validate_for_schedule().unwrap();
    assert_eq!(snapshot.statuses, dto.statuses);
    assert_eq!(snapshot.steps, dto.steps);
    assert_eq!(snapshot.completion_status_key("editing"), Some("published"));
    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(serde_json::from_value::<TaskPoolWorkflowSnapshotDto>(encoded).unwrap(), snapshot);
    let mut invalid = snapshot.clone();
    invalid.statuses[2].enabled = false;
    assert!(invalid.validate_for_schedule().unwrap_err().contains("target is disabled"));
    let mut invalid = snapshot;
    let crate::TaskPoolWorkflowStepHandoffDto::Agent { completion_status_key, .. } =
        &mut invalid.steps[1].handoff
    else {
        unreachable!()
    };
    *completion_status_key = Some("missing".into());
    assert!(invalid.validate_for_schedule().is_err());
}

#[test]
fn task_pool_workflow_snapshot_requires_step_configuration() {
    assert!(serde_json::from_value::<TaskPoolWorkflowSnapshotDto>(json!({
        "id":"legacy", "label":"Legacy", "statuses":[{"key":"running", "label":"Running", "role":"executing"}]
    })).is_err());
}

#[test]
fn task_plan_run_request_and_events_serialize_session_fields() {
    use crate::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

    let request = TaskPlanRunRequest {
        query: "plan it".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&request).expect("serialize request"),
        json!({
            "query": "plan it",
            "root": "/repo",
            "parent_session_id": "main-session",
        })
    );

    let without_parent: TaskPlanRunRequest = serde_json::from_value(json!({
        "query": "plan it",
        "root": "/repo",
    }))
    .expect("deserialize without parent_session_id");
    assert_eq!(without_parent.parent_session_id, None);

    let planning = TaskPlanRunStreamEventDto::PlanningStarted {
        task_id: "task-1".to_string(),
        plan_dir: "/plans/task-1".to_string(),
        session_id: "task-1-plan".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&planning).expect("serialize planning"),
        json!({
            "type": "planning_started",
            "task_id": "task-1",
            "plan_dir": "/plans/task-1",
            "session_id": "task-1-plan",
        })
    );

    let execution_delta = TaskPlanRunStreamEventDto::SessionDelta {
        task_id: "task-1".to_string(),
        session_id: "task-1-2".to_string(),
        delta: "working".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&execution_delta).expect("serialize execution delta"),
        json!({
            "type": "session_delta",
            "task_id": "task-1",
            "session_id": "task-1-2",
            "delta": "working",
        })
    );

    let waiting = TaskPlanRunStreamEventDto::SessionWaiting {
        task_id: "task-1".to_string(),
        session_id: "task-1-2".to_string(),
        active_descendant_session_ids: vec![
            "task-1-2-child".to_string(),
            "task-1-2-grandchild".to_string(),
        ],
    };
    assert_eq!(
        serde_json::to_value(&waiting).expect("serialize session waiting"),
        json!({
            "type": "session_waiting",
            "task_id": "task-1",
            "session_id": "task-1-2",
            "active_descendant_session_ids": [
                "task-1-2-child",
                "task-1-2-grandchild",
            ],
        })
    );
    assert_eq!(
        serde_json::from_value::<TaskPlanRunStreamEventDto>(
            serde_json::to_value(&waiting).expect("re-serialize session waiting"),
        )
        .expect("deserialize session waiting"),
        waiting
    );

    let batch_finished = TaskPlanRunStreamEventDto::BatchFinished {
        task_id: "task-1".to_string(),
        batch: 2,
        subtask_ids: vec!["2".to_string(), "3".to_string()],
        success: true,
    };
    assert_eq!(
        serde_json::to_value(&batch_finished).expect("serialize batch finished"),
        json!({
            "type": "batch_finished",
            "task_id": "task-1",
            "batch": 2,
            "subtask_ids": ["2", "3"],
            "success": true,
        })
    );

    let finished = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-1".to_string(),
        plan_dir: "/plans/task-1".to_string(),
        status: "completed".to_string(),
        output: "done".to_string(),
        session_id: Some("task-1-plan".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&finished).expect("serialize finished"),
        json!({
            "type": "finished",
            "task_id": "task-1",
            "plan_dir": "/plans/task-1",
            "status": "completed",
            "output": "done",
            "session_id": "task-1-plan",
        })
    );

    let finished_without_session: TaskPlanRunStreamEventDto = serde_json::from_value(json!({
        "type": "finished",
        "task_id": "task-1",
        "plan_dir": "/plans/task-1",
        "status": "completed",
        "output": "done",
    }))
    .expect("finished without session_id remains valid");
    assert!(matches!(
        finished_without_session,
        TaskPlanRunStreamEventDto::Finished { session_id: None, .. }
    ));
}

#[test]
fn task_plan_run_control_status_and_cancelled_event_roundtrip() {
    let cancelled = TaskPlanRunStreamEventDto::Cancelled {
        task_id: "task-control".to_string(),
        session_id: Some("task-control-1".to_string()),
    };
    let response = TaskPlanRunStatusResponse {
        task_id: "task-control".to_string(),
        status: TaskPlanRunStatus::Cancelled,
        current_session_id: Some("task-control-1".to_string()),
        terminal_event: Some(cancelled.clone()),
    };
    let encoded = serde_json::to_value(&response).expect("serialize control status");

    assert_eq!(
        encoded,
        json!({
            "task_id": "task-control",
            "status": "cancelled",
            "current_session_id": "task-control-1",
            "terminal_event": {
                "type": "cancelled",
                "task_id": "task-control",
                "session_id": "task-control-1"
            }
        })
    );
    assert_eq!(
        serde_json::from_value::<TaskPlanRunStatusResponse>(encoded)
            .expect("deserialize control status"),
        response
    );
    assert!(TaskPlanRunStatus::Completed.is_terminal());
    assert!(!TaskPlanRunStatus::Waiting.is_terminal());
}

#[test]
fn task_plan_run_request_ignores_legacy_mode_and_goal_events_round_trip() {
    use crate::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

    // 旧客户端仍可能发送 mode；网关侧已删除该参数，反序列化时忽略。
    let request: TaskPlanRunRequest = serde_json::from_value(json!({
        "query": "plan it",
        "root": "/repo",
        "mode": "workflow",
    }))
    .expect("legacy mode field is ignored");
    assert_eq!(request.query, "plan it");
    assert_eq!(request.root, "/repo");

    let goal = TaskPlanRunRequest {
        query: "finish with evidence".to_string(),
        root: "/repo".to_string(),
        model: None,
        agent: None,
        acp_agent: None,
        parent_session_id: Some("main-session".to_string()),
    };
    assert_eq!(
        serde_json::to_value(goal).expect("serialize goal request"),
        json!({
            "query": "finish with evidence",
            "root": "/repo",
            "parent_session_id": "main-session",
        })
    );

    let turn = TaskPlanRunStreamEventDto::GoalTurnStarted {
        task_id: "task-1".to_string(),
        turn: 2,
        session_id: "task-1-goal-2".to_string(),
        reason: "tests still fail".to_string(),
    };
    assert_eq!(
        serde_json::to_value(turn).expect("serialize goal turn"),
        json!({
            "type": "goal_turn_started",
            "task_id": "task-1",
            "turn": 2,
            "session_id": "task-1-goal-2",
            "reason": "tests still fail",
        })
    );

    let evaluated = TaskPlanRunStreamEventDto::GoalEvaluated {
        task_id: "task-1".to_string(),
        turn: 2,
        achieved: false,
        reason: "clippy exits 1".to_string(),
        session_id: "task-1-goal-2".to_string(),
    };
    let encoded = serde_json::to_value(&evaluated).expect("serialize goal evaluation");
    assert_eq!(encoded["type"], "goal_evaluated");
    assert_eq!(encoded["achieved"], false);
    assert_eq!(
        serde_json::from_value::<TaskPlanRunStreamEventDto>(encoded)
            .expect("deserialize goal evaluation"),
        evaluated
    );
}
