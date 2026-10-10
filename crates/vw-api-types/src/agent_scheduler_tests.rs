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
    AgentSchedulerTaskArtifactDto, AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventDto, AgentSchedulerTaskBoardEventsResponse,
    AgentSchedulerTaskBoardSnapshotReadModelDto,
    AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto,
    AgentSchedulerTaskBoardTaskReadModelDto, AgentSchedulerTaskBoardWorkflowResponse,
    AgentSchedulerTaskGraphEdgeDto, AgentSchedulerTaskGraphReadinessDto,
    AgentSchedulerTaskGraphResponse, AgentSchedulerTaskGraphTaskDto, AgentSchedulerTaskLogDto,
    AgentSchedulerTaskLogsResponse, AgentSchedulerTaskMetadataPatchDto,
    AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerTaskMetadataUpdateResponse,
    AgentSchedulerWakeRequest, AgentSchedulerWakeResponse, AgentSchedulerWorkspaceContextDto,
    AgentTaskDto, AgentTaskKindDto, AgentTaskStatusDto, TaskDependencyNodeDto,
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
