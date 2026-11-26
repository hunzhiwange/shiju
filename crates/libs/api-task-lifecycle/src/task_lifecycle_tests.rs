//! 生产 DTO 必须保留步骤规则且拒绝旧角色配置。
use crate::*;
use serde_json::json;

fn workflow() -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: "content".into(),
        label: "Content".into(),
        built_in: false,
        initial_status_key: "draft".into(),
        statuses: vec![
            TaskLifecycleStatusDto {
                key: "draft".into(),
                label: "Draft".into(),
                category: TaskLifecycleStatusCategoryDto::Unstarted,
                enabled: true,
            },
            TaskLifecycleStatusDto {
                key: "editing".into(),
                label: "Editing".into(),
                category: TaskLifecycleStatusCategoryDto::Started,
                enabled: true,
            },
            TaskLifecycleStatusDto {
                key: "published".into(),
                label: "Published".into(),
                category: TaskLifecycleStatusCategoryDto::Done,
                enabled: true,
            },
        ],
        steps: vec![
            TaskLifecycleStepDto {
                status_key: "draft".into(),
                handoff: TaskLifecycleStepHandoffDto::Member { member_id: "alice".into() },
            },
            TaskLifecycleStepDto {
                status_key: "editing".into(),
                handoff: TaskLifecycleStepHandoffDto::Agent {
                    agent_id: "editor".into(),
                    instructions: "Review the draft".into(),
                    completion_status_key: None,
                    return_status_key: Some("draft".into()),
                },
            },
            TaskLifecycleStepDto {
                status_key: "published".into(),
                handoff: TaskLifecycleStepHandoffDto::KeepAssignee,
            },
        ],
    }
}

#[test]
fn task_lifecycle_workflow_dto_round_trips_and_validates() {
    let workflow = workflow();
    workflow.validate().unwrap();
    let encoded =
        serde_json::to_value(TaskLifecycleWorkflowUpsertBody { workflow: workflow.clone() })
            .unwrap();
    assert_eq!(encoded["workflow"]["statuses"][1]["category"], "started");
    assert_eq!(encoded["workflow"]["steps"][1]["handoff"]["kind"], "agent");
    let decoded: TaskLifecycleWorkflowUpsertBody = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.workflow, workflow);
    assert_eq!(decoded.workflow.completion_status_key("editing"), Some("published"));
    assert_eq!(
        serde_json::to_value(TaskLifecycleWorkflowDeleteResponse {
            id: "content".into(),
            deleted: true
        })
        .unwrap(),
        json!({"id":"content", "deleted":true})
    );
}

#[test]
fn task_lifecycle_workflow_rejects_legacy_configuration_without_conversion() {
    let mut payload = serde_json::to_value(workflow()).unwrap();
    payload["statuses"][0]["role"] = json!("backlog");
    assert!(serde_json::from_value::<TaskLifecycleWorkflowDto>(payload).is_err());
    let mut payload = serde_json::to_value(workflow()).unwrap();
    payload.as_object_mut().unwrap().remove("steps");
    assert!(serde_json::from_value::<TaskLifecycleWorkflowDto>(payload).is_err());
}
