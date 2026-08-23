//! 验证任务生命周期工作流的序列化契约及状态转换约束。

use crate::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};
use serde_json::json;

#[test]
fn task_lifecycle_workflow_dto_round_trips_and_validates() {
    let workflow = TaskLifecycleWorkflowDto {
        id: "default".to_string(),
        label: "Default lifecycle".to_string(),
        built_in: true,
        statuses: vec![
            TaskLifecycleStatusDto {
                key: "queued".to_string(),
                label: "Queued".to_string(),
                role: TaskLifecycleStatusRoleDto::Queued,
                enabled: true,
                terminal: false,
                auto_execution_candidate: true,
                occupies_execution_slot: false,
                is_scheduling_backlog: true,
                next: Some("running".to_string()),
                transitions: Vec::new(),
            },
            TaskLifecycleStatusDto {
                key: "running".to_string(),
                label: "Running".to_string(),
                role: TaskLifecycleStatusRoleDto::Executing,
                enabled: true,
                terminal: false,
                auto_execution_candidate: true,
                occupies_execution_slot: true,
                is_scheduling_backlog: false,
                next: None,
                transitions: vec![TaskLifecycleStatusTransitionDto {
                    outcome: "done".to_string(),
                    label: Some("Done".to_string()),
                    target: "completed".to_string(),
                }],
            },
            TaskLifecycleStatusDto {
                key: "completed".to_string(),
                label: "Completed".to_string(),
                role: TaskLifecycleStatusRoleDto::Done,
                enabled: true,
                terminal: true,
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            },
        ],
    };

    workflow.validate().expect("valid lifecycle workflow");

    let encoded = serde_json::to_value(&workflow).expect("serialize workflow");
    assert_eq!(
        encoded,
        json!({
            "id": "default",
            "label": "Default lifecycle",
            "built_in": true,
            "statuses": [
                {
                    "key": "queued",
                    "label": "Queued",
                    "role": "queued",
                    "auto_execution_candidate": true,
                    "is_scheduling_backlog": true,
                    "next": "running"
                },
                {
                    "key": "running",
                    "label": "Running",
                    "role": "executing",
                    "auto_execution_candidate": true,
                    "occupies_execution_slot": true,
                    "transitions": [
                        {
                            "outcome": "done",
                            "label": "Done",
                            "target": "completed"
                        }
                    ]
                },
                {
                    "key": "completed",
                    "label": "Completed",
                    "role": "done",
                    "terminal": true
                }
            ]
        })
    );

    let decoded: TaskLifecycleWorkflowDto =
        serde_json::from_value(encoded).expect("deserialize workflow");
    assert_eq!(decoded, workflow);

    let upsert = TaskLifecycleWorkflowUpsertBody { workflow: workflow.clone() };
    assert_eq!(
        serde_json::to_value(&upsert).expect("serialize upsert body"),
        json!({ "workflow": workflow })
    );

    let deleted = TaskLifecycleWorkflowDeleteResponse { id: "custom".to_string(), deleted: true };
    assert_eq!(
        serde_json::to_value(deleted).expect("serialize delete response"),
        json!({ "id": "custom", "deleted": true })
    );

    let mut invalid = decoded.clone();
    invalid.id = " ".to_string();
    assert_eq!(invalid.validate(), Err("workflow id must not be empty".to_string()));

    let mut invalid = decoded.clone();
    invalid.statuses[1].key = "queued".to_string();
    assert_eq!(invalid.validate(), Err("workflow status key must be unique: queued".to_string()));

    let mut invalid = decoded.clone();
    invalid.statuses[0].next = Some("missing".to_string());
    assert_eq!(invalid.validate(), Err("unknown workflow next target: missing".to_string()));

    let mut invalid = decoded.clone();
    invalid.statuses[1].transitions[0].outcome.clear();
    assert_eq!(
        invalid.validate(),
        Err("workflow transition outcome must not be empty: running".to_string())
    );

    let mut invalid = decoded;
    invalid.statuses[1].transitions[0].target = "missing".to_string();
    assert_eq!(invalid.validate(), Err("unknown workflow transition target: missing".to_string()));
}

#[test]
fn task_lifecycle_workflow_rejects_blank_status_keys_and_targets() {
    let status = TaskLifecycleStatusDto {
        key: "queued".to_string(),
        label: "Queued".to_string(),
        role: TaskLifecycleStatusRoleDto::Queued,
        enabled: true,
        terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
        transitions: Vec::new(),
    };
    let workflow = TaskLifecycleWorkflowDto {
        id: "workflow".to_string(),
        label: "Workflow".to_string(),
        built_in: false,
        statuses: vec![status],
    };

    let mut invalid = workflow.clone();
    invalid.statuses[0].key = "\n".to_string();
    assert_eq!(invalid.validate(), Err("workflow status key must not be empty".to_string()));

    let mut invalid = workflow.clone();
    invalid.statuses[0].next = Some(" ".to_string());
    assert_eq!(invalid.validate(), Err("workflow next target must not be empty".to_string()));

    let mut invalid = workflow;
    invalid.statuses[0].transitions.push(TaskLifecycleStatusTransitionDto {
        outcome: "retry".to_string(),
        label: None,
        target: "\t".to_string(),
    });
    assert_eq!(invalid.validate(), Err("workflow transition target must not be empty".to_string()));
}
