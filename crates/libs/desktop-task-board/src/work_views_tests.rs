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
    assert!(!BuiltinWorkView::MyTasks.matches(&task, &workflow, &[], false));
    assert!(BuiltinWorkView::MyTasks.matches(&task, &workflow, &personal, false));
    assert!(BuiltinWorkView::AllTasks.matches(&task, &workflow, &[], false));
    assert!(!BuiltinWorkView::Subscribed.matches(&task, &workflow, &personal, false));
    assert!(BuiltinWorkView::Subscribed.matches(&task, &workflow, &[], true));
    for deleted in [false, true] {
        task.deleted = deleted;
        task.archived = !deleted;
        for view in [
            BuiltinWorkView::MyTasks,
            BuiltinWorkView::AgentExecuting,
            BuiltinWorkView::Subscribed,
            BuiltinWorkView::AllTasks,
        ] {
            assert!(!view.matches(&task, &workflow, &personal, true));
        }
        assert!(!WorkViewFilter::default().matches(&task, &workflow, WorkFilterContext::default()));
    }
}

#[test]
fn work_views_execution_uses_current_workflow_role() {
    let mut workflow = workflow();
    let mut task = task();
    for definition in workflow.statuses.clone() {
        task.status = definition.status;
        assert_eq!(
            BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[], false),
            definition.role == StatusRole::Executing
        );
    }
    let definition = &mut workflow.statuses[0];
    task.status = definition.status;
    definition.role = StatusRole::Executing;
    assert!(BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[], false));
    workflow.statuses.clear();
    assert!(!BuiltinWorkView::AgentExecuting.matches(&task, &workflow, &[], false));
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
        ..Default::default()
    };
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.priority = 21;
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.priority = 9;
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.priority = 10;
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.agent = Some("other".into());
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.acp_agent = Some("codex".into());
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
}

#[test]
fn work_view_status_exclusion_intersects_agent_activity() {
    let workflow = workflow();
    let mut task = task();
    let executing = workflow.statuses.iter().find(|status| status.role == StatusRole::Executing)
        .expect("default workflow has an executing status");
    let idle = workflow.statuses.iter().find(|status| status.role != StatusRole::Executing)
        .expect("default workflow has a non-executing status");
    let mut filter = WorkViewFilter {
        status_keys: vec![idle.key().to_owned()],
        status_excluded: true,
        agent_running: Some(true),
        ..Default::default()
    };
    task.status = executing.status;
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.status = idle.status;
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));

    filter.agent_running = Some(false);
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    filter.remove(&WorkFilterField::Status);
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
}

#[test]
fn work_views_unknown_keys_and_executor_kind_do_not_broaden_results() {
    let mut workflow = workflow();
    let mut task = task();
    let mut filter =
        WorkViewFilter { status_keys: vec!["removed-status".into()], ..Default::default() };
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    filter.status_keys = vec![workflow.status_definition(task.status).unwrap().key.clone()];
    workflow.statuses.retain(|definition| definition.status != task.status);
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    assert_eq!(filter.status_keys.len(), 1);
    filter.status_keys.clear();
    filter.agents = vec![ExecutionAgent::Acp { id: "main".into() }];
    task.assignee = "main".into();
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    task.acp_agent = Some("main".into());
    assert!(filter.matches(&task, &workflow, WorkFilterContext::default()));
    filter.agents = vec![ExecutionAgent::Builtin { id: "main".into() }];
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
    filter.agents = vec![ExecutionAgent::Acp { id: "unknown".into() }];
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
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
    assert!(!view.matches(&task(), &workflow, WorkFilterContext::default()));
    assert!(serde_json::from_str::<ExecutionAgent>(r#"{"kind":"unknown","id":"main"}"#).is_err());
}

#[test]
fn work_views_match_scoped_filters_and_invert_relation_selection() {
    let workflow = workflow();
    let task = Task { assignee: String::new(), created_by: Some("creator".into()), ..task() };
    let mut filter = WorkViewFilter::default();
    for (field, values) in [
        (WorkScopeField::Assignee, &[""][..]),
        (WorkScopeField::Creator, &["creator"][..]),
        (WorkScopeField::Project, &["project-a"][..]),
        (WorkScopeField::Relation, &["created_by_me", "subscribed"][..]),
    ] {
        filter.scopes.insert(field, WorkScopeSelection {
            values: values.iter().map(|value| (*value).to_owned()).collect(),
            excluded: false,
        });
    }
    let context = WorkFilterContext {
        project_id: Some("project-a"),
        viewer_id: Some("creator"),
        ..Default::default()
    };
    assert!(filter.matches(&task, &workflow, context));
    assert!(!filter.matches(&task, &workflow, WorkFilterContext {
        project_id: Some("project-b"),
        ..context
    }));
    filter.scopes.get_mut(&WorkScopeField::Relation).unwrap().excluded = true;
    assert!(!filter.matches(&task, &workflow, context));
    assert!(filter.matches(&task, &workflow, WorkFilterContext {
        viewer_id: Some("another-user"),
        ..context
    }));
}

#[test]
fn work_views_match_unset_or_selected_property_and_exclude_labels() {
    use vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue::SingleSelect;

    let workflow = workflow();
    let mut task = task();
    let filter = WorkViewFilter {
        properties: [("region".into(), SingleSelect("east".into()))].into(),
        unset_properties: ["region".into()].into(),
        label_ids: vec!["hidden".into()],
        labels_excluded: true,
        ..Default::default()
    };
    let context = WorkFilterContext::default();
    assert!(filter.matches(&task, &workflow, context));
    task.properties.insert("region".into(), SingleSelect("west".into()));
    assert!(!filter.matches(&task, &workflow, context));
    task.properties.insert("region".into(), SingleSelect("east".into()));
    assert!(filter.matches(&task, &workflow, context));
    task.label_ids.push("hidden".into());
    assert!(!filter.matches(&task, &workflow, context));
}

#[test]
fn work_views_combine_saved_advanced_groups_with_simple_filters() {
    let condition = |field, excluded, rule| {
        WorkFilterNode::Condition(WorkFilterCondition { field, excluded, rule })
    };
    let mut group = WorkFilterGroup {
        join: WorkFilterJoin::Any,
        children: vec![
            condition(WorkFilterField::Status, false, WorkViewFilter::default()),
            condition(WorkFilterField::Priority, false, WorkViewFilter {
                priority_min: Some(500),
                ..Default::default()
            }),
            WorkFilterNode::Group(WorkFilterGroup {
                join: WorkFilterJoin::All,
                children: vec![
                    condition(WorkFilterField::Label, false, WorkViewFilter {
                        label_ids: vec!["approved".into()],
                        ..Default::default()
                    }),
                    condition(WorkFilterField::Label, true, WorkViewFilter {
                        label_ids: vec!["blocked".into()],
                        ..Default::default()
                    }),
                ],
            }),
        ],
    };
    assert_eq!(group.condition_count(), 3);
    group.trim_empty();
    let filter = WorkViewFilter { unread_only: true, advanced: Some(group), ..Default::default() };
    let saved = serde_json::to_string(&filter).unwrap();
    assert_eq!(serde_json::from_str::<WorkViewFilter>(&saved).unwrap(), filter);

    let workflow = workflow();
    let unread = WorkFilterContext { unread: true, ..Default::default() };
    let mut task = task();
    assert!(!filter.matches(&task, &workflow, unread));
    task.label_ids.push("approved".into());
    assert!(filter.matches(&task, &workflow, unread));
    task.label_ids.push("blocked".into());
    assert!(!filter.matches(&task, &workflow, unread));
    task.priority = 600;
    assert!(filter.matches(&task, &workflow, unread));
    assert!(!filter.matches(&task, &workflow, WorkFilterContext::default()));
}
