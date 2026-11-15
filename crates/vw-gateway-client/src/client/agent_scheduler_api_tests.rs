#![allow(unused_imports, dead_code)]

//! 验证代理调度客户端的任务图、任务看板、工作者与团队接口契约。
//!
//! 测试同时覆盖项目作用域编码、租户隔离校验以及事件游标的可选查询语义。

use serde_json::json;
use vw_api_agent_scheduler::AgentSchedulerScopeDto;
use vw_api_agent_scheduler::{
    AgentEventKindDto, AgentSchedulerBulkCommandRequest, AgentSchedulerCommandKindDto,
    AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest, AgentSchedulerCreateTaskRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyTaskImportDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerOutboxDrainRequest, AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskGraphEdgeDto,
    AgentSchedulerTaskMetadataPatchDto, AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerWakeRequest, AgentTaskKindDto, AgentTaskStatusDto,
};
use vw_api_transport::common::TimestampMs;
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};
use vw_api_project::ListProjectsRequest;
use vw_api_project::TenantScopeDto;
use vw_api_task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
};

use crate::client::test_support;
use crate::endpoint::GatewayEndpoint;

#[tokio::test]
async fn gateway_client_encodes_project_and_scheduler_scopes() {
    let tenant: TenantScopeDto =
        TenantScope::new("platform/one", "company one").expect("tenant scope");
    let server = test_support::server_with_tenant_scope(
        vec![(200, json!({"items": []})), (200, json!({"project_id": "project?one", "tasks": []}))],
        tenant.clone(),
    );
    let scheduler = AgentSchedulerScopeDto::new(
        ProjectScope::new(tenant.clone(), ProjectId::from("project?one")).expect("project scope"),
    );

    server
        .client()
        .project_list(&tenant, &ListProjectsRequest::default())
        .await
        .expect("project list");
    server.client().agent_scheduler_status(&scheduler).await.expect("scheduler status");

    let project = server.take_request();
    let scheduler = server.take_request();
    assert_eq!(project.path, "/v1/projects");
    assert_eq!(scheduler.path, "/v1/agent-scheduler/projects/project%3Fone/tasks");
    server.join();
}
