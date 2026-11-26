use super::*;

fn content_workflow() -> (StepWorkflow, Vec<WorkflowStatus>) {
    let statuses = [
        ("draft", WorkflowStatusCategory::Unstarted),
        ("editing", WorkflowStatusCategory::Started),
        ("published", WorkflowStatusCategory::Done),
    ]
    .into_iter()
    .map(|(key, category)| WorkflowStatus {
        key: key.into(),
        label: key.into(),
        category,
        enabled: true,
    })
    .collect();
    let workflow = StepWorkflow {
        id: "content".into(),
        label: "Content".into(),
        initial_status_key: "draft".into(),
        steps: vec![
            WorkflowStep { status_key: "draft".into(), handoff: WorkflowStepHandoff::KeepAssignee },
            WorkflowStep {
                status_key: "editing".into(),
                handoff: WorkflowStepHandoff::Agent {
                    agent_id: "editor".into(),
                    instructions: "Review the draft".into(),
                    completion_status_key: None,
                    return_status_key: Some("draft".into()),
                },
            },
            WorkflowStep {
                status_key: "published".into(),
                handoff: WorkflowStepHandoff::KeepAssignee,
            },
        ],
    };
    (workflow, statuses)
}

#[test]
fn workflow_steps_round_trip_with_a_default_completion_target() {
    let (workflow, statuses) = content_workflow();
    let payload = serde_json::to_value(&workflow).unwrap();
    let restored: StepWorkflow = serde_json::from_value(payload).unwrap();
    assert_eq!(restored, workflow);
    restored.validate(&statuses).unwrap();
    assert_eq!(restored.completion_status_key("editing"), Some("published"));
    assert_eq!(restored.completion_status_key("draft"), None);
}

#[test]
fn workflow_steps_reject_self_references_and_terminal_handoffs() {
    let (mut workflow, statuses) = content_workflow();
    let WorkflowStepHandoff::Agent { return_status_key, .. } = &mut workflow.steps[1].handoff
    else {
        panic!("fixture must include an agent step");
    };
    *return_status_key = Some("editing".into());
    assert!(workflow.validate(&statuses).unwrap_err().contains("invalid target: editing"));

    let (mut workflow, statuses) = content_workflow();
    workflow.steps[2].handoff = WorkflowStepHandoff::Member { member_id: "alice".into() };
    assert!(workflow.validate(&statuses).unwrap_err().contains("terminal workflow status"));
}

#[test]
fn workflow_steps_require_the_new_configuration_shape() {
    let legacy_workflow = serde_json::json!({
        "id": "content",
        "label": "Content",
        "statuses": [{"key": "draft", "label": "Draft", "role": "backlog"}],
    });
    assert!(serde_json::from_value::<StepWorkflow>(legacy_workflow).is_err());
}
