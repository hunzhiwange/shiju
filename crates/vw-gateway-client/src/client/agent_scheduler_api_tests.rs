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

#[tokio::test]
async fn agent_scheduler_client_manages_workers_and_teams_on_independent_paths() {
    let server = test_support::server(vec![
        (200, json!({"workers": []})),
        (
            200,
            json!({
                "worker": {
                    "id": "worker-one",
                    "runtime": "codex",
                    "deployment": "desktop",
                    "model": "gpt-5",
                    "status": "idle",
                    "updated_at_ms": 101
                }
            }),
        ),
        (200, json!({"teams": []})),
        (
            200,
            json!({
                "team": {
                    "id": "team-one",
                    "name": "Team One",
                    "status": "active",
                    "leader_id": "worker-one",
                    "leader_ready": true,
                    "ready_workers": 1,
                    "total_workers": 1,
                    "members": [{
                        "worker_id": "worker-one",
                        "role": "team_leader",
                        "ready": true
                    }],
                    "updated_at_ms": 102
                }
            }),
        ),
    ]);
    let project_id = "project/one ?";
    let worker_request = AgentSchedulerUpsertWorkerRequest {
        worker_id: "worker-one".to_string(),
        runtime: Some("codex".to_string()),
        deployment: Some("desktop".to_string()),
        model: Some("gpt-5".to_string()),
        model_provider: Some("openai".to_string()),
        status: Some(AgentWorkerStatusDto::Idle),
        desired_state: Some("running".to_string()),
        team_id: Some("team-one".to_string()),
        worker_role: Some("worker".to_string()),
        matrix_user_id: None,
        room_id: None,
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: Some(TimestampMs(100)),
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: Some(TimestampMs(101)),
    };
    let team_request = AgentSchedulerUpsertTeamRequest {
        team_id: "team-one".to_string(),
        name: "Team One".to_string(),
        status: Some(AgentTeamStatusDto::Active),
        leader_id: "worker-one".to_string(),
        team_room_id: None,
        leader_dm_room_id: None,
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: vec![AgentTeamMemberDto {
            worker_id: "worker-one".to_string(),
            runtime_name: Some("codex".to_string()),
            role: "team_leader".to_string(),
            room_id: None,
            matrix_user_id: None,
            ready: true,
        }],
        updated_at_ms: Some(TimestampMs(102)),
    };

    let workers = server
        .client()
        .agent_scheduler_workers(&scheduler_scope(project_id))
        .await
        .expect("workers");
    let worker = server
        .client()
        .agent_scheduler_upsert_worker(&scheduler_scope(project_id), &worker_request)
        .await
        .expect("upsert worker");
    let teams =
        server.client().agent_scheduler_teams(&scheduler_scope(project_id)).await.expect("teams");
    let team = server
        .client()
        .agent_scheduler_upsert_team(&scheduler_scope(project_id), &team_request)
        .await
        .expect("upsert team");

    assert!(workers.workers.is_empty());
    assert_eq!(worker.worker.id, "worker-one");
    assert!(teams.teams.is_empty());
    assert_eq!(team.team.id, "team-one");

    let workers_get = server.take_request();
    let worker_post = server.take_request();
    let teams_get = server.take_request();
    let team_post = server.take_request();
    let project_path = "/v1/agent-scheduler/projects/project%2Fone%20%3F";
    assert_eq!(workers_get.method, "GET");
    assert_eq!(workers_get.path, format!("{project_path}/workers"));
    assert_eq!(worker_post.method, "POST");
    assert_eq!(worker_post.path, format!("{project_path}/workers"));
    assert_eq!(worker_post.body["worker_id"], "worker-one");
    assert_eq!(worker_post.body["deployment"], "desktop");
    assert_eq!(teams_get.method, "GET");
    assert_eq!(teams_get.path, format!("{project_path}/teams"));
    assert_eq!(team_post.method, "POST");
    assert_eq!(team_post.path, format!("{project_path}/teams"));
    assert_eq!(team_post.body["team_id"], "team-one");
    assert_eq!(team_post.body["members"][0]["worker_id"], "worker-one");
    server.join();
}

#[tokio::test]
async fn agent_scheduler_client_uses_native_scheduler_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "task_id": "task/a",
                "task": {
                    "id": "task/a",
                    "title": "Task A",
                    "kind": "finite",
                    "status": "queued",
                    "created_at_ms": 8,
                    "updated_at_ms": 8,
                    "metadata": {"prompt": "prompt for task/a"}
                }
            }),
        ),
        (
            200,
            json!({
                "worker": {
                    "id": "matrix-local",
                    "runtime": "matrix",
                    "model": "auto",
                    "status": "idle",
                    "desired_state": "Running",
                    "worker_role": "worker",
                    "matrix_user_id": "@tester:localhost",
                    "room_id": "!room:localhost",
                    "container_state": "running",
                    "updated_at_ms": 9
                }
            }),
        ),
        (
            200,
            json!({
            "workers": [{
                "id": "matrix-local",
                "runtime": "matrix",
                "model": "auto",
                "status": "idle",
                    "desired_state": "Running",
                    "worker_role": "worker",
                    "matrix_user_id": "@tester:localhost",
                    "room_id": "!room:localhost",
                    "container_state": "running",
                    "updated_at_ms": 9
                }]
            }),
        ),
        (
            200,
            json!({
                "accepted": true,
                "task": {
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "status_key": "queued",
                    "status": "queued",
                    "priority": 1,
                    "order_no": 0,
                    "model": "auto",
                    "agent": "main",
                    "prompt": "prompt for task/a",
                    "retry_count": 0,
                    "run_id": "run-1",
                    "run_stage": "queued",
                    "workspace": {
                        "slot_id": "slot-1",
                        "path": "/work/project-one",
                        "branch": "agent/task-a"
                    },
                    "updated_at_ms": 10
                },
                "event": {
                    "cursor": 2,
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "run_id": "run-1",
                    "kind": "task_started",
                    "from_status_key": "queued",
                    "from_status": "queued",
                    "to_status_key": "running",
                    "to_status": "running",
                    "message": "started",
                    "workspace": {
                        "slot_id": "slot-1",
                        "path": "/work/project-one",
                        "branch": "agent/task-a"
                    },
                    "payload": {"command": "start"},
                    "created_at_ms": 11
                }
            }),
        ),
        (
            200,
            json!({
                "project_id": "project/one",
                "tasks": [{
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "status_key": "running",
                    "status": "running",
                    "priority": 1,
                    "order_no": 0,
                    "model": "auto",
                    "agent": "main",
                    "prompt": "prompt for task/a",
                    "retry_count": 0,
                    "run_id": "run-1",
                    "run_stage": "executing",
                    "workspace": {
                        "slot_id": "slot-1",
                        "path": "/work/project-one",
                        "branch": "agent/task-a"
                    },
                    "updated_at_ms": 12
                }]
            }),
        ),
        (
            200,
            json!({
                "task_id": "task/a",
                "logs": [{
                    "task_id": "task/a",
                    "seq_no": 1,
                    "timestamp_ms": 13,
                    "from_status_key": "queued",
                    "status_from": "queued",
                    "to_status_key": "running",
                    "status_to": "running",
                    "message": "started"
                }]
            }),
        ),
        (
            200,
            json!({
                "task_id": "task/a",
                "artifacts": [{
                    "task_id": "task/a",
                    "run_id": "run-1",
                    "kind": "execution_result",
                    "name": "execution-result.json",
                    "path": "shared/tasks/task-a/artifacts/execution-result.json",
                    "payload": {"success": true},
                    "created_at_ms": 14
                }]
            }),
        ),
        (
            200,
            json!({
                "cursor": 2,
                "events": [{
                    "cursor": 2,
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "run_id": "run-1",
                    "run_stage": "executing",
                    "kind": "task_started",
                    "from_status_key": "queued",
                    "from_status": "queued",
                    "to_status_key": "running",
                    "to_status": "running",
                    "message": "started",
                    "workspace": {
                        "slot_id": "slot-1",
                        "path": "/work/project-one",
                        "branch": "agent/task-a"
                    },
                    "payload": {"command": "start"},
                    "created_at_ms": 11
                }]
            }),
        ),
        (
            200,
            json!({
                "dispatched": true,
                "task_id": "task/a",
                "run_id": "run-1",
                "worker_id": "worker-a",
                "outbox_message_id": "outbox-1"
            }),
        ),
        (
            200,
            json!({
                "claimed": 1,
                "messages": [{
                    "id": "outbox-1",
                    "target_kind": "matrix_room",
                    "target_id": "!room:example.test",
                    "recipient_id": "@worker-a:example.test",
                    "body": "task/a ready",
                    "status": "pending",
                    "idempotency_key": "task/a:dispatch",
                    "attempts": 0,
                    "next_attempt_at_ms": 15,
                    "payload": {"task_id": "task/a"},
                    "created_at_ms": 15
                }]
            }),
        ),
        (
            200,
            json!({
                "accepted": true,
                "task": {
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "status_key": "completed",
                    "status": "completed",
                    "priority": 1,
                    "order_no": 0,
                    "model": "auto",
                    "agent": "main",
                    "prompt": "prompt for task/a",
                    "retry_count": 0,
                    "run_id": "run/1",
                    "run_stage": "succeeded",
                    "updated_at_ms": 22
                },
                "run": {
                    "id": "run/1",
                    "task_id": "task/a",
                    "worker_id": "matrix-local",
                    "status": "succeeded",
                    "attempt": 1,
                    "started_at_ms": 20,
                    "finished_at_ms": 22
                },
                "event": {
                    "cursor": 3,
                    "project_id": "project/one",
                    "task_id": "task/a",
                    "run_id": "run/1",
                    "run_stage": "succeeded",
                    "kind": "run_completed",
                    "from_status_key": "submitted",
                    "from_status": "submitted",
                    "to_status_key": "completed",
                    "to_status": "completed",
                    "message": "completed by matrix worker",
                    "payload": {"source": "matrix_worker"},
                    "created_at_ms": 22
                }
            }),
        ),
    ]);
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
        created_at_ms: TimestampMs(8),
    };
    let command_request = AgentSchedulerCommandRequest {
        task_id: "task/a".into(),
        command: AgentSchedulerCommandKindDto::Start,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: Some(2),
        reason: Some("ready".to_string()),
        payload: AgentSchedulerCommandPayloadDto::default(),
    };
    let worker_request = AgentSchedulerUpsertWorkerRequest {
        worker_id: "matrix-local".to_string(),
        runtime: Some("matrix".to_string()),
        deployment: Some("local".to_string()),
        model: Some("auto".to_string()),
        model_provider: None,
        status: Some(AgentWorkerStatusDto::Idle),
        desired_state: Some("Running".to_string()),
        team_id: None,
        worker_role: Some("worker".to_string()),
        matrix_user_id: Some("@tester:localhost".to_string()),
        room_id: Some("!room:localhost".to_string()),
        container_state: Some("running".to_string()),
        idle_since_ms: None,
        last_seen_at_ms: None,
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: None,
    };

    let created = server
        .client()
        .agent_scheduler_create_task(&scheduler_scope("project/one"), &create_request)
        .await
        .expect("create scheduler task");
    let worker = server
        .client()
        .agent_scheduler_upsert_worker(&scheduler_scope("project/one"), &worker_request)
        .await
        .expect("upsert worker");
    let workers = server
        .client()
        .agent_scheduler_workers(&scheduler_scope("project/one"))
        .await
        .expect("workers");
    let command = server
        .client()
        .agent_scheduler_command(&scheduler_scope("project/one"), &command_request)
        .await
        .expect("post command");
    let status = server
        .client()
        .agent_scheduler_status(&scheduler_scope("project/one"))
        .await
        .expect("status");
    let logs = server
        .client()
        .agent_scheduler_logs(&scheduler_scope("project/one"), "task/a")
        .await
        .expect("logs");
    let artifacts = server
        .client()
        .agent_scheduler_artifacts(&scheduler_scope("project/one"), "task/a")
        .await
        .expect("artifacts");
    let events = server
        .client()
        .agent_scheduler_events(&scheduler_scope("project/one"), Some(1))
        .await
        .expect("events");
    let wake = server
        .client()
        .agent_scheduler_wake(
            &scheduler_scope("project/one"),
            &AgentSchedulerWakeRequest {
                now_ms: TimestampMs(20),
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: Some(2),
                auto_promote_pool_tasks: false,
            },
        )
        .await
        .expect("wake");
    let outbox = server
        .client()
        .agent_scheduler_drain_outbox(
            &scheduler_scope("project/one"),
            &AgentSchedulerOutboxDrainRequest { now_ms: TimestampMs(21), limit: Some(10) },
        )
        .await
        .expect("outbox");
    let worker_event = server
        .client()
        .agent_scheduler_worker_run_event(
            &scheduler_scope("project/one"),
            "task/a",
            "run/1",
            &AgentSchedulerWorkerRunEventRequest {
                platform_id: "local".to_string(),
                company_id: "local".to_string(),
                project_id: "project/one".to_string(),
                worker_id: "matrix-local".to_string(),
                event: AgentSchedulerWorkerRunEventKindDto::Completed,
                message: Some("completed by matrix worker".to_string()),
                result: Some("done".to_string()),
                error_message: None,
                workspace: None,
                payload: json!({"source": "matrix_worker"}),
                now_ms: Some(TimestampMs(22)),
                lease_expires_at_ms: None,
            },
        )
        .await
        .expect("worker event");
    let stream_path = crate::client::GatewayClient::agent_scheduler_events_stream_path(
        &scheduler_scope("project/one"),
        Some(2),
    );

    assert_eq!(created.task_id, "task/a");
    assert_eq!(worker.worker.room_id.as_deref(), Some("!room:localhost"));
    assert_eq!(workers.workers[0].matrix_user_id.as_deref(), Some("@tester:localhost"));
    assert!(command.accepted);
    assert_eq!(command.task.expect("command task").workspace.expect("workspace").slot_id, "slot-1");
    assert_eq!(
        status.tasks[0].workspace.as_ref().expect("status workspace").branch,
        "agent/task-a"
    );
    assert_eq!(logs.logs[0].message, "started");
    assert_eq!(artifacts.artifacts[0].payload, json!({"success": true}));
    assert_eq!(
        events,
        AgentSchedulerTaskBoardEventsResponse { cursor: 2, events: events.events.clone() }
    );
    assert_eq!(events.events[0].kind, AgentEventKindDto::TaskStarted);
    assert_eq!(wake.task_id.as_deref(), Some("task/a"));
    assert_eq!(outbox.claimed, 1);
    assert_eq!(worker_event.task.status_key, "completed");
    assert_eq!(worker_event.run.worker_id.as_deref(), Some("matrix-local"));
    assert_eq!(
        stream_path,
        "/v1/agent-scheduler/projects/project%2Fone/events/stream?after_cursor=2"
    );

    let create_post = server.take_request();
    let worker_post = server.take_request();
    let workers_get = server.take_request();
    let post = server.take_request();
    let status_get = server.take_request();
    let logs_get = server.take_request();
    let artifacts_get = server.take_request();
    let events_get = server.take_request();
    let wake_post = server.take_request();
    let outbox_post = server.take_request();
    let worker_event_post = server.take_request();
    assert_eq!(create_post.method, "POST");
    assert_eq!(create_post.path, "/v1/agent-scheduler/projects/project%2Fone/tasks");
    assert_eq!(create_post.body["task_id"], "task/a");
    assert!(create_post.body.get("project_path").is_none());
    assert_eq!(worker_post.method, "POST");
    assert_eq!(worker_post.path, "/v1/agent-scheduler/projects/project%2Fone/workers");
    assert_eq!(worker_post.body["worker_id"], "matrix-local");
    assert_eq!(worker_post.body["matrix_user_id"], "@tester:localhost");
    assert_eq!(workers_get.method, "GET");
    assert_eq!(workers_get.path, "/v1/agent-scheduler/projects/project%2Fone/workers");
    assert_eq!(post.method, "POST");
    assert_eq!(post.path, "/v1/agent-scheduler/projects/project%2Fone/commands");
    assert_eq!(post.body["command"], "start");
    assert_eq!(post.body["task_id"], "task/a");
    assert_eq!(post.body["max_concurrent"], 2);
    assert_eq!(status_get.method, "GET");
    assert_eq!(status_get.path, "/v1/agent-scheduler/projects/project%2Fone/tasks");
    assert_eq!(logs_get.method, "GET");
    assert_eq!(logs_get.path, "/v1/agent-scheduler/projects/project%2Fone/tasks/task%2Fa/logs");
    assert_eq!(artifacts_get.method, "GET");
    assert_eq!(
        artifacts_get.path,
        "/v1/agent-scheduler/projects/project%2Fone/tasks/task%2Fa/artifacts"
    );
    assert_eq!(events_get.method, "GET");
    assert_eq!(events_get.path, "/v1/agent-scheduler/projects/project%2Fone/events?after_cursor=1");
    assert_eq!(wake_post.method, "POST");
    assert_eq!(wake_post.path, "/v1/agent-scheduler/projects/project%2Fone/wake");
    assert_eq!(wake_post.body["now_ms"], 20);
    assert_eq!(wake_post.body["max_concurrent"], 2);
    assert!(wake_post.body.get("auto_promote_pool_tasks").is_none());
    assert_eq!(outbox_post.method, "POST");
    assert_eq!(outbox_post.path, "/v1/agent-scheduler/projects/project%2Fone/outbox/drain");
    assert_eq!(outbox_post.body["limit"], 10);
    assert_eq!(worker_event_post.method, "POST");
    assert_eq!(
        worker_event_post.path,
        "/v1/agent-scheduler/projects/project%2Fone/tasks/task%2Fa/runs/run%2F1/worker-events"
    );
    assert_eq!(worker_event_post.body["worker_id"], "matrix-local");
    assert_eq!(worker_event_post.body["event"], "completed");
    assert!(
        [
            &post,
            &status_get,
            &logs_get,
            &artifacts_get,
            &events_get,
            &wake_post,
            &outbox_post,
            &create_post,
            &worker_post,
            &workers_get,
            &worker_event_post,
        ]
        .iter()
        .all(|request| !request.path.starts_with("/v1/task-pool")),
        "agent scheduler client must not call legacy task-pool or Matrix routes"
    );
    let tenant = TenantScope::default();
    for request in [
        &create_post,
        &worker_post,
        &workers_get,
        &post,
        &status_get,
        &logs_get,
        &artifacts_get,
        &events_get,
        &wake_post,
        &outbox_post,
        &worker_event_post,
    ] {
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
