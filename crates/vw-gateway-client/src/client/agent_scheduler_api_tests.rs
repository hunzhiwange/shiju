#![allow(unused_imports, dead_code)]

//! 验证代理调度客户端的任务图、任务看板、工作者与团队接口契约。
//!
//! 测试同时覆盖项目作用域编码、租户隔离校验以及事件游标的可选查询语义。

use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_api_types::agent_scheduler::{
    AgentEventKindDto, AgentSchedulerBulkCommandRequest, AgentSchedulerCommandKindDto,
    AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest, AgentSchedulerCreateTaskRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyTaskImportDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerOutboxDrainRequest, AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskGraphEdgeDto,
    AgentSchedulerTaskMetadataPatchDto, AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerUpsertTeamRequest, AgentSchedulerUpsertWorkerRequest, AgentSchedulerWakeRequest,
    AgentSchedulerWorkerRunEventKindDto, AgentSchedulerWorkerRunEventRequest, AgentTaskKindDto,
    AgentTaskStatusDto, AgentTeamMemberDto, AgentTeamStatusDto, AgentWorkerStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::project::ListProjectsRequest;
use vw_api_types::project::TenantScopeDto;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
};

use crate::client::test_support;
use crate::endpoint::GatewayEndpoint;

fn scheduler_scope(project_id: &str) -> AgentSchedulerScopeDto {
    AgentSchedulerScopeDto::for_default_tenant(ProjectId::from(project_id))
        .expect("scheduler scope")
}

#[tokio::test]
async fn agent_scheduler_client_reads_and_replaces_task_graph() {
    let tenant = TenantScope::new("platform-graph", "company-graph").expect("tenant scope");
    let scope = AgentSchedulerScopeDto::new(
        ProjectScope::new(tenant.clone(), ProjectId::from("project/one ?")).expect("project scope"),
    );
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({
                    "project_id": "project/one ?",
                    "edges": [{
                        "predecessor_task_id": "task/prepare",
                        "successor_task_id": "task/review"
                    }],
                    "tasks": [{
                        "task_id": "task/review",
                        "status": "queued",
                        "readiness": "waiting_dependencies",
                        "waiting_task_ids": ["task/prepare"],
                        "blocking_task_ids": []
                    }]
                }),
            ),
            (200, json!({"project_id": "project/one ?", "edges": [], "tasks": []})),
        ],
        tenant.clone(),
    );
    let replace = AgentSchedulerReplaceTaskGraphRequest {
        edges: vec![AgentSchedulerTaskGraphEdgeDto {
            predecessor_task_id: "task/prepare".to_string(),
            successor_task_id: "task/review".to_string(),
        }],
    };

    let snapshot =
        server.client().agent_scheduler_task_graph(&scope).await.expect("read task graph");
    let replaced = server
        .client()
        .agent_scheduler_replace_task_graph(&scope, &replace)
        .await
        .expect("replace task graph");

    assert_eq!(snapshot.edges, replace.edges);
    assert!(replaced.edges.is_empty());
    let get = server.take_request();
    let put = server.take_request();
    let path = "/v1/agent-scheduler/projects/project%2Fone%20%3F/task-graph";
    assert_eq!(get.method, "GET");
    assert_eq!(get.path, path);
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, path);
    assert_eq!(put.body["edges"][0]["predecessor_task_id"], "task/prepare");
    assert_eq!(put.body["edges"][0]["successor_task_id"], "task/review");
    for request in [&get, &put] {
        assert_eq!(
            request.header(vw_api_types::id::PLATFORM_ID_HEADER),
            Some(tenant.platform_id().as_ref())
        );
        assert_eq!(
            request.header(vw_api_types::id::COMPANY_ID_HEADER),
            Some(tenant.company_id().as_ref())
        );
    }
    server.join();
}

#[tokio::test]
async fn agent_scheduler_events_omit_cursor_query_when_absent() {
    let server = test_support::server(vec![(200, json!({"cursor": 0, "events": []}))]);
    let scope = scheduler_scope("项目/one");

    let response =
        server.client().agent_scheduler_events(&scope, None).await.expect("scheduler events");
    let stream_path =
        crate::client::GatewayClient::agent_scheduler_events_stream_path(&scope, None);

    assert_eq!(response.cursor, 0);
    assert!(response.events.is_empty());
    assert_eq!(stream_path, "/v1/agent-scheduler/projects/%E9%A1%B9%E7%9B%AE%2Fone/events/stream");
    let request = server.take_request();
    assert_eq!(request.path, "/v1/agent-scheduler/projects/%E9%A1%B9%E7%9B%AE%2Fone/events");
    server.join();
}

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

#[tokio::test]
async fn agent_scheduler_client_rejects_scope_that_conflicts_with_endpoint_tenant() {
    let client = crate::client::GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", 9).with_tenant_scope(TenantScope::default()),
    )
    .expect("gateway client");
    let scope = AgentSchedulerScopeDto::new(
        ProjectScope::new(
            TenantScope::new("other-platform", "other-company").expect("tenant scope"),
            ProjectId::from("project-one"),
        )
        .expect("project scope"),
    );

    let error = client.agent_scheduler_status(&scope).await.expect_err("scope mismatch");
    assert_eq!(error, "tenant scope does not match gateway endpoint tenant scope");
}

#[tokio::test]
async fn agent_scheduler_client_creates_tasks_with_native_scheduler_path() {
    let server = test_support::server(vec![(
        200,
        json!({
            "task_id": "task/create",
            "task": {
                "id": "task/create",
                "title": "Create through Agent Scheduler",
                "kind": "finite",
                "status": "queued",
                "created_at_ms": 30,
                "updated_at_ms": 30,
                "metadata": {"prompt": "create this task"}
            }
        }),
    )]);
    let request = AgentSchedulerCreateTaskRequest {
        task_id: "task/create".to_string(),
        title: "Create through Agent Scheduler".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: Some(AgentTaskStatusDto::Queued),
        spec_uri: "task-board/task-create/spec.md".to_string(),
        meta_uri: "task-board/task-create/meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        metadata: json!({"prompt": "create this task"}),
        created_at_ms: TimestampMs(30),
    };

    let response = server
        .client()
        .agent_scheduler_create_task(&scheduler_scope("project/one"), &request)
        .await
        .expect("create scheduler task");

    assert_eq!(response.task_id, "task/create");
    assert_eq!(response.task.status, AgentTaskStatusDto::Queued);

    let post = server.take_request();
    assert_eq!(post.method, "POST");
    assert_eq!(post.path, "/v1/agent-scheduler/projects/project%2Fone/tasks");
    assert!(!post.path.starts_with("/v1/task-pool"));
    assert!(post.body.get("project_path").is_none());
    assert_eq!(post.body["task_id"], "task/create");
    assert_eq!(post.body["kind"], "finite");
    assert_eq!(post.body["status"], "queued");
    assert_eq!(post.body["metadata"]["prompt"], "create this task");
    server.join();
}

#[tokio::test]
async fn agent_scheduler_client_uses_task_board_contract_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "project_id": "project/one",
                "workflow": {
                    "id": "content_publishing",
                    "label": "Content Publishing",
                    "statuses": [{
                        "key": "content_writing",
                        "label": "Writing",
                        "role": "executing"
                    }]
                }
            }),
        ),
        (
            200,
            json!({
                "accepted": 1,
                "rejected": 0,
                "results": [{"accepted": true}]
            }),
        ),
        (
            200,
            json!({
                "task": {
                    "project_id": "project/one",
                    "task_id": "task/custom",
                    "status_key": "content_writing",
                    "priority": 3,
                    "order_no": 4,
                    "model": "auto",
                    "prompt": "write",
                    "retry_count": 0,
                    "updated_at_ms": 22
                }
            }),
        ),
        (
            200,
            json!({
                "imported": 1,
                "skipped": 0,
                "tasks": [],
                "events": []
            }),
        ),
    ]);
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "content_publishing".to_string(),
        label: "Content Publishing".to_string(),
        statuses: vec![TaskPoolWorkflowStatusDto {
            key: "content_writing".to_string(),
            label: "Writing".to_string(),
            role: TaskPoolWorkflowStatusRoleDto::Executing,
            enabled: true,
            terminal: false,
            next: None,
            transitions: Vec::new(),
        }],
    };
    let move_command = AgentSchedulerCommandRequest {
        task_id: "task/custom".into(),
        command: AgentSchedulerCommandKindDto::Move,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: Some("drag".to_string()),
        payload: AgentSchedulerCommandPayloadDto {
            move_task: Some(AgentSchedulerMoveTaskPayloadDto {
                from_status_key: Some("content_queued".to_string()),
                to_status_key: "content_writing".to_string(),
                to_status: None,
                order_no: Some(7),
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    };
    let bulk_request =
        AgentSchedulerBulkCommandRequest { commands: vec![move_command], stop_on_error: true };
    let metadata_request = AgentSchedulerTaskMetadataUpdateRequest {
        patch: AgentSchedulerTaskMetadataPatchDto {
            status_key: Some("content_writing".to_string()),
            status_role: Some(TaskPoolWorkflowStatusRoleDto::Executing),
            metadata_merge: Some(json!({"task_board.workflow_id": "content_publishing"})),
            ..AgentSchedulerTaskMetadataPatchDto::default()
        },
        reason: Some("sync projection".to_string()),
    };
    let import_request = AgentSchedulerLegacyImportRequest {
        workflow: Some(workflow),
        tasks: vec![AgentSchedulerLegacyTaskImportDto {
            task_id: "task/imported".into(),
            title: "Imported".to_string(),
            status_key: Some("content_writing".to_string()),
            status: None,
            priority: 1,
            order_no: 2,
            model: "auto".to_string(),
            agent: Some("main".to_string()),
            acp_agent: None,
            reasoning_effort: None,
            prompt: "import me".to_string(),
            workflow_id: Some("content_publishing".to_string()),
            metadata: json!({"legacy": true}),
            created_at_ms: TimestampMs(20),
            updated_at_ms: None,
        }],
        dry_run: false,
    };

    let workflow_response = server
        .client()
        .agent_scheduler_workflow(&scheduler_scope("project/one"))
        .await
        .expect("workflow");
    let bulk = server
        .client()
        .agent_scheduler_bulk_command(&scheduler_scope("project/one"), &bulk_request)
        .await
        .expect("bulk");
    let metadata = server
        .client()
        .agent_scheduler_update_task_metadata(
            &scheduler_scope("project/one"),
            "task/custom",
            &metadata_request,
        )
        .await
        .expect("metadata");
    let imported = server
        .client()
        .agent_scheduler_import_legacy_tasks(&scheduler_scope("project/one"), &import_request)
        .await
        .expect("import");

    assert_eq!(workflow_response.workflow.expect("workflow").id, "content_publishing");
    assert_eq!(bulk.accepted, 1);
    assert_eq!(metadata.task.status_key, "content_writing");
    assert_eq!(imported.imported, 1);

    let workflow_get = server.take_request();
    let bulk_post = server.take_request();
    let metadata_patch = server.take_request();
    let import_post = server.take_request();
    assert_eq!(workflow_get.method, "GET");
    assert_eq!(workflow_get.path, "/v1/agent-scheduler/projects/project%2Fone/workflow");
    assert_eq!(bulk_post.method, "POST");
    assert_eq!(bulk_post.path, "/v1/agent-scheduler/projects/project%2Fone/commands/bulk");
    assert_eq!(bulk_post.body["commands"][0]["command"], "move");
    assert_eq!(
        bulk_post.body["commands"][0]["payload"]["move_task"]["to_status_key"],
        "content_writing"
    );
    assert_eq!(metadata_patch.method, "PATCH");
    assert_eq!(
        metadata_patch.path,
        "/v1/agent-scheduler/projects/project%2Fone/tasks/task%2Fcustom/metadata"
    );
    assert_eq!(metadata_patch.body["patch"]["status_key"], "content_writing");
    assert_eq!(import_post.method, "POST");
    assert_eq!(import_post.path, "/v1/agent-scheduler/projects/project%2Fone/legacy-import");
    assert_eq!(import_post.body["tasks"][0]["task_id"], "task/imported");
    assert!(
        [&workflow_get, &bulk_post, &metadata_patch, &import_post]
            .iter()
            .all(|request| !request.path.starts_with("/v1/task-pool"))
    );
    server.join();
}
