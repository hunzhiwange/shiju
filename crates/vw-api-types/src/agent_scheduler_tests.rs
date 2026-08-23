//! 验证代理调度器 DTO 的线协议、作用域校验及新旧载荷兼容性。

use super::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentEventDto, AgentEventKindDto,
    AgentOutboxMessageDto, AgentOutboxMessageStatusDto, AgentResourceLeaseDto,
    AgentResourceLeaseStatusDto, AgentRunDto, AgentRunStatusDto, AgentSchedulerBulkCommandRequest,
    AgentSchedulerBulkCommandResponse, AgentSchedulerCommandKindDto,
    AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest, AgentSchedulerCommandResponse,
    AgentSchedulerCreateTaskRequest, AgentSchedulerCreateTaskResponse,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse, AgentSchedulerLegacyTaskImportDto,
    AgentSchedulerMatrixDispatchEventDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerOutboxDrainRequest, AgentSchedulerOutboxDrainResponse,
    AgentSchedulerReplaceTaskGraphRequest, AgentSchedulerScope,
    AgentSchedulerStaffCommandPayloadDto, AgentSchedulerStageActionCommandPayloadDto,
    AgentSchedulerStageActionInputsDto, AgentSchedulerStageActionOutcomeDto,
    AgentSchedulerStageCapabilityIoSummaryDto, AgentSchedulerStageCapabilityKindDto,
    AgentSchedulerTaskArtifactDto, AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventDto, AgentSchedulerTaskBoardEventsResponse,
    AgentSchedulerTaskBoardSnapshotReadModelDto, AgentSchedulerTaskBoardStaffAssignmentDto,
    AgentSchedulerTaskBoardStageActionCapabilityDto, AgentSchedulerTaskBoardStageActionDto,
    AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto,
    AgentSchedulerTaskBoardTaskReadModelDto, AgentSchedulerTaskBoardWorkflowResponse,
    AgentSchedulerTaskGraphEdgeDto, AgentSchedulerTaskGraphReadinessDto,
    AgentSchedulerTaskGraphResponse, AgentSchedulerTaskGraphTaskDto, AgentSchedulerTaskLogDto,
    AgentSchedulerTaskLogsResponse, AgentSchedulerTaskMetadataPatchDto,
    AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerTaskMetadataUpdateResponse,
    AgentSchedulerTeamsResponse, AgentSchedulerUpsertTeamRequest, AgentSchedulerUpsertTeamResponse,
    AgentSchedulerWakeRequest, AgentSchedulerWakeResponse, AgentSchedulerWorkspaceContextDto,
    AgentTaskDto, AgentTaskKindDto, AgentTaskStatusDto, AgentTeamDto, AgentTeamMemberDto,
    AgentTeamStatusDto, AgentWorkerDto, AgentWorkerStatusDto, TeamTaskDagNodeDto,
};
use super::chat::ReasoningEffortDto;
use super::common::TimestampMs;
use super::id::{
    CompanyId, DEFAULT_COMPANY_ID, DEFAULT_PLATFORM_ID, PlatformId, ProjectId, ProjectScope,
    TenantScope,
};
use super::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
};
use serde_json::json;
use std::collections::BTreeMap;

// 图快照 fixture 同时覆盖依赖边方向、阻塞原因以及任务就绪状态的往返序列化。
#[test]
fn agent_scheduler_task_graph_contract_roundtrips_success_dependencies_and_readiness() {
    let edge = AgentSchedulerTaskGraphEdgeDto {
        predecessor_task_id: "task/prepare".to_string(),
        successor_task_id: "task/review".to_string(),
    };
    let replace_request = AgentSchedulerReplaceTaskGraphRequest { edges: vec![edge.clone()] };
    let snapshot = AgentSchedulerTaskGraphResponse {
        project_id: "project/one".into(),
        edges: vec![edge],
        tasks: vec![AgentSchedulerTaskGraphTaskDto {
            task_id: "task/review".to_string(),
            status: AgentTaskStatusDto::Queued,
            readiness: AgentSchedulerTaskGraphReadinessDto::BlockedByDependency,
            waiting_task_ids: vec!["task/approve".to_string()],
            blocking_task_ids: vec!["task/prepare".to_string()],
        }],
    };

    let replace_json = serde_json::to_value(&replace_request).expect("serialize graph replace");
    assert_eq!(replace_json["edges"][0]["predecessor_task_id"], "task/prepare");
    assert_eq!(replace_json["edges"][0]["successor_task_id"], "task/review");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerReplaceTaskGraphRequest>(replace_json)
            .expect("decode graph replace"),
        replace_request
    );

    let snapshot_json = serde_json::to_value(&snapshot).expect("serialize graph snapshot");
    assert_eq!(snapshot_json["tasks"][0]["status"], "queued");
    assert_eq!(snapshot_json["tasks"][0]["readiness"], "blocked_by_dependency");
    assert_eq!(snapshot_json["tasks"][0]["waiting_task_ids"][0], "task/approve");
    assert_eq!(snapshot_json["tasks"][0]["blocking_task_ids"][0], "task/prepare");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerTaskGraphResponse>(snapshot_json)
            .expect("decode graph snapshot"),
        snapshot
    );
}

// 推理强度既要支持当前枚举，也要兼容缺少该字段的旧任务导入载荷。
#[test]
fn task_reasoning_effort_dto_roundtrips() {
    for effort in [
        None,
        Some(ReasoningEffortDto::None),
        Some(ReasoningEffortDto::High),
        Some(ReasoningEffortDto::Xhigh),
        Some(ReasoningEffortDto::Max),
    ] {
        let patch =
            AgentSchedulerTaskMetadataPatchDto { reasoning_effort: effort, ..Default::default() };
        let encoded = serde_json::to_value(&patch).expect("serialize task metadata patch");
        assert_eq!(
            encoded.get("reasoning_effort").cloned(),
            effort.map(|value| serde_json::to_value(value).expect("serialize effort"))
        );
        let decoded: AgentSchedulerTaskMetadataPatchDto =
            serde_json::from_value(encoded).expect("deserialize task metadata patch");
        assert_eq!(decoded.reasoning_effort, effort);
    }

    let legacy: AgentSchedulerLegacyTaskImportDto = serde_json::from_value(json!({
        "task_id": "legacy-task",
        "title": "Legacy task",
        "created_at_ms": 1,
        "reasoning_effort": "high"
    }))
    .expect("deserialize legacy task reasoning effort");
    assert_eq!(legacy.reasoning_effort, Some(ReasoningEffortDto::High));

    let old: AgentSchedulerLegacyTaskImportDto = serde_json::from_value(json!({
        "task_id": "old-task",
        "title": "Old task",
        "created_at_ms": 1
    }))
    .expect("old task without reasoning_effort remains compatible");
    assert_eq!(old.reasoning_effort, None);

    let error = serde_json::from_value::<AgentSchedulerLegacyTaskImportDto>(json!({
        "task_id": "invalid-task",
        "title": "Invalid task",
        "created_at_ms": 1,
        "reasoning_effort": "extreme"
    }))
    .expect_err("invalid reasoning_effort must be rejected");
    assert!(error.to_string().contains("reasoning_effort"));
}

// 作用域构造器和反序列化必须拒绝空标识，同时保留默认租户的固定种子值。
#[test]
fn tenant_and_project_scopes_require_non_empty_ids() {
    assert!(PlatformId::new(" ").is_err());
    assert!(CompanyId::new("\t").is_err());
    assert!(TenantScope::new("", "company-a").is_err());
    assert!(TenantScope::new("platform-a", "\n").is_err());

    let tenant = TenantScope::new("platform-a", "company-a").expect("valid tenant scope");
    assert_eq!(tenant.platform_id().as_ref(), "platform-a");
    assert_eq!(tenant.company_id().as_ref(), "company-a");

    assert!(ProjectScope::new(tenant.clone(), ProjectId::from(" ")).is_err());
    let project = ProjectScope::new(tenant.clone(), ProjectId::from("project-a"))
        .expect("valid project scope");
    assert_eq!(project.tenant_scope(), &tenant);
    assert_eq!(project.project_id().as_ref(), "project-a");

    let scheduler = AgentSchedulerScope::new(project.clone());
    assert_eq!(scheduler.project_scope(), &project);
    let encoded = serde_json::to_value(&scheduler).expect("serialize scheduler scope");
    assert_eq!(encoded["platform_id"], "platform-a");
    assert_eq!(encoded["company_id"], "company-a");
    assert_eq!(encoded["project_id"], "project-a");

    let local = AgentSchedulerScope::for_default_tenant(ProjectId::from("project-local"))
        .expect("valid local scheduler scope");
    assert_eq!(local.project_scope().platform_id().as_ref(), DEFAULT_PLATFORM_ID);
    assert_eq!(local.project_scope().company_id().as_ref(), DEFAULT_COMPANY_ID);
}

// Matrix 调度事件 fixture 覆盖版本、必填项、可选文本和字符数上限四类边界。
#[test]
fn matrix_dispatch_event_validation_accepts_valid_content_and_rejects_invalid_text() {
    let valid = AgentSchedulerMatrixDispatchEventDto {
        event_version: 1,
        platform_id: "platform-a".to_string(),
        company_id: "company-a".to_string(),
        project_id: "project-a".to_string(),
        task_id: "task-a".to_string(),
        run_id: "run-a".to_string(),
        worker_id: "worker-a".to_string(),
        worker_role: "worker".to_string(),
        room_id: "!room:example.test".to_string(),
        matrix_user_id: "@worker:example.test".to_string(),
        model: "gpt-5".to_string(),
        model_provider: None,
        agent: None,
        acp_agent: None,
        reasoning_effort: None,
        spec_path: "/tasks/task-a/spec.md".to_string(),
        metadata_path: "/tasks/task-a/meta.json".to_string(),
        created_at_ms: TimestampMs(1),
    };

    assert!(valid.validate_for_matrix_room_event().is_ok());

    let mut wrong_version = valid.clone();
    wrong_version.event_version = 2;
    assert_eq!(
        wrong_version.validate_for_matrix_room_event().expect_err("event version"),
        "event_version must be 1"
    );

    let mut missing_required = valid.clone();
    missing_required.worker_id.clear();
    assert_eq!(
        missing_required.validate_for_matrix_room_event().expect_err("worker id"),
        "worker_id must not be empty"
    );

    let mut empty_optional = valid.clone();
    empty_optional.model_provider = Some(String::new());
    assert_eq!(
        empty_optional.validate_for_matrix_room_event().expect_err("model provider"),
        "model_provider must not be empty"
    );

    let mut oversized = valid;
    oversized.spec_path = "界".repeat(1025);
    assert_eq!(
        oversized.validate_for_matrix_room_event().expect_err("spec path limit"),
        "spec_path must be at most 1024 characters"
    );
}

// 用有限、周期和团队 DAG 任务 fixture 验证调度领域快照共享同一稳定线协议。
#[test]
fn agent_scheduler_dto_round_trips_task_run_worker_team_and_outbox() {
    let finite_task = AgentTaskDto {
        id: "task-20260702-100001".to_string(),
        title: "Implement scheduler API".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: AgentTaskStatusDto::Assigned,
        assigned_to: Some("worker-a".to_string()),
        team_id: None,
        room_id: Some("!worker-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        created_at_ms: TimestampMs(1_783_000_000_000),
        updated_at_ms: TimestampMs(1_783_000_000_010),
        metadata: json!({"meta_path": "shared/tasks/task-20260702-100001/meta.json"}),
    };
    let finite_json = serde_json::to_value(&finite_task).expect("finite task");
    assert_eq!(finite_json["kind"], "finite");
    assert_eq!(finite_json["assigned_to"], "worker-a");
    assert_eq!(
        serde_json::from_value::<AgentTaskDto>(finite_json).expect("decode finite task"),
        finite_task
    );

    let infinite_task = AgentTaskDto {
        id: "task-20260702-100002".to_string(),
        title: "Monitor queue".to_string(),
        kind: AgentTaskKindDto::Infinite,
        status: AgentTaskStatusDto::Active,
        assigned_to: Some("worker-b".to_string()),
        team_id: None,
        room_id: Some("!worker-b-room:example.test".to_string()),
        project_room_id: None,
        delegated_to_team: None,
        schedule: Some("*/15 * * * *".to_string()),
        timezone: Some("Asia/Shanghai".to_string()),
        next_scheduled_at: Some("2026-07-02T10:15:00+08:00".to_string()),
        last_executed_at: Some("2026-07-02T10:00:00+08:00".to_string()),
        dag: Vec::new(),
        created_at_ms: TimestampMs(1_783_000_100_000),
        updated_at_ms: TimestampMs(1_783_000_100_010),
        metadata: json!({}),
    };
    let infinite_json = serde_json::to_value(&infinite_task).expect("infinite task");
    assert_eq!(infinite_json["kind"], "infinite");
    assert_eq!(infinite_json["schedule"], "*/15 * * * *");
    assert_eq!(
        serde_json::from_value::<AgentTaskDto>(infinite_json).expect("decode infinite task"),
        infinite_task
    );

    let team_task = AgentTaskDto {
        id: "task-20260702-100003".to_string(),
        title: "Ship team project".to_string(),
        kind: AgentTaskKindDto::TeamDag,
        status: AgentTaskStatusDto::Assigned,
        assigned_to: Some("leader-a".to_string()),
        team_id: Some("alpha-team".to_string()),
        room_id: Some("!leader-room:example.test".to_string()),
        project_room_id: None,
        delegated_to_team: Some("alpha-team".to_string()),
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: vec![
            TeamTaskDagNodeDto {
                task_id: "team-task-1".to_string(),
                title: "Build API".to_string(),
                depends_on: Vec::new(),
                assigned_to: Some("worker-api".to_string()),
                status: AgentTaskStatusDto::Running,
            },
            TeamTaskDagNodeDto {
                task_id: "team-task-2".to_string(),
                title: "Verify UI".to_string(),
                depends_on: vec!["team-task-1".to_string()],
                assigned_to: Some("worker-ui".to_string()),
                status: AgentTaskStatusDto::Queued,
            },
        ],
        created_at_ms: TimestampMs(1_783_000_200_000),
        updated_at_ms: TimestampMs(1_783_000_200_010),
        metadata: json!({"mode": "project"}),
    };
    let team_task_json = serde_json::to_value(&team_task).expect("team task");
    assert_eq!(team_task_json["kind"], "team_dag");
    assert_eq!(team_task_json["dag"][1]["depends_on"][0], "team-task-1");
    assert_eq!(
        serde_json::from_value::<AgentTaskDto>(team_task_json).expect("decode team task"),
        team_task
    );

    let run = AgentRunDto {
        id: "run-1".to_string(),
        task_id: finite_task.id.clone(),
        worker_id: Some("worker-a".to_string()),
        status: AgentRunStatusDto::Leased,
        lease_expires_at_ms: Some(TimestampMs(1_783_000_300_000)),
        attempt: 2,
        started_at_ms: Some(TimestampMs(1_783_000_210_000)),
        finished_at_ms: None,
        last_heartbeat_at_ms: Some(TimestampMs(1_783_000_220_000)),
        error_message: None,
    };
    let lease = AgentResourceLeaseDto {
        id: "lease-1".to_string(),
        resource_kind: "worktree".to_string(),
        resource_id: "slot-1".to_string(),
        holder_id: run.id.clone(),
        status: AgentResourceLeaseStatusDto::Active,
        acquired_at_ms: TimestampMs(1_783_000_210_000),
        expires_at_ms: Some(TimestampMs(1_783_000_300_000)),
        released_at_ms: None,
    };
    assert_eq!(
        serde_json::from_value::<AgentRunDto>(serde_json::to_value(&run).expect("run"))
            .expect("decode run"),
        run
    );
    assert_eq!(
        serde_json::from_value::<AgentResourceLeaseDto>(
            serde_json::to_value(&lease).expect("resource lease")
        )
        .expect("decode resource lease"),
        lease
    );

    let worker = AgentWorkerDto {
        id: "worker-a".to_string(),
        runtime: Some("copaw".to_string()),
        deployment: Some("desktop".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatusDto::Running,
        desired_state: Some("Running".to_string()),
        team_id: Some("alpha-team".to_string()),
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@worker-a:example.test".to_string()),
        room_id: Some("!worker-room:example.test".to_string()),
        container_state: Some("Running".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_000_000_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_000_000_005)),
        last_heartbeat_at: Some("2026-07-02T10:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Skill,
            name: "git-delegation".to_string(),
            metadata: json!({"source": "hiclaw"}),
        }],
        updated_at_ms: TimestampMs(1_783_000_000_010),
    };
    let team = AgentTeamDto {
        id: "alpha-team".to_string(),
        name: "Alpha Team".to_string(),
        status: AgentTeamStatusDto::Active,
        leader_id: "leader-a".to_string(),
        team_room_id: Some("!team-room:example.test".to_string()),
        leader_dm_room_id: Some("!leader-dm:example.test".to_string()),
        leader_ready: true,
        ready_workers: 1,
        total_workers: 2,
        members: vec![AgentTeamMemberDto {
            worker_id: "leader-a".to_string(),
            runtime_name: Some("leader-a".to_string()),
            role: "team_leader".to_string(),
            room_id: Some("!leader-room:example.test".to_string()),
            matrix_user_id: Some("@leader-a:example.test".to_string()),
            ready: true,
        }],
        updated_at_ms: TimestampMs(1_783_000_200_010),
    };
    assert_eq!(
        serde_json::from_value::<AgentWorkerDto>(serde_json::to_value(&worker).expect("worker"))
            .expect("decode worker"),
        worker
    );
    assert_eq!(
        serde_json::from_value::<AgentTeamDto>(serde_json::to_value(&team).expect("team"))
            .expect("decode team"),
        team
    );

    let event = AgentEventDto {
        id: "event-1".to_string(),
        task_id: Some(finite_task.id),
        run_id: Some(run.id),
        kind: AgentEventKindDto::TaskAssigned,
        message: "assigned to worker-a".to_string(),
        payload: json!({"room_id": "!worker-room:example.test"}),
        created_at_ms: TimestampMs(1_783_000_210_000),
    };
    let outbox = AgentOutboxMessageDto {
        id: "outbox-1".to_string(),
        target_kind: "matrix_room".to_string(),
        target_id: "!worker-room:example.test".to_string(),
        recipient_id: Some("@worker-a:example.test".to_string()),
        body: "@worker-a:example.test New task [task-20260702-100001]".to_string(),
        status: AgentOutboxMessageStatusDto::Pending,
        idempotency_key: "task-20260702-100001:assign".to_string(),
        attempts: 0,
        next_attempt_at_ms: Some(TimestampMs(1_783_000_220_000)),
        delivered_at_ms: None,
        last_error: None,
        payload: json!({"event_id": event.id}),
        created_at_ms: TimestampMs(1_783_000_210_010),
    };
    let outbox_json = serde_json::to_value(&outbox).expect("outbox");
    assert_eq!(outbox_json["target_kind"], "matrix_room");
    assert_eq!(outbox_json["idempotency_key"], "task-20260702-100001:assign");
    assert_eq!(
        serde_json::from_value::<AgentEventDto>(serde_json::to_value(&event).expect("event"))
            .expect("decode event"),
        event
    );
    assert_eq!(
        serde_json::from_value::<AgentOutboxMessageDto>(outbox_json).expect("decode outbox"),
        outbox
    );
}

// 看板 fixture 覆盖任务、事件、命令、工作流、日志、产物和旧版导入的原生字段名。
#[test]
fn agent_scheduler_task_board_contract_uses_native_scheduler_names() {
    let workspace = AgentSchedulerWorkspaceContextDto {
        slot_id: "slot-1".to_string(),
        path: "/work/project-one".to_string(),
        branch: "agent/task-a".to_string(),
    };
    let task = AgentSchedulerTaskBoardTaskDto {
        project_id: "project/one".into(),
        task_id: "task/a".into(),
        status_key: "running".to_string(),
        status: Some(AgentTaskStatusDto::Running),
        priority: 1,
        order_no: 0,
        model: "auto".to_string(),
        agent: Some("main".to_string()),
        acp_agent: None,
        reasoning_effort: None,
        prompt: "prompt for task/a".to_string(),
        retry_count: 0,
        run_id: Some("run-1".to_string()),
        run_stage: Some("executing".to_string()),
        workspace: Some(workspace.clone()),
        last_error: None,
        pause_reason: None,
        merge_source_branch: None,
        merge_target_branch: None,
        selected_worktree_path: Some("/work/project-one".to_string()),
        selected_worktree_slot_id: Some("slot-1".to_string()),
        selected_worktree_branch: Some("agent/task-a".to_string()),
        read_model: AgentSchedulerTaskBoardTaskReadModelDto {
            title: Some("Task A".to_string()),
            workflow_id: Some("dev_lifecycle".to_string()),
            status_label: Some("Running".to_string()),
            status_role: Some(TaskPoolWorkflowStatusRoleDto::Executing),
            stage_id: Some("running".to_string()),
            stage_label: Some("Running".to_string()),
            staff: Some(AgentSchedulerTaskBoardStaffAssignmentDto {
                member_id: Some("staff-executor".to_string()),
                member_role: Some("executor".to_string()),
                primary_member_id: Some("staff-executor".to_string()),
                backup_member_ids: vec!["staff-reviewer".to_string()],
                manager_member_ids: vec!["staff-orchestrator".to_string()],
                autonomous: true,
            }),
            stage_action: Some(AgentSchedulerTaskBoardStageActionDto {
                stage_id: "running".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: AgentSchedulerStageActionInputsDto::default(),
            }),
            available_stage_actions: vec![AgentSchedulerTaskBoardStageActionCapabilityDto {
                capability_id: "execute_task".to_string(),
                label: "Execute task".to_string(),
                capability_type: AgentSchedulerStageCapabilityKindDto::BuiltIn,
                built_in: true,
                supported_roles: vec![TaskPoolWorkflowStatusRoleDto::Queued],
                requires_workflow_uuid: false,
                input_summary: AgentSchedulerStageCapabilityIoSummaryDto {
                    description: "Task summary".to_string(),
                    fields: vec!["task_id".to_string()],
                },
                result_summary: AgentSchedulerStageCapabilityIoSummaryDto {
                    description: "Execution result".to_string(),
                    fields: vec!["status".to_string()],
                },
            }],
        },
        updated_at_ms: TimestampMs(12),
    };
    let event = AgentSchedulerTaskBoardEventDto {
        cursor: 2,
        project_id: "project/one".into(),
        task_id: "task/a".into(),
        run_id: Some("run-1".to_string()),
        run_stage: Some("executing".to_string()),
        kind: AgentEventKindDto::TaskStarted,
        from_status_key: Some("queued".to_string()),
        from_status: Some(AgentTaskStatusDto::Queued),
        to_status_key: Some("running".to_string()),
        to_status: Some(AgentTaskStatusDto::Running),
        message: "started".to_string(),
        error_message: None,
        workspace: Some(workspace.clone()),
        payload: json!({"command": "start"}),
        created_at_ms: TimestampMs(13),
    };
    let command_request = AgentSchedulerCommandRequest {
        task_id: "task/a".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(2),
        reason: Some("ready".to_string()),
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    let create_task = AgentTaskDto {
        id: "task/a".to_string(),
        title: "Task A".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: AgentTaskStatusDto::Queued,
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        created_at_ms: TimestampMs(11),
        updated_at_ms: TimestampMs(11),
        metadata: json!({"prompt": "prompt for task/a"}),
    };
    let create_request = AgentSchedulerCreateTaskRequest {
        task_id: "task/a".to_string(),
        title: "Task A".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: Some(AgentTaskStatusDto::Queued),
        spec_uri: "task-board/task/a/spec.md".to_string(),
        meta_uri: "task-board/task/a/meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        metadata: json!({"prompt": "prompt for task/a"}),
        created_at_ms: TimestampMs(11),
    };
    let create_response =
        AgentSchedulerCreateTaskResponse { task_id: "task/a".to_string(), task: create_task };
    let command_response = AgentSchedulerCommandResponse {
        accepted: true,
        task: Some(task.clone()),
        event: Some(event.clone()),
    };
    let status_response = AgentSchedulerTaskBoardStatusResponse {
        project_id: "project/one".into(),
        read_model: AgentSchedulerTaskBoardSnapshotReadModelDto::default(),
        tasks: vec![task],
    };
    let logs_response = AgentSchedulerTaskLogsResponse {
        task_id: "task/a".into(),
        logs: vec![AgentSchedulerTaskLogDto {
            task_id: "task/a".into(),
            seq_no: 1,
            timestamp_ms: TimestampMs(14),
            from_status_key: Some("queued".to_string()),
            status_from: Some(AgentTaskStatusDto::Queued),
            to_status_key: Some("running".to_string()),
            status_to: Some(AgentTaskStatusDto::Running),
            message: "started".to_string(),
        }],
    };
    let artifacts_response = AgentSchedulerTaskArtifactsResponse {
        task_id: "task/a".into(),
        artifacts: vec![AgentSchedulerTaskArtifactDto {
            task_id: "task/a".into(),
            run_id: Some("run-1".to_string()),
            kind: "execution_result".to_string(),
            name: "execution-result.json".to_string(),
            path: Some("shared/tasks/task-a/artifacts/execution-result.json".to_string()),
            payload: json!({"success": true}),
            created_at_ms: TimestampMs(15),
        }],
    };
    let events_response = AgentSchedulerTaskBoardEventsResponse { cursor: 2, events: vec![event] };
    let workflow_response = AgentSchedulerTaskBoardWorkflowResponse {
        project_id: "project/one".into(),
        workflow: Some(TaskPoolWorkflowSnapshotDto {
            id: "dev_lifecycle".to_string(),
            label: "Development".to_string(),
            statuses: vec![TaskPoolWorkflowStatusDto {
                key: "running".to_string(),
                label: "Running".to_string(),
                role: TaskPoolWorkflowStatusRoleDto::Executing,
                enabled: true,
                terminal: false,
                next: Some("code_complete".to_string()),
                transitions: Vec::new(),
            }],
        }),
    };
    let move_command = AgentSchedulerCommandRequest {
        task_id: "task/a".into(),
        command: AgentSchedulerCommandKindDto::Move,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("dragged to review".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            move_task: Some(AgentSchedulerMoveTaskPayloadDto {
                from_status_key: Some("running".to_string()),
                to_status_key: "code_review".to_string(),
                to_status: None,
                order_no: Some(12),
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };
    let metadata_update = AgentSchedulerTaskMetadataUpdateRequest {
        patch: AgentSchedulerTaskMetadataPatchDto {
            status_key: Some("content_writing".to_string()),
            status_role: Some(TaskPoolWorkflowStatusRoleDto::Executing),
            metadata_merge: Some(json!({"task_board.workflow_id": "content_publishing"})),
            ..AgentSchedulerTaskMetadataPatchDto::default()
        },
        reason: Some("sync custom workflow status".to_string()),
    };
    let metadata_update_response = AgentSchedulerTaskMetadataUpdateResponse {
        task: status_response.tasks[0].clone(),
        event: None,
    };
    let legacy_import = AgentSchedulerLegacyImportRequest {
        workflow: workflow_response.workflow.clone(),
        tasks: vec![AgentSchedulerLegacyTaskImportDto {
            task_id: "legacy/custom".into(),
            title: "Legacy custom task".to_string(),
            status_key: Some("content_writing".to_string()),
            status: None,
            priority: 9,
            order_no: 3,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "migrate this task".to_string(),
            workflow_id: Some("content_publishing".to_string()),
            metadata: json!({"legacy_path": "tasks.json"}),
            created_at_ms: TimestampMs(19),
            updated_at_ms: Some(TimestampMs(20)),
        }],
        dry_run: true,
    };
    let legacy_import_response = AgentSchedulerLegacyImportResponse {
        imported: 0,
        skipped: 1,
        tasks: Vec::new(),
        events: Vec::new(),
    };
    let bulk_request = AgentSchedulerBulkCommandRequest {
        commands: vec![move_command.clone()],
        stop_on_error: true,
    };
    let bulk_response = AgentSchedulerBulkCommandResponse {
        accepted: 1,
        rejected: 0,
        results: vec![AgentSchedulerCommandResponse {
            accepted: true,
            task: Some(metadata_update_response.task.clone()),
            event: None,
        }],
    };
    let staff_payload = AgentSchedulerCommandPayloadDto {
        staff_command: Some(AgentSchedulerStaffCommandPayloadDto {
            command_id: "staff-start-1".to_string(),
            member_id: "staff-executor".to_string(),
            stage_id: Some("running".to_string()),
            role: Some("executor".to_string()),
            action: Some("start_task".to_string()),
            metadata: Some(json!({"source": "staff_mode"})),
        }),
        stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
            stage_id: "code_review".to_string(),
            capability_id: "code_review".to_string(),
            workflow_uuid: None,
            inputs: AgentSchedulerStageActionInputsDto::default(),
            outcome: None,
            member_id: Some("staff-reviewer".to_string()),
            metadata: None,
        }),
        ..AgentSchedulerCommandPayloadDto::default()
    };
    let wake_request = AgentSchedulerWakeRequest {
        now_ms: TimestampMs(16),
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(2),
        auto_promote_pool_tasks: true,
    };
    let wake_response = AgentSchedulerWakeResponse {
        dispatched: true,
        task_id: Some("task/a".to_string()),
        run_id: Some("run-1".to_string()),
        worker_id: Some("worker-a".to_string()),
        outbox_message_id: Some("outbox-1".to_string()),
    };
    let drain_request =
        AgentSchedulerOutboxDrainRequest { now_ms: TimestampMs(17), limit: Some(10) };
    let drain_response = AgentSchedulerOutboxDrainResponse {
        claimed: 1,
        messages: vec![AgentOutboxMessageDto {
            id: "outbox-1".to_string(),
            target_kind: "matrix_room".to_string(),
            target_id: "!room:example.test".to_string(),
            recipient_id: None,
            body: "task/a ready".to_string(),
            status: AgentOutboxMessageStatusDto::Pending,
            idempotency_key: "task/a:dispatch".to_string(),
            attempts: 0,
            next_attempt_at_ms: Some(TimestampMs(18)),
            delivered_at_ms: None,
            last_error: None,
            payload: json!({"task_id": "task/a"}),
            created_at_ms: TimestampMs(18),
        }],
    };

    let create_json = serde_json::to_value(&create_request).expect("create request");
    assert!(create_json.get("project_path").is_none());
    assert_eq!(create_json["task_id"], "task/a");
    assert_eq!(create_json["kind"], "finite");
    assert_eq!(create_json["status"], "queued");
    assert_eq!(create_json["metadata"]["prompt"], "prompt for task/a");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerCreateTaskResponse>(
            serde_json::to_value(create_response).expect("create response")
        )
        .expect("decode create response")
        .task
        .status,
        AgentTaskStatusDto::Queued
    );
    let command_json = serde_json::to_value(&command_request).expect("command request");
    assert_eq!(command_json["command"], "start");
    assert_eq!(command_json["task_id"], "task/a");
    let status_json = serde_json::to_value(&status_response).expect("status");
    assert_eq!(status_json["tasks"][0]["status"], "running");
    assert_eq!(status_json["tasks"][0]["workspace"]["slot_id"], "slot-1");
    assert_eq!(status_json["tasks"][0]["read_model"]["workflow_id"], "dev_lifecycle");
    assert_eq!(status_json["tasks"][0]["read_model"]["status_role"], "executing");
    assert_eq!(
        status_json["tasks"][0]["read_model"]["stage_action"]["capability_id"],
        "execute_task"
    );
    let events_json = serde_json::to_value(&events_response).expect("events");
    assert_eq!(events_json["events"][0]["kind"], "task_started");
    assert_eq!(events_json["events"][0]["workspace"]["branch"], "agent/task-a");
    let workflow_json = serde_json::to_value(&workflow_response).expect("workflow response");
    assert_eq!(workflow_json["workflow"]["statuses"][0]["role"], "executing");
    let move_json = serde_json::to_value(&move_command).expect("move command");
    assert_eq!(move_json["command"], "move");
    assert_eq!(move_json["payload"]["move_task"]["to_status_key"], "code_review");
    let metadata_update_json =
        serde_json::to_value(&metadata_update).expect("metadata update request");
    assert_eq!(metadata_update_json["patch"]["status_key"], "content_writing");
    assert_eq!(metadata_update_json["patch"]["status_role"], "executing");
    let legacy_json = serde_json::to_value(&legacy_import).expect("legacy import");
    assert_eq!(legacy_json["tasks"][0]["status_key"], "content_writing");
    assert_eq!(legacy_json["dry_run"], true);
    assert_eq!(
        serde_json::from_value::<AgentSchedulerLegacyImportResponse>(
            serde_json::to_value(legacy_import_response).expect("legacy response")
        )
        .expect("decode legacy response")
        .skipped,
        1
    );
    let bulk_json = serde_json::to_value(&bulk_request).expect("bulk request");
    assert_eq!(bulk_json["commands"][0]["payload"]["move_task"]["order_no"], 12);
    assert_eq!(
        serde_json::from_value::<AgentSchedulerBulkCommandResponse>(
            serde_json::to_value(bulk_response).expect("bulk response")
        )
        .expect("decode bulk response")
        .accepted,
        1
    );
    let staff_json = serde_json::to_value(staff_payload).expect("staff payload");
    assert_eq!(staff_json["staff_command"]["member_id"], "staff-executor");
    assert_eq!(staff_json["stage_action"]["capability_id"], "code_review");
    let wake_json = serde_json::to_value(&wake_request).expect("wake request");
    assert_eq!(wake_json["auto_promote_pool_tasks"], true);
    assert!(
        serde_json::from_value::<AgentSchedulerWakeRequest>(wake_json)
            .expect("decode wake request")
            .auto_promote_pool_tasks
    );
    assert!(
        serde_json::from_value::<AgentSchedulerCommandResponse>(
            serde_json::to_value(command_response).expect("command response")
        )
        .expect("decode command response")
        .accepted
    );
    assert_eq!(
        serde_json::from_value::<AgentSchedulerTaskLogsResponse>(
            serde_json::to_value(logs_response).expect("logs")
        )
        .expect("decode logs")
        .logs[0]
            .message,
        "started"
    );
    assert_eq!(
        serde_json::from_value::<AgentSchedulerTaskArtifactsResponse>(
            serde_json::to_value(artifacts_response).expect("artifacts")
        )
        .expect("decode artifacts")
        .artifacts[0]
            .payload,
        json!({"success": true})
    );
    assert_eq!(serde_json::to_value(wake_request).expect("wake request")["now_ms"], 16);
    assert_eq!(
        serde_json::from_value::<AgentSchedulerWakeResponse>(
            serde_json::to_value(wake_response).expect("wake response")
        )
        .expect("decode wake")
        .worker_id
        .as_deref(),
        Some("worker-a")
    );
    assert_eq!(serde_json::to_value(drain_request).expect("drain request")["limit"], 10);
    assert_eq!(
        serde_json::from_value::<AgentSchedulerOutboxDrainResponse>(
            serde_json::to_value(drain_response).expect("drain response")
        )
        .expect("decode drain")
        .claimed,
        1
    );
}

// 阶段动作输入允许结构化参数，并继续接受旧载荷中的顶层工作流标识。
#[test]
fn staff_stage_action_inputs_and_outcome_roundtrip() {
    let inputs = AgentSchedulerStageActionInputsDto {
        workflow_uuid: Some("workflow-review".to_string()),
        parameters: BTreeMap::from([("focus".to_string(), json!("security"))]),
    };
    let command_payload = AgentSchedulerCommandPayloadDto {
        stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
            stage_id: "code_review".to_string(),
            capability_id: "product_review".to_string(),
            workflow_uuid: None,
            inputs: inputs.clone(),
            outcome: Some(AgentSchedulerStageActionOutcomeDto::Retry),
            member_id: Some("staff-reviewer".to_string()),
            metadata: Some(json!({"source": "staff"})),
        }),
        ..AgentSchedulerCommandPayloadDto::default()
    };

    let value = serde_json::to_value(&command_payload).expect("serialize command payload");
    assert_eq!(value["stage_action"]["inputs"]["workflow_uuid"], "workflow-review");
    assert_eq!(value["stage_action"]["inputs"]["parameters"]["focus"], "security");
    assert_eq!(value["stage_action"]["outcome"], "retry");
    assert!(value["stage_action"].get("workflow_uuid").is_none());
    assert_eq!(
        serde_json::from_value::<AgentSchedulerCommandPayloadDto>(value)
            .expect("decode command payload"),
        command_payload
    );

    let read_model_action = AgentSchedulerTaskBoardStageActionDto {
        stage_id: "code_review".to_string(),
        capability_id: "product_review".to_string(),
        workflow_uuid: None,
        inputs,
    };
    let read_model_json = serde_json::to_value(&read_model_action).expect("serialize read model");
    assert_eq!(read_model_json["inputs"]["workflow_uuid"], "workflow-review");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerTaskBoardStageActionDto>(read_model_json)
            .expect("decode read model"),
        read_model_action
    );

    let legacy_payload = json!({
        "stage_id": "code_review",
        "capability_id": "saved_workflow",
        "workflow_uuid": "legacy-workflow"
    });
    let legacy_action: AgentSchedulerTaskBoardStageActionDto =
        serde_json::from_value(legacy_payload).expect("legacy action decodes");
    assert_eq!(legacy_action.workflow_uuid.as_deref(), Some("legacy-workflow"));
    assert_eq!(legacy_action.inputs, AgentSchedulerStageActionInputsDto::default());
}

// 管理接口 fixture 保证完整 worker 与 team 快照在增改查响应间无字段丢失。
#[test]
fn agent_scheduler_management_dto_round_trips_complete_worker_and_team_snapshots() {
    let worker = AgentWorkerDto {
        id: "worker-management".to_string(),
        runtime: Some("codex".to_string()),
        deployment: Some("desktop".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatusDto::Idle,
        desired_state: Some("running".to_string()),
        team_id: Some("team-management".to_string()),
        worker_role: Some("worker".to_string()),
        matrix_user_id: None,
        room_id: None,
        container_state: Some("running".to_string()),
        idle_since_ms: Some(TimestampMs(1_783_001_000_000)),
        last_seen_at_ms: Some(TimestampMs(1_783_001_000_100)),
        last_heartbeat_at: Some("2026-07-13T09:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Skill,
            name: "gateway-management".to_string(),
            metadata: json!({"level": "project"}),
        }],
        updated_at_ms: TimestampMs(1_783_001_000_200),
    };
    let worker_json = serde_json::to_value(&worker).expect("serialize worker management dto");
    assert_eq!(worker_json["deployment"], "desktop");
    assert_eq!(worker_json["last_seen_at_ms"], 1_783_001_000_100_i64);
    assert_eq!(worker_json["updated_at_ms"], 1_783_001_000_200_i64);
    assert_eq!(
        serde_json::from_value::<AgentWorkerDto>(worker_json)
            .expect("decode worker management dto"),
        worker
    );

    let member = AgentTeamMemberDto {
        worker_id: "leader-management".to_string(),
        runtime_name: Some("leader-runtime".to_string()),
        role: "team_leader".to_string(),
        room_id: Some("!leader:example.test".to_string()),
        matrix_user_id: Some("@leader:example.test".to_string()),
        ready: true,
    };
    let request = AgentSchedulerUpsertTeamRequest {
        team_id: "team-management".to_string(),
        name: "Management Team".to_string(),
        status: Some(AgentTeamStatusDto::Active),
        leader_id: "leader-management".to_string(),
        team_room_id: Some("!team:example.test".to_string()),
        leader_dm_room_id: Some("!leader-dm:example.test".to_string()),
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: vec![member.clone()],
        updated_at_ms: Some(TimestampMs(1_783_001_100_000)),
    };
    let request_json = serde_json::to_value(&request).expect("serialize team upsert request");
    assert_eq!(request_json["team_id"], "team-management");
    assert_eq!(request_json["members"][0]["worker_id"], "leader-management");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerUpsertTeamRequest>(request_json)
            .expect("decode team upsert request"),
        request
    );

    let team = AgentTeamDto {
        id: request.team_id.clone(),
        name: request.name.clone(),
        status: AgentTeamStatusDto::Active,
        leader_id: request.leader_id.clone(),
        team_room_id: request.team_room_id.clone(),
        leader_dm_room_id: request.leader_dm_room_id.clone(),
        leader_ready: request.leader_ready,
        ready_workers: request.ready_workers,
        total_workers: request.total_workers,
        members: request.members.clone(),
        updated_at_ms: request.updated_at_ms.expect("team timestamp"),
    };
    let response = AgentSchedulerUpsertTeamResponse { team: team.clone() };
    let list = AgentSchedulerTeamsResponse { teams: vec![team] };
    let response_json = serde_json::to_value(&response).expect("serialize team upsert response");
    let list_json = serde_json::to_value(&list).expect("serialize team list response");
    assert_eq!(response_json["team"]["updated_at_ms"], 1_783_001_100_000_i64);
    assert_eq!(list_json["teams"][0]["name"], "Management Team");
    assert_eq!(
        serde_json::from_value::<AgentSchedulerUpsertTeamResponse>(response_json)
            .expect("decode team upsert response"),
        response
    );
    assert_eq!(
        serde_json::from_value::<AgentSchedulerTeamsResponse>(list_json)
            .expect("decode team list response"),
        list
    );
}
