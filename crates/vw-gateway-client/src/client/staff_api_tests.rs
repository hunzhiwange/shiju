//! 验证员工运行时、能力目录及项目和工作流编排接口。
//!
//! 项目作用域必须与端点租户一致，冲突请求应在发送前被客户端拒绝。

use serde_json::json;
use vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_api_types::id::{
    COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope,
};
use vw_api_types::staff::{
    StaffAutoSchedulePlanRequest, StaffCapabilityIoSummaryDto, StaffCapabilityKindDto,
    StaffCommandDto, StaffCommandRequest, StaffProjectCapabilityCatalogRequest,
    StaffProjectCapabilityDto, StaffProjectOrchestrationRequest, StaffProjectStateDto,
    StaffStageAssignmentDto, StaffWorkflowOrchestrationDto, StaffWorkflowOrchestrationRequest,
};
use vw_api_types::task::TaskPoolWorkflowSnapshotDto;

use crate::client::test_support;

fn staff_scope() -> AgentSchedulerScopeDto {
    AgentSchedulerScopeDto::for_default_tenant(ProjectId::from("project/one")).expect("staff scope")
}

fn other_tenant_staff_scope() -> AgentSchedulerScopeDto {
    AgentSchedulerScopeDto::new(
        ProjectScope::new(
            TenantScope::new("other-platform", "other-company").expect("tenant scope"),
            ProjectId::from("project/one"),
        )
        .expect("project scope"),
    )
}

#[tokio::test]
async fn staff_api_uses_staff_runtime_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "capabilities": [{
                    "capability_id": "manual",
                    "label": "人工处理",
                    "supported_roles": ["queued"],
                    "requires_workflow_uuid": false,
                    "input_contract": "task",
                    "result_semantics": "manual",
                    "permission_hint": "none"
                }]
            }),
        ),
        (
            200,
            json!({
                "project_id": "project/one",
                "projection": {
                    "lanes": [],
                    "member_loads": [],
                    "unassigned": [],
                    "total_task_count": 0,
                    "blocked_task_count": 0
                },
                "capabilities": []
            }),
        ),
        (
            200,
            json!({
                "accepted": true,
                "message": "staff command validated",
                "action": {
                    "type": "assign_task",
                    "task_id": "task/one",
                    "member_id": "staff-executor"
                }
            }),
        ),
        (
            200,
            json!({
                "project_id": "project/one",
                "plan": {
                    "commands": [],
                    "command_contexts": [],
                    "skipped": []
                }
            }),
        ),
    ]);
    let command = StaffCommandRequest {
        command: StaffCommandDto::AssignTask {
            command_id: "cmd-1".to_string(),
            task_id: "task/one".to_string(),
            member_id: "staff-executor".to_string(),
        },
        dry_run: true,
    };
    let plan = StaffAutoSchedulePlanRequest::default();

    let capabilities =
        server.client().staff_stage_capabilities().await.expect("staff capabilities");
    let projection = server.client().staff_projection(&staff_scope()).await.expect("projection");
    let command_response =
        server.client().staff_command(&staff_scope(), &command).await.expect("staff command");
    let plan_response =
        server.client().staff_auto_schedule_plan(&staff_scope(), &plan).await.expect("staff plan");

    assert_eq!(capabilities.capabilities[0].capability_id, "manual");
    assert_eq!(projection.project_id.as_ref(), "project/one");
    assert!(command_response.accepted);
    assert_eq!(plan_response.project_id.as_ref(), "project/one");

    let capabilities_get = server.take_request();
    let projection_get = server.take_request();
    let command_post = server.take_request();
    let plan_post = server.take_request();
    assert_eq!(capabilities_get.method, "GET");
    assert_eq!(capabilities_get.path, "/v1/staff/stage-capabilities");
    assert_eq!(capabilities_get.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(capabilities_get.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(projection_get.method, "GET");
    assert_eq!(projection_get.path, "/v1/staff/projects/project%2Fone/projection");
    assert_eq!(projection_get.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(projection_get.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(command_post.method, "POST");
    assert_eq!(command_post.path, "/v1/staff/projects/project%2Fone/commands");
    assert_eq!(command_post.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(command_post.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(command_post.body["command"]["type"], "assign_task");
    assert_eq!(command_post.body["command"]["task_id"], "task/one");
    assert_eq!(plan_post.method, "POST");
    assert_eq!(plan_post.path, "/v1/staff/projects/project%2Fone/auto-schedule-plan");
    assert_eq!(plan_post.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(plan_post.header(COMPANY_ID_HEADER), Some("100000000"));
    server.join();
}

#[tokio::test]
async fn staff_client_uses_project_capability_catalog_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "capabilities": [{
                    "capability_id": "manual",
                    "label": "人工处理",
                    "supported_roles": ["queued"],
                    "requires_workflow_uuid": false,
                    "input_contract": "task",
                    "result_semantics": "manual",
                    "permission_hint": "none"
                }]
            }),
        ),
        (
            200,
            json!({
                "capabilities": [{
                    "capability_id": "release_workflow",
                    "label": "Release Workflow",
                    "type": "saved_workflow",
                    "built_in": false,
                    "supported_roles": ["queued"],
                    "requires_workflow_uuid": true,
                    "input_summary": {
                        "description": "Workflow UUID and release channel",
                        "fields": ["workflow_uuid", "release_channel"]
                    },
                    "result_summary": {
                        "description": "Release automation result",
                        "fields": ["summary"]
                    },
                    "input_contract": "Workflow UUID and release channel",
                    "result_semantics": "Release automation result",
                    "permission_hint": "Use scoped workflow access"
                }]
            }),
        ),
    ]);
    let request = StaffProjectCapabilityCatalogRequest {
        capabilities: vec![StaffProjectCapabilityDto {
            id: "release_workflow".to_string(),
            label: "Release Workflow".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Release automation result".to_string(),
                fields: vec!["summary".to_string()],
            },
        }],
    };

    let initial =
        server.client().staff_project_capabilities(&staff_scope()).await.expect("project catalog");
    let saved = server
        .client()
        .save_staff_project_capabilities(&staff_scope(), &request)
        .await
        .expect("save project catalog");

    assert_eq!(initial.capabilities[0].capability_id, "manual");
    assert_eq!(saved.capabilities[0].capability_id, "release_workflow");

    let get = server.take_request();
    let put = server.take_request();
    assert_eq!(get.method, "GET");
    assert_eq!(get.path, "/v1/staff/projects/project%2Fone/capabilities");
    assert_eq!(get.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(get.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, "/v1/staff/projects/project%2Fone/capabilities");
    assert_eq!(put.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(put.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(put.body["capabilities"][0]["id"], "release_workflow");
    assert_eq!(put.body["capabilities"][0]["type"], "saved_workflow");
    server.join();
}

#[tokio::test]
async fn staff_client_reads_and_saves_project_orchestration() {
    let response = json!({
        "project_id": "project/one",
        "workflow": {"id": "staff-flow", "label": "Staff Flow", "statuses": []},
        "project_state": {}
    });
    let server = test_support::server(vec![(200, response.clone()), (200, response)]);
    let request = StaffProjectOrchestrationRequest {
        workflow: TaskPoolWorkflowSnapshotDto {
            id: "staff-flow".to_string(),
            label: "Staff Flow".to_string(),
            statuses: Vec::new(),
        },
        project_state: StaffProjectStateDto::default(),
    };

    let loaded = server
        .client()
        .staff_project_orchestration(&staff_scope())
        .await
        .expect("load project orchestration");
    let saved = server
        .client()
        .save_staff_project_orchestration(&staff_scope(), &request)
        .await
        .expect("save project orchestration");

    assert_eq!(loaded.workflow.id, "staff-flow");
    assert_eq!(saved.project_id.as_ref(), "project/one");
    let get = server.take_request();
    let put = server.take_request();
    assert_eq!(get.method, "GET");
    assert_eq!(get.path, "/v1/staff/projects/project%2Fone/orchestration");
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, "/v1/staff/projects/project%2Fone/orchestration");
    assert_eq!(put.body["workflow"]["id"], "staff-flow");
    assert_eq!(put.body["project_state"]["stage_assignments"], json!([]));
    server.join();
}

#[tokio::test]
async fn staff_client_uses_workflow_orchestration_paths() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "capabilities": [{
                    "capability_id": "manual",
                    "label": "人工处理",
                    "supported_roles": ["queued"],
                    "requires_workflow_uuid": false,
                    "input_contract": "task",
                    "result_semantics": "manual",
                    "permission_hint": "none"
                }]
            }),
        ),
        (
            200,
            json!({
                "workflow_id": "workflow/one",
                "orchestration": {
                    "capabilities": [],
                    "stage_assignments": [{
                        "stage_id": "running",
                        "primary_member_id": "staff-executor",
                        "backup_member_ids": [],
                        "manager_member_ids": [],
                        "autonomous": true
                    }],
                    "stage_workflow_bindings": [],
                    "stage_actions": []
                }
            }),
        ),
        (
            200,
            json!({
                "workflow_id": "workflow/one",
                "orchestration": {
                    "capabilities": [],
                    "stage_assignments": [{
                        "stage_id": "code_review",
                        "primary_member_id": "staff-reviewer",
                        "backup_member_ids": [],
                        "manager_member_ids": [],
                        "autonomous": true
                    }],
                    "stage_workflow_bindings": [],
                    "stage_actions": []
                }
            }),
        ),
        (
            200,
            json!({
                "capabilities": [{
                    "capability_id": "release_workflow",
                    "label": "Release Workflow",
                    "type": "saved_workflow",
                    "built_in": false,
                    "supported_roles": ["queued"],
                    "requires_workflow_uuid": true,
                    "input_summary": {
                        "description": "Workflow UUID and release channel",
                        "fields": ["workflow_uuid", "release_channel"]
                    },
                    "result_summary": {
                        "description": "Release automation result",
                        "fields": ["summary"]
                    },
                    "input_contract": "Workflow UUID and release channel",
                    "result_semantics": "Release automation result",
                    "permission_hint": "Use scoped workflow access"
                }]
            }),
        ),
    ]);
    let orchestration_request = StaffWorkflowOrchestrationRequest {
        workflow_id: "workflow/one".to_string(),
        orchestration: StaffWorkflowOrchestrationDto {
            stage_assignments: vec![StaffStageAssignmentDto {
                stage_id: "code_review".to_string(),
                primary_member_id: Some("staff-reviewer".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            }],
            ..StaffWorkflowOrchestrationDto::default()
        },
    };
    let capability_request = StaffProjectCapabilityCatalogRequest {
        capabilities: vec![StaffProjectCapabilityDto {
            id: "release_workflow".to_string(),
            label: "Release Workflow".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Release automation result".to_string(),
                fields: vec!["summary".to_string()],
            },
        }],
    };

    let catalog = server
        .client()
        .staff_workflow_capabilities(&staff_scope(), "workflow/one")
        .await
        .expect("workflow catalog");
    let loaded = server
        .client()
        .staff_workflow_orchestration(&staff_scope(), "workflow/one")
        .await
        .expect("workflow orchestration");
    let saved = server
        .client()
        .save_staff_workflow_orchestration(&staff_scope(), "workflow/one", &orchestration_request)
        .await
        .expect("save workflow orchestration");
    let saved_catalog = server
        .client()
        .save_staff_workflow_capabilities(&staff_scope(), "workflow/one", &capability_request)
        .await
        .expect("save workflow catalog");

    assert_eq!(catalog.capabilities[0].capability_id, "manual");
    assert_eq!(loaded.workflow_id, "workflow/one");
    assert_eq!(saved.orchestration.stage_assignments[0].stage_id, "code_review");
    assert_eq!(saved_catalog.capabilities[0].capability_id, "release_workflow");

    let catalog_get = server.take_request();
    let orchestration_get = server.take_request();
    let orchestration_put = server.take_request();
    let catalog_put = server.take_request();
    assert_eq!(catalog_get.method, "GET");
    assert_eq!(
        catalog_get.path,
        "/v1/staff/projects/project%2Fone/workflows/workflow%2Fone/capabilities"
    );
    assert_eq!(catalog_get.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(catalog_get.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(orchestration_get.method, "GET");
    assert_eq!(
        orchestration_get.path,
        "/v1/staff/projects/project%2Fone/workflows/workflow%2Fone/orchestration"
    );
    assert_eq!(orchestration_get.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(orchestration_get.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(orchestration_put.method, "PUT");
    assert_eq!(
        orchestration_put.path,
        "/v1/staff/projects/project%2Fone/workflows/workflow%2Fone/orchestration"
    );
    assert_eq!(orchestration_put.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(orchestration_put.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(orchestration_put.body["workflow_id"], "workflow/one");
    assert_eq!(
        orchestration_put.body["orchestration"]["stage_assignments"][0]["stage_id"],
        "code_review"
    );
    assert_eq!(catalog_put.method, "PUT");
    assert_eq!(
        catalog_put.path,
        "/v1/staff/projects/project%2Fone/workflows/workflow%2Fone/capabilities"
    );
    assert_eq!(catalog_put.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(catalog_put.header(COMPANY_ID_HEADER), Some("100000000"));
    assert_eq!(catalog_put.body["capabilities"][0]["id"], "release_workflow");
    server.join();
}

#[tokio::test]
async fn staff_client_rejects_project_scope_that_conflicts_with_endpoint_tenant() {
    let client =
        crate::client::GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", 9))
            .expect("gateway client");

    let error =
        client.staff_projection(&other_tenant_staff_scope()).await.expect_err("scope mismatch");

    assert_eq!(error, "tenant scope does not match gateway endpoint tenant scope");
}
