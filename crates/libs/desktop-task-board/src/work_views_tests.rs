use super::work_views::*;
use desktop_task_model::{StatusRole, Task, TaskWorkflow, default_task_workflow};

fn workflow() -> TaskWorkflow {
    TaskWorkflow::from(default_task_workflow())
}

fn task() -> Task {
    Task { id: "task-1".into(), priority: 20, ..Task::default() }
}

#[test]
fn work_views_builtin_personal_is_explicit_and_all_excludes_hidden() {
    let workflow = workflow();
    let mut task = task();
    let personal = vec![task.id.clone()];
    assert!(!BuiltinWorkView::MyTasks.matches(&task, &workflow, &[]));
    assert!(BuiltinWorkView::MyTasks.matches(&task, &workflow, &personal));
    assert!(BuiltinWorkView::AllTasks.matches(&task, &workflow, &[]));
    for deleted in [false, true] {
        task.deleted = deleted;
        task.archived = !deleted;
        for view in
            [BuiltinWorkView::MyTasks, BuiltinWorkView::AgentExecuting, BuiltinWorkView::AllTasks]
        {
            assert!(!view.matches(&task, &workflow, &personal));
        }
        assert!(!WorkViewFilter::default().matches(&task, &workflow));
    }
}

#[test]
fn work_views_execution_uses_current_workflow_role() {
    let mut workflow = workflow();
    let mut task = task();
    for definition in workflow.statuses.clone() {
        task.status = definition.status;
        assert_eq!(
            BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[]),
            definition.role == StatusRole::Executing
        );
    }
    let definition = &mut workflow.statuses[0];
    task.status = definition.status;
    definition.role = StatusRole::Executing;
    assert!(BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[]));
    workflow.statuses.clear();
    assert!(!BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[]));
}

#[test]
fn work_views_compound_filter_unions_options_and_intersects_fields() {
    let workflow = workflow();
    let mut task = task();
    let filter = WorkViewFilter {
        status_keys: vec![
            "missing".into(),
            workflow.status_definition(task.status).unwrap().key.clone(),
        ],
        priority_min: Some(10),
        priority_max: Some(20),
        agents: vec![
            ExecutionAgent::Acp { id: "codex".into() },
            ExecutionAgent::Builtin { id: "main".into() },
        ],
        ..Default::default(),
    };
    assert!(filter.matches(&task, &workflow));
    task.priority = 21;
    assert!(!filter.matches(&task, &workflow));
    task.priority = 9;
    assert!(!filter.matches(&task, &workflow));
    task.priority = 10;
    assert!(filter.matches(&task, &workflow));
    task.agent = Some("other".into());
    assert!(!filter.matches(&task, &workflow));
    task.acp_agent = Some("codex".into());
    assert!(filter.matches(&task, &workflow));
}

#[test]
fn work_views_unknown_keys_and_executor_kind_do_not_broaden_results() {
    let mut workflow = workflow();
    let mut task = task();
    let mut filter =
        WorkViewFilter { status_keys: vec!["removed-status".into()], ..Default::default() };
    assert!(!filter.matches(&task, &workflow));
    filter.status_keys = vec![workflow.status_definition(task.status).unwrap().key.clone()];
    workflow.statuses.retain(|definition| definition.status != task.status);
    assert!(!filter.matches(&task, &workflow));
    assert_eq!(filter.status_keys.len(), 1);
    filter.status_keys.clear();
    filter.agents = vec![ExecutionAgent::Acp { id: "main".into() }];
    task.assignee = "main".into();
    assert!(!filter.matches(&task, &workflow));
    task.acp_agent = Some("main".into());
    assert!(filter.matches(&task, &workflow));
    filter.agents = vec![ExecutionAgent::Builtin { id: "main".into() }];
    assert!(!filter.matches(&task, &workflow));
    filter.agents = vec![ExecutionAgent::Acp { id: "unknown".into() }];
    assert!(!filter.matches(&task, &workflow));
}

#[test]
fn work_views_saved_round_trip_and_invalid_priority_range() {
    let workflow = workflow();
    assert!(serde_json::from_str::<WorkViewFilter>(r#"{"unknown_rule":true}"#).is_err());
    let view = SavedWorkView {
        id: "view-1".into(),
        name: "重点".into(),
        filter: WorkViewFilter {
            priority_min: Some(30),
            priority_max: Some(10),
            ..Default::default()
        },
    };
    let json = serde_json::to_string(&view).unwrap();
    assert_eq!(serde_json::from_str::<SavedWorkView>(&json).unwrap(), view);
    assert!(!view.matches(&task(), &workflow));
    assert!(serde_json::from_str::<ExecutionAgent>(r#"{"kind":"unknown","id":"main"}"#).is_err());
}

#[test]
fn work_plan_detail_projection_groups_scopes_numerically() {
    use desktop_task_model::TaskStatus;
    use vw_api_agent_scheduler::AgentSchedulerTaskPlanPlacementDto::{Child, Root};

    let tasks = [
        Task { id: "second".into(), ..Task::default() },
        Task { id: "ordinary".into(), ..Task::default() },
        Task { id: "tenth".into(), ..Task::default() },
        Task { id: "first".into(), status: TaskStatus::Completed, ..Task::default() },
        Task { id: "hidden".into(), ..Task::default() },
        Task { id: "child-a".into(), ..Task::default() },
        Task { id: "child-b".into(), ..Task::default() },
    ];
    let plan = WorkPlanView {
        tasks: [
            ("second", Root { outer_batch: 2, inner_batch: 1 }),
            ("first", Root { outer_batch: 1, inner_batch: 5 }),
            ("hidden", Root { outer_batch: 1, inner_batch: 5 }),
            ("tenth", Root { outer_batch: 1, inner_batch: 10 }),
            ("child-a", Child { parent_task_id: "first".into(), batch: 1 }),
            ("child-b", Child { parent_task_id: "second".into(), batch: 1 }),
        ].into_iter().map(|(id, placement)| (id.into(), WorkPlanTask {
            placement: Some(placement),
            status: tasks.iter().find(|task| task.id == id).unwrap().status,
            status_label: None,
            blocker: None,
        })).collect(),
        ..WorkPlanView::default()
    };
    let groups = plan.roots(&tasks, &workflow(), |task| task.id != "hidden");
    assert_eq!(groups.keys().copied().collect::<Vec<_>>(), [None, Some((1, 5)), Some((1, 10)), Some((2, 1))]);
    assert_eq!(groups[&None].rows[0].id, "ordinary");
    assert_eq!(groups[&Some((1, 5))].rows.len(), 1);
    assert_eq!(groups[&Some((1, 5))].progress.total, 2);
    assert_eq!(groups[&Some((1, 5))].progress.completed, 1);
    assert_eq!(plan.children(&tasks, &workflow(), "first")[&1].rows[0].id, "child-a");
    assert_eq!(plan.children(&tasks, &workflow(), "second")[&1].rows[0].id, "child-b");
}
