use crate::staff::{
    StaffCapabilityIoSummaryDto, StaffCapabilityKindDto, StaffCommandActionDto, StaffCommandDto,
    StaffCommandRequest, StaffProjectCapabilityCatalogRequest,
    StaffProjectCapabilityCatalogResponse, StaffProjectCapabilityDto, StaffStageActionDto,
    StaffStageActionInputsDto, StaffStageAssignmentDto, StaffStageCapabilityDto,
    StaffStageWorkflowBindingDto, StaffWorkflowOrchestrationDto, StaffWorkflowOrchestrationRequest,
    StaffWorkflowOrchestrationResponse,
};
use crate::task::TaskPoolWorkflowStatusRoleDto;
use serde_json::json;

#[test]
fn staff_command_contract_uses_stable_tagged_shapes() {
    let request = StaffCommandRequest {
        command: StaffCommandDto::StartTask {
            command_id: "cmd-1".to_string(),
            task_id: "task-1".to_string(),
            member_id: "staff-executor".to_string(),
        },
        dry_run: true,
    };
    let value = serde_json::to_value(&request).expect("serialize staff command");

    assert_eq!(value["command"]["type"], "start_task");
    assert_eq!(value["command"]["command_id"], "cmd-1");
    assert_eq!(value["command"]["task_id"], "task-1");
    assert_eq!(value["command"]["member_id"], "staff-executor");
    assert_eq!(value["dry_run"], true);

    let action = StaffCommandActionDto::RunStageWorkflow {
        task_id: "task-1".to_string(),
        stage_id: "review".to_string(),
        status_key: "code_review".to_string(),
        workflow_uuid: "workflow-1".to_string(),
        member_id: "staff-reviewer".to_string(),
    };
    let value = serde_json::to_value(action).expect("serialize action");
    assert_eq!(value["type"], "run_stage_workflow");
    assert_eq!(value["workflow_uuid"], "workflow-1");

    let capability = StaffStageCapabilityDto {
        capability_id: "product_review".to_string(),
        label: "Product Review".to_string(),
        capability_type: StaffCapabilityKindDto::SavedWorkflow,
        built_in: false,
        supported_roles: vec![TaskPoolWorkflowStatusRoleDto::Review],
        requires_workflow_uuid: true,
        input_summary: StaffCapabilityIoSummaryDto {
            description: "Workflow UUID plus task summary".to_string(),
            fields: vec!["workflow_uuid".to_string()],
        },
        result_summary: StaffCapabilityIoSummaryDto {
            description: "Review result".to_string(),
            fields: vec!["status".to_string()],
        },
        input_contract: "Workflow UUID plus task summary".to_string(),
        result_semantics: "Review result".to_string(),
        permission_hint: "Use scoped workflow access".to_string(),
    };
    let value = serde_json::to_value(&capability).expect("serialize capability");
    assert_eq!(value["type"], "saved_workflow");
    assert_eq!(value["built_in"], false);
    assert_eq!(value["input_summary"]["fields"][0], "workflow_uuid");
    assert_eq!(
        serde_json::from_value::<StaffStageCapabilityDto>(value).expect("decode capability"),
        capability
    );
}

#[test]
fn staff_project_capability_catalog_contract_round_trips_custom_entries() {
    let request = StaffProjectCapabilityCatalogRequest {
        capabilities: vec![StaffProjectCapabilityDto {
            id: " release_workflow ".to_string(),
            label: " Release Workflow ".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Release result".to_string(),
                fields: vec!["summary".to_string()],
            },
        }],
    };

    let value = serde_json::to_value(&request).expect("serialize project catalog request");
    assert_eq!(value["capabilities"][0]["id"], " release_workflow ");
    assert_eq!(value["capabilities"][0]["type"], "saved_workflow");
    assert_eq!(value["capabilities"][0]["input_summary"]["fields"][1], "release_channel");
    assert_eq!(
        serde_json::from_value::<StaffProjectCapabilityCatalogRequest>(value)
            .expect("decode project catalog request"),
        request
    );

    let response = StaffProjectCapabilityCatalogResponse {
        capabilities: vec![StaffStageCapabilityDto {
            capability_id: "release_workflow".to_string(),
            label: "Release Workflow".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            supported_roles: vec![TaskPoolWorkflowStatusRoleDto::Queued],
            requires_workflow_uuid: true,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Release result".to_string(),
                fields: vec!["summary".to_string()],
            },
            input_contract: "Workflow UUID and release channel".to_string(),
            result_semantics: "Release result".to_string(),
            permission_hint: "Use scoped workflow access".to_string(),
        }],
    };
    let value = serde_json::to_value(&response).expect("serialize project catalog response");
    assert_eq!(value["capabilities"][0]["capability_id"], "release_workflow");
    assert_eq!(
        serde_json::from_value::<StaffProjectCapabilityCatalogResponse>(value)
            .expect("decode project catalog response"),
        response
    );
}

#[test]
fn staff_workflow_orchestration_dto_roundtrips_without_project_assignments() {
    let orchestration = StaffWorkflowOrchestrationDto {
        capabilities: vec![StaffProjectCapabilityDto {
            id: "product_review".to_string(),
            label: "Product Review".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and review focus".to_string(),
                fields: vec!["workflow_uuid".to_string(), "focus".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Review result".to_string(),
                fields: vec!["status".to_string()],
            },
        }],
        stage_assignments: vec![StaffStageAssignmentDto {
            stage_id: "code_review".to_string(),
            primary_member_id: Some("staff-reviewer".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }],
        stage_workflow_bindings: vec![StaffStageWorkflowBindingDto {
            stage_id: "planning".to_string(),
            workflow_uuid: "workflow-planning".to_string(),
        }],
        stage_actions: vec![StaffStageActionDto {
            stage_id: "code_review".to_string(),
            capability_id: "product_review".to_string(),
            workflow_uuid: None,
            inputs: StaffStageActionInputsDto {
                workflow_uuid: Some("workflow-product-review".to_string()),
                parameters: serde_json::Map::from_iter([("focus".to_string(), json!("release"))]),
            },
        }],
    };

    let request = StaffWorkflowOrchestrationRequest {
        workflow_id: "dev_lifecycle".to_string(),
        orchestration: orchestration.clone(),
    };
    let value = serde_json::to_value(&request).expect("serialize workflow orchestration request");
    assert_eq!(value["workflow_id"], "dev_lifecycle");
    assert!(value["orchestration"].get("capabilities").is_some());
    assert!(value["orchestration"].get("stage_assignments").is_some());
    assert!(value["orchestration"].get("stage_workflow_bindings").is_some());
    assert!(value["orchestration"].get("stage_actions").is_some());
    assert!(value["orchestration"].get("member_task_assignments").is_none());
    assert_eq!(
        serde_json::from_value::<StaffWorkflowOrchestrationRequest>(value)
            .expect("decode workflow orchestration request"),
        request
    );

    let response = StaffWorkflowOrchestrationResponse {
        workflow_id: "dev_lifecycle".to_string(),
        orchestration,
    };
    let value = serde_json::to_value(&response).expect("serialize workflow orchestration response");
    assert!(value["orchestration"].get("member_task_assignments").is_none());
    assert_eq!(
        serde_json::from_value::<StaffWorkflowOrchestrationResponse>(value)
            .expect("decode workflow orchestration response"),
        response
    );
}
