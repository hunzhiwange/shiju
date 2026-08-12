use crate::task::{
    CommandTaskSpecDto, CreateTaskRequest, ListTasksRequest, TASK_POOL_MATRIX_EVENT_TYPE, TaskKind,
    TaskPoolMatrixActorDto, TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto,
    TaskPoolMatrixEventsResponse, TaskPoolMatrixPublishRequest, TaskPoolMatrixPublishResponse,
    TaskPoolStatus, TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto,
    TaskPoolWorkflowStatusRoleDto, TaskPoolWorkflowTransitionDto, TaskPoolWorktreeAcquireRequest,
    TaskPoolWorktreeAcquireResponse, TaskPoolWorktreeMaintenanceRequest,
    TaskPoolWorktreeMaintenanceResponse, TaskPoolWorktreeMergeLockDto,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeLockResponse,
    TaskPoolWorktreeMergeUnlockRequest, TaskPoolWorktreeMergeUnlockResponse,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeRecycleResponse,
    TaskPoolWorktreeReleaseRequest, TaskPoolWorktreeReleaseResponse, TaskPoolWorktreeSlotDto,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSlotRecycleResponse,
    TaskPoolWorktreeSlotStateDto, TaskPoolWorktreeSnapshotRequest,
    TaskPoolWorktreeSnapshotResponse, TaskStatus, TaskPlanRunStatus, TaskPlanRunStatusResponse,
    TaskPlanRunStreamEventDto,
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
fn task_pool_matrix_event_serializes_and_validates_payload_limits() {
    assert_eq!(TASK_POOL_MATRIX_EVENT_TYPE, "com.vibewindow.task_pool.event.v1");

    let actor = TaskPoolMatrixActorDto {
        matrix_user_id: "@agent:example.test".to_string(),
        display_name: Some("Agent One".to_string()),
    };
    let event = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$event-1".to_string(),
        task_id: "task-1".to_string(),
        actor,
        occurred_at_ms: 1_716_000_000_000,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pool),
            to_status: TaskPoolStatus::CodeReview,
        },
    };

    assert!(event.validate_for_matrix_room_event().is_ok());
    assert_eq!(
        serde_json::to_value(&event).expect("serialize status event"),
        json!({
            "version": 1,
            "event_id": "$event-1",
            "task_id": "task-1",
            "actor": {
                "matrix_user_id": "@agent:example.test",
                "display_name": "Agent One"
            },
            "occurred_at_ms": 1_716_000_000_000u64,
            "kind": {
                "type": "task_status_changed",
                "from_status": "pool",
                "to_status": "code_review"
            }
        })
    );

    let claimed =
        TaskPoolMatrixEventKindDto::TaskClaimed { assignee: "@worker:example.test".to_string() };
    assert_eq!(
        serde_json::to_value(claimed).expect("serialize claimed kind"),
        json!({
            "type": "task_claimed",
            "assignee": "@worker:example.test"
        })
    );

    let note = TaskPoolMatrixEventKindDto::TaskNote { message: "ready for review".to_string() };
    assert_eq!(
        serde_json::to_value(note).expect("serialize note kind"),
        json!({
            "type": "task_note",
            "message": "ready for review"
        })
    );

    let blocked =
        TaskPoolMatrixEventKindDto::TaskBlocked { reason: "waiting for dependency".to_string() };
    assert_eq!(
        serde_json::to_value(blocked).expect("serialize blocked kind"),
        json!({
            "type": "task_blocked",
            "reason": "waiting for dependency"
        })
    );

    let mut invalid = event.clone();
    invalid.event_id.clear();
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("event_id must not be empty".to_string())
    );

    let mut invalid = event.clone();
    invalid.task_id.clear();
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("task_id must not be empty".to_string())
    );

    let mut invalid = event.clone();
    invalid.actor.matrix_user_id.clear();
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("actor.matrix_user_id must not be empty".to_string())
    );

    let mut invalid = event.clone();
    invalid.version = 2;
    assert_eq!(invalid.validate_for_matrix_room_event(), Err("version must be 1".to_string()));

    let mut invalid = event;
    invalid.kind = TaskPoolMatrixEventKindDto::TaskNote { message: "x".repeat(1025) };
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("kind.message must be at most 1024 characters".to_string())
    );
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
        TaskPoolStatus::CodeReview,
        TaskPoolStatus::PrSubmitted,
        TaskPoolStatus::Completed,
        TaskPoolStatus::Archived,
    ];

    for status in statuses {
        assert_eq!(TaskPoolStatus::from_status_key(status.status_key()), Some(status));
    }
    assert_eq!(TaskPoolStatus::from_status_key("custom_review"), None);
}

#[test]
fn task_pool_matrix_event_rejects_oversized_optional_and_variant_text() {
    let base = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$event-oversized".to_string(),
        task_id: "task-oversized".to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@agent:example.test".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: None,
            to_status: TaskPoolStatus::Running,
        },
    };

    let mut invalid = base.clone();
    invalid.event_id = "x".repeat(1025);
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("event_id must be at most 1024 characters".to_string())
    );

    let mut invalid = base.clone();
    invalid.actor.display_name = Some("x".repeat(1025));
    assert_eq!(
        invalid.validate_for_matrix_room_event(),
        Err("actor.display_name must be at most 1024 characters".to_string())
    );

    for (kind, expected) in [
        (
            TaskPoolMatrixEventKindDto::TaskClaimed { assignee: "x".repeat(1025) },
            "kind.assignee must be at most 1024 characters",
        ),
        (
            TaskPoolMatrixEventKindDto::TaskBlocked { reason: "x".repeat(1025) },
            "kind.reason must be at most 1024 characters",
        ),
    ] {
        let invalid = TaskPoolMatrixEventDto { kind, ..base.clone() };
        assert_eq!(invalid.validate_for_matrix_room_event(), Err(expected.to_string()));
    }
}

#[test]
fn matrix_event_structural_validation_accepts_any_parseable_status_transition() {
    // API 层只做结构校验：from/to 只要是可解析的 enum 即通过，工作流是否允许该流转
    // 由桌面应用层结合本地任务上下文用状态机判定，网关与 DTO 无任务上下文不做伪校验。
    let actor = TaskPoolMatrixActorDto {
        matrix_user_id: "@agent:example.test".to_string(),
        display_name: None,
    };

    let workflow_illegal = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$illegal-transition".to_string(),
        task_id: "task-illegal".to_string(),
        actor: actor.clone(),
        occurred_at_ms: 1_716_000_000_300,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            // Running -> Pending 在开发生命周期工作流中非法，但 DTO 层不感知工作流规则。
            from_status: Some(TaskPoolStatus::Running),
            to_status: TaskPoolStatus::Pending,
        },
    };
    assert!(
        workflow_illegal.validate_for_matrix_room_event().is_ok(),
        "API 层不应以工作流规则拒绝结构合法的状态事件"
    );

    let missing_from = TaskPoolMatrixEventDto {
        event_id: "$missing-from".to_string(),
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: None,
            to_status: TaskPoolStatus::Archived,
        },
        ..workflow_illegal
    };
    assert!(
        missing_from.validate_for_matrix_room_event().is_ok(),
        "缺省 from_status 的状态事件在 API 层仍结构合法"
    );
}

#[test]
fn task_pool_matrix_instance_publish_request_accepts_matrix_instance_id() {
    let event = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$event-instance".to_string(),
        task_id: "task-instance".to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@agent:example.test".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1_716_000_000_200,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: Some(TaskPoolStatus::Pending),
            to_status: TaskPoolStatus::Running,
        },
    };

    let publish =
        TaskPoolMatrixPublishRequest { event, matrix_instance_id: Some("planner".to_string()) };

    let serialized = serde_json::to_value(&publish).expect("serialize named publish request");
    assert_eq!(serialized["matrix_instance_id"], "planner");

    let decoded: TaskPoolMatrixPublishRequest =
        serde_json::from_value(serialized).expect("deserialize named publish request");
    assert_eq!(decoded.matrix_instance_id.as_deref(), Some("planner"));
}

#[test]
fn task_pool_matrix_api_payloads_serialize() {
    let event = TaskPoolMatrixEventDto {
        version: 1,
        event_id: "$event-2".to_string(),
        task_id: "task-2".to_string(),
        actor: TaskPoolMatrixActorDto {
            matrix_user_id: "@agent:example.test".to_string(),
            display_name: None,
        },
        occurred_at_ms: 1_716_000_000_100,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: None,
            to_status: TaskPoolStatus::Running,
        },
    };

    let publish = TaskPoolMatrixPublishRequest { event: event.clone(), matrix_instance_id: None };
    assert_eq!(
        serde_json::to_value(&publish).expect("serialize publish request"),
        json!({
            "event": {
                "version": 1,
                "event_id": "$event-2",
                "task_id": "task-2",
                "actor": {
                    "matrix_user_id": "@agent:example.test"
                },
                "occurred_at_ms": 1_716_000_000_100u64,
                "kind": {
                    "type": "task_status_changed",
                    "to_status": "running"
                }
            }
        })
    );

    let named_instance_publish = TaskPoolMatrixPublishRequest {
        event: event.clone(),
        matrix_instance_id: Some("planner".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&named_instance_publish).expect("serialize named publish request"),
        json!({
            "event": {
                "version": 1,
                "event_id": "$event-2",
                "task_id": "task-2",
                "actor": {
                    "matrix_user_id": "@agent:example.test"
                },
                "occurred_at_ms": 1_716_000_000_100u64,
                "kind": {
                    "type": "task_status_changed",
                    "to_status": "running"
                }
            },
            "matrix_instance_id": "planner"
        })
    );

    let decoded_legacy: TaskPoolMatrixPublishRequest =
        serde_json::from_value(json!({ "event": named_instance_publish.event }))
            .expect("legacy publish request should decode");
    assert_eq!(decoded_legacy.matrix_instance_id, None);

    let accepted = TaskPoolMatrixPublishResponse {
        accepted: true,
        matrix_event_id: Some("$matrix-event".to_string()),
    };
    assert_eq!(
        serde_json::to_value(accepted).expect("serialize publish response"),
        json!({
            "accepted": true,
            "matrix_event_id": "$matrix-event"
        })
    );

    let events = TaskPoolMatrixEventsResponse { cursor: 42, events: vec![event] };
    assert_eq!(
        serde_json::to_value(events).expect("serialize events response"),
        json!({
            "cursor": 42,
            "events": [{
                "version": 1,
                "event_id": "$event-2",
                "task_id": "task-2",
                "actor": {
                    "matrix_user_id": "@agent:example.test"
                },
                "occurred_at_ms": 1_716_000_000_100u64,
                "kind": {
                    "type": "task_status_changed",
                    "to_status": "running"
                }
            }]
        })
    );
}

#[test]
fn task_worktree_pool_types_round_trip() {
    let slot = TaskPoolWorktreeSlotDto {
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
fn task_pool_workflow_snapshot_round_trips_roles_and_routes() {
    let snapshot = TaskPoolWorkflowSnapshotDto {
        id: "content_pipeline".to_string(),
        label: "Content Pipeline".to_string(),
        statuses: vec![
            TaskPoolWorkflowStatusDto {
                key: "content_queued".to_string(),
                label: "Queued".to_string(),
                role: TaskPoolWorkflowStatusRoleDto::Queued,
                enabled: true,
                terminal: false,
                next: Some("drafting".to_string()),
                transitions: Vec::new(),
            },
            TaskPoolWorkflowStatusDto {
                key: "drafting".to_string(),
                label: "Drafting".to_string(),
                role: TaskPoolWorkflowStatusRoleDto::Executing,
                enabled: true,
                terminal: false,
                next: None,
                transitions: vec![
                    TaskPoolWorkflowTransitionDto {
                        outcome: "succeed".to_string(),
                        label: Some("Ready".to_string()),
                        target: "review".to_string(),
                    },
                    TaskPoolWorkflowTransitionDto {
                        outcome: "failed".to_string(),
                        label: None,
                        target: "content_queued".to_string(),
                    },
                ],
            },
            TaskPoolWorkflowStatusDto {
                key: "review".to_string(),
                label: "Review".to_string(),
                role: TaskPoolWorkflowStatusRoleDto::Review,
                enabled: true,
                terminal: true,
                next: None,
                transitions: Vec::new(),
            },
        ],
    };

    snapshot.validate_for_schedule().expect("workflow snapshot should validate");
    let encoded = serde_json::to_value(&snapshot).expect("serialize workflow snapshot");
    assert_eq!(encoded["statuses"][0]["role"], "queued");
    assert_eq!(encoded["statuses"][0]["next"], "drafting");
    assert_eq!(encoded["statuses"][1]["transitions"][0]["target"], "review");
    assert_eq!(TaskPoolStatus::from_status_key("content_writing"), None);
    assert_eq!(
        serde_json::from_value::<TaskPoolWorkflowSnapshotDto>(encoded)
            .expect("deserialize workflow snapshot"),
        snapshot
    );

    let minimal: TaskPoolWorkflowSnapshotDto = serde_json::from_value(json!({
        "id": "custom_flow",
        "label": "Custom Flow",
        "statuses": [{
            "key": "content_writing",
            "label": "Writing",
            "role": "custom"
        }]
    }))
    .expect("deserialize minimal workflow snapshot");
    assert!(minimal.validate_for_schedule().is_ok());
    assert!(minimal.statuses[0].enabled);
    assert!(!minimal.statuses[0].terminal);
    assert!(minimal.statuses[0].next.is_none());
    assert!(minimal.statuses[0].transitions.is_empty());

    let mut invalid = snapshot.clone();
    invalid.statuses[0].key.clear();
    assert!(
        invalid
            .validate_for_schedule()
            .expect_err("empty status key should fail")
            .contains("status key must not be empty")
    );

    let mut invalid = snapshot;
    invalid.statuses[1].transitions[0].target = "missing".to_string();
    assert!(
        invalid
            .validate_for_schedule()
            .expect_err("unknown target should fail")
            .contains("unknown workflow transition target: missing")
    );
}

#[test]
fn task_pool_workflow_snapshot_rejects_invalid_identity_and_empty_routes() {
    let status = TaskPoolWorkflowStatusDto {
        key: "queued".to_string(),
        label: "Queued".to_string(),
        role: TaskPoolWorkflowStatusRoleDto::Queued,
        enabled: true,
        terminal: false,
        next: None,
        transitions: Vec::new(),
    };
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "workflow".to_string(),
        label: "Workflow".to_string(),
        statuses: vec![status.clone()],
    };

    let mut invalid = workflow.clone();
    invalid.id = " ".to_string();
    assert_eq!(invalid.validate_for_schedule(), Err("workflow id must not be empty".to_string()));

    let mut invalid = workflow.clone();
    invalid.statuses.push(status);
    assert_eq!(
        invalid.validate_for_schedule(),
        Err("workflow status key must be unique: queued".to_string())
    );

    let mut invalid = workflow.clone();
    invalid.statuses[0].next = Some(" ".to_string());
    assert_eq!(
        invalid.validate_for_schedule(),
        Err("workflow next target must not be empty".to_string())
    );

    let mut invalid = workflow.clone();
    invalid.statuses[0].transitions.push(TaskPoolWorkflowTransitionDto {
        outcome: " ".to_string(),
        label: None,
        target: "queued".to_string(),
    });
    assert_eq!(
        invalid.validate_for_schedule(),
        Err("workflow transition outcome must not be empty: queued".to_string())
    );

    let mut invalid = workflow;
    invalid.statuses[0].transitions.push(TaskPoolWorkflowTransitionDto {
        outcome: "retry".to_string(),
        label: None,
        target: "\t".to_string(),
    });
    assert_eq!(
        invalid.validate_for_schedule(),
        Err("workflow transition target must not be empty".to_string())
    );
}

#[test]
fn task_plan_run_request_and_events_serialize_session_fields() {
    use crate::task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

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
    use crate::task::{TaskPlanRunRequest, TaskPlanRunStreamEventDto};

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
