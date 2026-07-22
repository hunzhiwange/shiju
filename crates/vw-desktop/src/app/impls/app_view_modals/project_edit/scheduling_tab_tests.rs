use super::*;
use iced::advanced::widget::Tree;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

#[test]
fn workflow_option_displays_label_instead_of_stable_id() {
    let option = WorkflowOption {
        id: "stable-workflow-id".to_string(),
        label: "Readable workflow label".to_string(),
    };

    assert_eq!(option.to_string(), "Readable workflow label");
}

#[test]
fn workflow_cache_hint_reports_lifecycle_loading() {
    let mut app = test_app();
    let mut registry = crate::app::task::workflow::TaskWorkflowRegistry::new();
    crate::app::views::workflow_manager::update(
        &mut app.workflow_manager,
        &mut registry,
        crate::app::views::workflow_manager::WorkflowManagerMessage::RefreshLifecycles,
    );

    assert_eq!(workflow_cache_hint(&app).as_deref(), Some("生命周期列表正在从网关加载。"));
}

#[test]
fn workflow_cache_hint_reports_lifecycle_load_error() {
    let mut app = test_app();
    let mut registry = crate::app::task::workflow::TaskWorkflowRegistry::new();
    crate::app::views::workflow_manager::update(
        &mut app.workflow_manager,
        &mut registry,
        crate::app::views::workflow_manager::WorkflowManagerMessage::LifecycleWorkflowsLoaded {
            result: Err("gateway unavailable".to_string()),
            notify_error: false,
        },
    );

    assert_eq!(
        workflow_cache_hint(&app).as_deref(),
        Some("生命周期列表加载失败：gateway unavailable")
    );
}

#[test]
fn workflow_cache_hint_reports_local_cache_before_initial_load() {
    let app = test_app();

    assert_eq!(
        workflow_cache_hint(&app).as_deref(),
        Some("生命周期列表尚未从网关加载，当前显示本地缓存。")
    );
}

#[test]
fn workflow_cache_hint_is_absent_after_lifecycles_are_loaded() {
    let mut app = test_app();
    let registry = crate::app::task::workflow::TaskWorkflowRegistry::new();
    app.workflow_manager =
        crate::app::views::workflow_manager::WorkflowManagerState::from_registry(&registry);

    assert_eq!(workflow_cache_hint(&app), None);
}

#[test]
fn labels_distinguish_wake_and_pool_promote_intervals_in_project_edit_scheduling_tab() {
    let source = include_str!("scheduling_tab.rs");

    assert!(source.contains("执行唤醒间隔(秒)"));
    assert!(source.contains("任务池回推间隔(秒)"));
    assert!(!source.contains("任务调度间隔"));
    assert!(!source.contains("任务池调度间隔"));
    assert!(!source.contains("自动执行任务池间隔"));
}

#[test]
fn scheduling_tab_builds_with_default_settings() {
    let app = test_app();

    let element = scheduling_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn scheduling_tab_clamps_low_values_for_stepper_controls() {
    let mut app = test_app();
    app.project_edit_task_board_settings.max_concurrent = 0;
    app.project_edit_task_board_settings.scheduler_tick_interval_seconds = 0;
    app.project_edit_task_board_settings.auto_promote_tick_interval_seconds = 0;
    app.project_edit_task_board_settings.failed_retry_minutes = 0;
    app.project_edit_task_board_settings.failed_retry_max_attempts = 0;
    app.project_edit_task_board_settings.running_timeout_minutes = 0;
    app.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds = 0;
    app.project_edit_task_board_settings.recycle_worktree_on_task_finish = true;

    let element = scheduling_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn scheduling_tab_clamps_high_values_for_stepper_controls() {
    let mut app = test_app();
    app.project_edit_task_board_settings.max_concurrent = 99;
    app.project_edit_task_board_settings.scheduler_tick_interval_seconds = 99;
    app.project_edit_task_board_settings.auto_promote_tick_interval_seconds = 9000;
    app.project_edit_task_board_settings.failed_retry_minutes = 9999;
    app.project_edit_task_board_settings.failed_retry_max_attempts = 99;
    app.project_edit_task_board_settings.running_timeout_minutes = 9999;
    app.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds = 9999;

    let element = scheduling_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn scheduling_tab_falls_back_when_configured_workflow_is_unknown() {
    let mut app = test_app();
    app.project_edit_task_board_settings.workflow_id = "missing-workflow".to_string();

    let element = scheduling_tab(&app);

    std::hint::black_box(element);
}

#[test]
fn scheduling_tab_builds_without_cache_hint_after_lifecycles_loaded() {
    let mut app = test_app();
    let registry = crate::app::task::workflow::TaskWorkflowRegistry::new();
    app.workflow_manager =
        crate::app::views::workflow_manager::WorkflowManagerState::from_registry(&registry);
    app.project_edit_task_board_settings.workflow_id =
        crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string();

    assert_eq!(workflow_cache_hint(&app), None);
    std::hint::black_box(scheduling_tab(&app));
}

#[test]
fn scheduling_tab_keeps_summary_and_midrange_controls_in_separate_sections() {
    let mut app = test_app();
    app.project_edit_task_board_settings.max_concurrent = 5;
    app.project_edit_task_board_settings.scheduler_tick_interval_seconds = 30;
    app.project_edit_task_board_settings.auto_promote_tick_interval_seconds = 300;
    app.project_edit_task_board_settings.failed_retry_minutes = 60;
    app.project_edit_task_board_settings.failed_retry_max_attempts = 5;
    app.project_edit_task_board_settings.running_timeout_minutes = 120;
    app.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds = 300;

    let element = scheduling_tab(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 2);
}
