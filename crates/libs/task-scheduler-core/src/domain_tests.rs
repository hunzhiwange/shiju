//! 测试调度领域标识、状态值和记录类型的校验与序列化契约。
//! 往返序列化覆盖合法值和拒绝路径，防止存储字符串与领域枚举悄然漂移。

use vw_api_agent_scheduler::{
    AgentResourceLeaseDto, AgentResourceLeaseStatusDto,
    AgentSchedulerScope, AgentTaskKindDto,
};
use vw_api_transport::common::TimestampMs;
use vw_api_transport::id::ProjectId;

use super::domain::{
    AgentResourceLease, AgentResourceLeaseStatus,
    AgentSchedulerContext, AgentTaskKind,
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
    type MutateLease = fn(&mut AgentResourceLeaseDto);
    let cases: [(MutateLease, &str); 4] = [
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
