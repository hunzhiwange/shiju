//! 测试调度领域标识、状态值和记录类型的校验与序列化契约。
//! 往返序列化覆盖合法值和拒绝路径，防止存储字符串与领域枚举悄然漂移。

use serde_json::json;
use vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentResourceLeaseDto, AgentResourceLeaseStatusDto,
    AgentRunDto, AgentRunStatusDto, AgentSchedulerScope, AgentTaskDto, AgentTaskKindDto,
    AgentTaskStatusDto, TaskDependencyNodeDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::id::ProjectId;

use super::domain::{
    AgentCapabilityKind, AgentResourceLease, AgentResourceLeaseStatus, AgentRun, AgentRunStatus,
    AgentSchedulerContext, AgentTask, AgentTaskKind, AgentTaskStatus, TeamId, AssigneeId,
};

#[test]
fn agent_scheduler_context_requires_workspace_path() {
    let scope = AgentSchedulerScope::for_default_tenant(ProjectId::from("project-a"))
        .expect("valid scheduler scope");
    assert!(AgentSchedulerContext::new(scope.clone(), std::path::PathBuf::new()).is_err());

    let context = AgentSchedulerContext::new(scope.clone(), "/work/project-a")
        .expect("valid scheduler context");
    assert_eq!(context.scope(), &scope);
    assert_eq!(context.project_path(), std::path::Path::new("/work/project-a"));
    assert_eq!(&*context, std::path::Path::new("/work/project-a"));
}
#[test]
fn agent_domain_rejects_unknown_task_kind() {
    let error = AgentTaskKind::parse("long_running").expect_err("unknown kind rejected");
    assert!(error.to_string().contains("unsupported agent task kind: long_running"));
}

#[test]
fn agent_domain_parses_known_task_kinds() {
    let cases = [
        ("finite", AgentTaskKindDto::Finite, AgentTaskKind::Finite),
        ("infinite", AgentTaskKindDto::Infinite, AgentTaskKind::Infinite),
        ("dependency_graph", AgentTaskKindDto::DependencyGraph, AgentTaskKind::DependencyGraph),
    ];

    for (value, dto, expected) in cases {
        assert_eq!(AgentTaskKind::parse(value).expect("known task kind"), expected);
        assert_eq!(AgentTaskKind::from(dto), expected);
    }
}

#[test]
fn agent_resource_lease_preserves_resource_identity_and_release_state() {
    let mut resource_lease = resource_lease_dto("run-1");
    resource_lease.status = AgentResourceLeaseStatusDto::Released;
    resource_lease.released_at_ms = Some(TimestampMs(1_783_000_310_000));

    let domain_resource_lease =
        AgentResourceLease::try_from(resource_lease).expect("resource lease converts");

    assert_eq!(domain_resource_lease.resource_kind.as_str(), "worktree");
    assert_eq!(domain_resource_lease.resource_id.as_str(), "slot-1");
    assert_eq!(domain_resource_lease.status, AgentResourceLeaseStatus::Released);
    assert_eq!(domain_resource_lease.released_at_ms, Some(TimestampMs(1_783_000_310_000)));
}

#[test]
fn agent_domain_rejects_empty_resource_lease_identifiers() {
    let cases: [(fn(&mut AgentResourceLeaseDto), &str); 4] = [
        (|resource_lease| resource_lease.id.clear(), "resource lease id must not be empty"),
        (|resource_lease| resource_lease.resource_kind.clear(), "resource kind must not be empty"),
        (|resource_lease| resource_lease.resource_id.clear(), "resource id must not be empty"),
        (|resource_lease| resource_lease.holder_id.clear(), "holder id must not be empty"),
    ];

    for (mutate, expected_message) in cases {
        let mut resource_lease = resource_lease_dto("run-1");
        mutate(&mut resource_lease);

        let error =
            AgentResourceLease::try_from(resource_lease).expect_err("empty lease field rejected");
        assert!(error.to_string().contains(expected_message));
    }
}

fn task_dto() -> AgentTaskDto {
    AgentTaskDto {
        id: "task-20260702-100003".to_string(),
        title: "Ship team project".to_string(),
        kind: AgentTaskKindDto::DependencyGraph,
        status: AgentTaskStatusDto::Assigned,
        assigned_to: Some("leader-a".to_string()),
        team_id: Some("alpha-team".to_string()),
        room_id: Some("!leader-room:example.test".to_string()),
        project_room_id: Some("!project-room:example.test".to_string()),
        delegated_to_team: Some("alpha-team".to_string()),
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: vec![
            TaskDependencyNodeDto {
                task_id: "team-task-1".to_string(),
                title: "Build API".to_string(),
                depends_on: Vec::new(),
                assigned_to: Some("worker-api".to_string()),
                status: AgentTaskStatusDto::Running,
            },
            TaskDependencyNodeDto {
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
    }
}

fn run_dto(task_id: &str) -> AgentRunDto {
    AgentRunDto {
        id: "run-1".to_string(),
        task_id: task_id.to_string(),
        status: AgentRunStatusDto::Leased,
        lease_expires_at_ms: Some(TimestampMs(1_783_000_300_000)),
        attempt: 2,
        started_at_ms: Some(TimestampMs(1_783_000_210_000)),
        finished_at_ms: None,
        last_heartbeat_at_ms: Some(TimestampMs(1_783_000_220_000)),
        error_message: None,
    }
}



fn resource_lease_dto(holder_id: &str) -> AgentResourceLeaseDto {
    AgentResourceLeaseDto {
        id: "lease-1".to_string(),
        resource_kind: "worktree".to_string(),
        resource_id: "slot-1".to_string(),
        holder_id: holder_id.to_string(),
        status: AgentResourceLeaseStatusDto::Active,
        acquired_at_ms: TimestampMs(1_783_000_210_000),
        expires_at_ms: Some(TimestampMs(1_783_000_300_000)),
        released_at_ms: None,
    }
}
