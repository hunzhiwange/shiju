//! 内置预设只有进度分类与显式步骤，无自动派发或结果迁移。
use super::*;

#[test]
fn built_in_workflows_expose_stable_unique_catalog() {
    let workflows = built_in_workflows();
    assert_eq!(
        workflows.iter().map(|workflow| workflow.id.as_str()).collect::<Vec<_>>(),
        vec![
            "generic_task",
            "dev_lifecycle",
            "content_publishing",
            "marketing_campaign",
            "customer_support",
            "recruiting_pipeline"
        ]
    );
    for workflow in workflows {
        workflow.validate().unwrap();
        assert!(workflow.built_in);
        assert_eq!(built_in_workflow_by_id(&workflow.id), Some(workflow.clone()));
        assert!(is_built_in_workflow_id(&workflow.id));
        assert_eq!(workflow.initial_status_key, workflow.steps[0].status_key);
        assert_eq!(workflow.statuses.len(), workflow.steps.len());
        assert!(
            workflow
                .steps
                .iter()
                .all(|step| step.handoff == TaskLifecycleStepHandoffDto::KeepAssignee)
        );
        assert!(workflow.statuses.iter().any(|status| status.category == Category::Done));
        assert!(workflow.statuses.iter().any(|status| status.category == Category::Closed));
    }
    assert!(built_in_workflow_by_id("unknown").is_none());
    assert!(!is_built_in_workflow_id("unknown"));
}

#[test]
fn built_in_categories_are_independent_of_scheduler_runtime() {
    let workflow = built_in_workflow_by_id("dev_lifecycle").unwrap();
    let category =
        |key| workflow.statuses.iter().find(|status| status.key == key).unwrap().category;
    assert_eq!(category("pool"), Category::Unstarted);
    assert_eq!(category("running"), Category::Started);
    assert_eq!(category("paused"), Category::Started);
    assert_eq!(category("completed"), Category::Done);
    assert_eq!(category("archived"), Category::Closed);
}
