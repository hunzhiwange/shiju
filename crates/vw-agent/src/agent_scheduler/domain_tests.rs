use serde_json::json;
use vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentResourceLeaseDto, AgentResourceLeaseStatusDto,
    AgentRunDto, AgentRunStatusDto, AgentSchedulerScope, AgentTaskDto, AgentTaskKindDto,
    AgentTaskStatusDto, AgentTeamDto, AgentTeamMemberDto, AgentTeamStatusDto, AgentWorkerDto,
    AgentWorkerStatusDto, TeamTaskDagNodeDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::id::ProjectId;

use super::domain::{
    AgentCapabilityKind, AgentResourceLease, AgentResourceLeaseStatus, AgentRun, AgentRunStatus,
    AgentSchedulerContext, AgentTask, AgentTaskKind, AgentTaskStatus, AgentTeam, AgentTeamStatus,
    AgentWorker, AgentWorkerStatus, TeamId, WorkerName,
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
fn agent_domain_converts_from_api_dto_without_losing_scheduler_fields() {
    let task = task_dto();
    let run = run_dto(&task.id);
    let worker = worker_dto();
    let team = team_dto();
    let resource_lease = resource_lease_dto(&run.id);

    let domain_task = AgentTask::try_from(task).expect("task converts");
    assert_eq!(domain_task.id.as_str(), "task-20260702-100003");
    assert_eq!(domain_task.kind, AgentTaskKind::TeamDag);
    assert_eq!(domain_task.status, AgentTaskStatus::Assigned);
    assert_eq!(domain_task.assigned_to.as_ref().map(WorkerName::as_str), Some("leader-a"));
    assert_eq!(domain_task.team_id.as_ref().map(TeamId::as_str), Some("alpha-team"));
    assert_eq!(domain_task.delegated_to_team.as_ref().map(TeamId::as_str), Some("alpha-team"));
    assert_eq!(domain_task.dag[1].depends_on[0].as_str(), "team-task-1");
    assert_eq!(domain_task.dag[1].assigned_to.as_ref().map(WorkerName::as_str), Some("worker-ui"));

    let domain_run = AgentRun::try_from(run).expect("run converts");
    assert_eq!(domain_run.id.as_str(), "run-1");
    assert_eq!(domain_run.task_id.as_str(), "task-20260702-100003");
    assert_eq!(domain_run.worker.as_ref().map(WorkerName::as_str), Some("leader-a"));
    assert_eq!(domain_run.lease_expires_at_ms, Some(TimestampMs(1_783_000_300_000)));
    assert_eq!(domain_run.attempt, 2);

    let domain_worker = AgentWorker::try_from(worker).expect("worker converts");
    assert_eq!(domain_worker.id.as_str(), "leader-a");
    assert_eq!(domain_worker.team_id.as_ref().map(TeamId::as_str), Some("alpha-team"));
    assert_eq!(domain_worker.capabilities[0].name, "git-delegation");

    let domain_team = AgentTeam::try_from(team).expect("team converts");
    assert_eq!(domain_team.id.as_str(), "alpha-team");
    assert_eq!(domain_team.leader_id.as_str(), "leader-a");
    assert_eq!(domain_team.members[0].worker_id.as_str(), "leader-a");

    let domain_resource_lease =
        AgentResourceLease::try_from(resource_lease).expect("resource lease converts");
    assert_eq!(domain_resource_lease.id.as_str(), "lease-1");
    assert_eq!(domain_resource_lease.holder_id.as_str(), "run-1");
    assert_eq!(domain_resource_lease.expires_at_ms, Some(TimestampMs(1_783_000_300_000)));
}

#[test]
fn agent_domain_rejects_empty_ids() {
    let mut task = task_dto();
    task.id = " ".to_string();
    let error = AgentTask::try_from(task).expect_err("empty task id rejected");
    assert!(error.to_string().contains("task id must not be empty"));

    let mut team = team_dto();
    team.id.clear();
    let error = AgentTeam::try_from(team).expect_err("empty team id rejected");
    assert!(error.to_string().contains("team id must not be empty"));
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
        ("team_dag", AgentTaskKindDto::TeamDag, AgentTaskKind::TeamDag),
    ];

    for (value, dto, expected) in cases {
        assert_eq!(AgentTaskKind::parse(value).expect("known task kind"), expected);
        assert_eq!(AgentTaskKind::from(dto), expected);
    }
}

#[test]
fn agent_domain_rejects_missing_worker_name_for_leased_run() {
    let mut run = run_dto("task-20260702-100003");
    run.worker_id = None;

    let error = AgentRun::try_from(run).expect_err("leased run requires worker");
    assert!(error.to_string().contains("leased run requires worker name"));

    let mut worker = worker_dto();
    worker.id.clear();
    let error = AgentWorker::try_from(worker).expect_err("empty worker name rejected");
    assert!(error.to_string().contains("worker name must not be empty"));
}

#[test]
fn agent_domain_rejects_missing_worker_name_for_running_run() {
    let mut run = run_dto("task-20260702-100003");
    run.status = AgentRunStatusDto::Running;
    run.worker_id = None;

    let error = AgentRun::try_from(run).expect_err("running run requires worker");
    assert!(error.to_string().contains("running run requires worker name"));
}

#[test]
fn agent_domain_parses_capability_worker_and_team_statuses() {
    let capability_cases = [
        ("skill", AgentCapabilityKind::Skill),
        ("remote_skill", AgentCapabilityKind::RemoteSkill),
        ("mcp_server", AgentCapabilityKind::McpServer),
        ("model", AgentCapabilityKind::Model),
        ("runtime", AgentCapabilityKind::Runtime),
        ("tool", AgentCapabilityKind::Tool),
    ];
    for (value, expected) in capability_cases {
        let parsed = AgentCapabilityKind::parse(value).expect("known capability kind");
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), value);
    }

    let worker_cases = [
        ("pending", AgentWorkerStatus::Pending),
        ("idle", AgentWorkerStatus::Idle),
        ("busy", AgentWorkerStatus::Busy),
        ("running", AgentWorkerStatus::Running),
        ("sleeping", AgentWorkerStatus::Sleeping),
        ("stopped", AgentWorkerStatus::Stopped),
        ("failed", AgentWorkerStatus::Failed),
        ("unavailable", AgentWorkerStatus::Unavailable),
    ];
    for (value, expected) in worker_cases {
        let parsed = AgentWorkerStatus::parse(value).expect("known worker status");
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), value);
    }

    let team_cases = [
        ("pending", AgentTeamStatus::Pending),
        ("active", AgentTeamStatus::Active),
        ("degraded", AgentTeamStatus::Degraded),
        ("failed", AgentTeamStatus::Failed),
    ];
    for (value, expected) in team_cases {
        let parsed = AgentTeamStatus::parse(value).expect("known team status");
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), value);
    }
}

#[test]
fn agent_domain_rejects_unknown_capability_worker_and_team_statuses() {
    let capability_error =
        AgentCapabilityKind::parse("database").expect_err("unknown capability kind rejected");
    assert!(capability_error.to_string().contains("unsupported agent capability kind: database"));

    let worker_error =
        AgentWorkerStatus::parse("offline").expect_err("unknown worker status rejected");
    assert!(worker_error.to_string().contains("unsupported agent worker status: offline"));

    let team_error = AgentTeamStatus::parse("archived").expect_err("unknown team status rejected");
    assert!(team_error.to_string().contains("unsupported agent team status: archived"));
}

#[test]
fn agent_domain_converts_all_api_status_variants() {
    let task_status_cases = [
        (AgentTaskStatusDto::Draft, AgentTaskStatus::Draft),
        (AgentTaskStatusDto::Queued, AgentTaskStatus::Queued),
        (AgentTaskStatusDto::Assigned, AgentTaskStatus::Assigned),
        (AgentTaskStatusDto::Dispatched, AgentTaskStatus::Dispatched),
        (AgentTaskStatusDto::Acknowledged, AgentTaskStatus::Acknowledged),
        (AgentTaskStatusDto::Active, AgentTaskStatus::Active),
        (AgentTaskStatusDto::Running, AgentTaskStatus::Running),
        (AgentTaskStatusDto::Submitted, AgentTaskStatus::Submitted),
        (AgentTaskStatusDto::Revision, AgentTaskStatus::Revision),
        (AgentTaskStatusDto::Blocked, AgentTaskStatus::Blocked),
        (AgentTaskStatusDto::Paused, AgentTaskStatus::Paused),
        (AgentTaskStatusDto::Completed, AgentTaskStatus::Completed),
        (AgentTaskStatusDto::Failed, AgentTaskStatus::Failed),
        (AgentTaskStatusDto::Cancelled, AgentTaskStatus::Cancelled),
        (AgentTaskStatusDto::Archived, AgentTaskStatus::Archived),
    ];
    for (dto, expected) in task_status_cases {
        assert_eq!(AgentTaskStatus::from(dto), expected);
    }

    let run_status_cases = [
        (AgentRunStatusDto::Pending, AgentRunStatus::Pending),
        (AgentRunStatusDto::Leased, AgentRunStatus::Leased),
        (AgentRunStatusDto::Running, AgentRunStatus::Running),
        (AgentRunStatusDto::Succeeded, AgentRunStatus::Succeeded),
        (AgentRunStatusDto::Failed, AgentRunStatus::Failed),
        (AgentRunStatusDto::Cancelled, AgentRunStatus::Cancelled),
        (AgentRunStatusDto::TimedOut, AgentRunStatus::TimedOut),
        (AgentRunStatusDto::Stale, AgentRunStatus::Stale),
    ];
    for (dto, expected) in run_status_cases {
        assert_eq!(AgentRunStatus::from(dto), expected);
    }

    let capability_kind_cases = [
        (AgentCapabilityKindDto::Skill, AgentCapabilityKind::Skill),
        (AgentCapabilityKindDto::RemoteSkill, AgentCapabilityKind::RemoteSkill),
        (AgentCapabilityKindDto::McpServer, AgentCapabilityKind::McpServer),
        (AgentCapabilityKindDto::Model, AgentCapabilityKind::Model),
        (AgentCapabilityKindDto::Runtime, AgentCapabilityKind::Runtime),
        (AgentCapabilityKindDto::Tool, AgentCapabilityKind::Tool),
    ];
    for (dto, expected) in capability_kind_cases {
        assert_eq!(AgentCapabilityKind::from(dto), expected);
    }

    let worker_status_cases = [
        (AgentWorkerStatusDto::Pending, AgentWorkerStatus::Pending),
        (AgentWorkerStatusDto::Idle, AgentWorkerStatus::Idle),
        (AgentWorkerStatusDto::Busy, AgentWorkerStatus::Busy),
        (AgentWorkerStatusDto::Running, AgentWorkerStatus::Running),
        (AgentWorkerStatusDto::Sleeping, AgentWorkerStatus::Sleeping),
        (AgentWorkerStatusDto::Stopped, AgentWorkerStatus::Stopped),
        (AgentWorkerStatusDto::Failed, AgentWorkerStatus::Failed),
        (AgentWorkerStatusDto::Unavailable, AgentWorkerStatus::Unavailable),
    ];
    for (dto, expected) in worker_status_cases {
        assert_eq!(AgentWorkerStatus::from(dto), expected);
    }

    let team_status_cases = [
        (AgentTeamStatusDto::Pending, AgentTeamStatus::Pending),
        (AgentTeamStatusDto::Active, AgentTeamStatus::Active),
        (AgentTeamStatusDto::Degraded, AgentTeamStatus::Degraded),
        (AgentTeamStatusDto::Failed, AgentTeamStatus::Failed),
    ];
    for (dto, expected) in team_status_cases {
        assert_eq!(AgentTeamStatus::from(dto), expected);
    }

    let resource_lease_status_cases = [
        (AgentResourceLeaseStatusDto::Active, AgentResourceLeaseStatus::Active),
        (AgentResourceLeaseStatusDto::Released, AgentResourceLeaseStatus::Released),
        (AgentResourceLeaseStatusDto::Expired, AgentResourceLeaseStatus::Expired),
        (AgentResourceLeaseStatusDto::Revoked, AgentResourceLeaseStatus::Revoked),
    ];
    for (dto, expected) in resource_lease_status_cases {
        assert_eq!(AgentResourceLeaseStatus::from(dto), expected);
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
        kind: AgentTaskKindDto::TeamDag,
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
    }
}

fn run_dto(task_id: &str) -> AgentRunDto {
    AgentRunDto {
        id: "run-1".to_string(),
        task_id: task_id.to_string(),
        worker_id: Some("leader-a".to_string()),
        status: AgentRunStatusDto::Leased,
        lease_expires_at_ms: Some(TimestampMs(1_783_000_300_000)),
        attempt: 2,
        started_at_ms: Some(TimestampMs(1_783_000_210_000)),
        finished_at_ms: None,
        last_heartbeat_at_ms: Some(TimestampMs(1_783_000_220_000)),
        error_message: None,
    }
}

fn worker_dto() -> AgentWorkerDto {
    AgentWorkerDto {
        id: "leader-a".to_string(),
        runtime: Some("copaw".to_string()),
        deployment: Some("desktop".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatusDto::Running,
        desired_state: Some("Running".to_string()),
        team_id: Some("alpha-team".to_string()),
        worker_role: Some("team_leader".to_string()),
        matrix_user_id: Some("@leader-a:example.test".to_string()),
        room_id: Some("!leader-room:example.test".to_string()),
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
    }
}

fn team_dto() -> AgentTeamDto {
    AgentTeamDto {
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
