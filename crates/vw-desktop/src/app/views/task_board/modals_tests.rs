//! 任务看板弹窗的测试，覆盖创建、导入、设置及各类确认流程。
//! 草稿与边界值 fixture 用于验证标签区分、分页和弹出层组合状态。
use crate::app::state::TaskBoardSettingsModalTab;
use crate::app::task::{Task, TaskStatus};
use crate::app::{App, Message};
use iced::{Element, Length};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn labels_distinguish_wake_and_pool_promote_intervals_in_task_settings_modal() {
    let source = include_str!("modals.rs");

    assert!(source.contains("刷新间隔(秒)"));
    assert!(source.contains("执行唤醒间隔(秒)"));
    assert!(source.contains("任务池回推间隔(秒)"));
    assert!(source.contains("统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。"));
    assert!(!source.contains("任务调度间隔"));
    assert!(!source.contains("任务池调度间隔"));
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("modals_tests"));
}

#[test]
fn create_task_modal_builds_single_and_import_modes() {
    let mut app = test_app();

    let single = super::build_create_task_modal(&app);
    assert_eq!(single.as_widget().size().width, Length::Fill);
    assert_eq!(single.as_widget().size().height, Length::Fixed(480.0));
    keep_element(single);

    app.task_board_is_import_mode = true;
    let import = super::build_create_task_modal(&app);
    assert_eq!(import.as_widget().size().width, Length::Fill);
    assert_eq!(import.as_widget().size().height, Length::Fixed(480.0));
    keep_element(import);
}

#[test]
fn create_task_modal_builds_with_nondefault_draft_and_open_popovers() {
    let mut app = test_app();
    app.task_board_draft.priority = "42".to_string();
    app.task_board_draft.model = "model-a".to_string();
    app.task_board_draft.acp_agent = Some("agent:worker".to_string());
    app.task_board_model_popover = true;
    app.task_board_executor_popover = true;

    let single = super::build_create_task_modal(&app);

    assert_eq!(single.as_widget().size().height, Length::Fixed(480.0));
    keep_element(single);

    app.task_board_is_import_mode = true;
    app.task_board_import_editor =
        iced::widget::text_editor::Content::with_text("priority,prompt\n7,ship it");
    let import = super::build_create_task_modal(&app);

    assert_eq!(import.as_widget().size().height, Length::Fixed(480.0));
    keep_element(import);
}

#[test]
fn settings_modal_builds_refresh_tab_with_clamped_edges() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
    app.task_board_settings.auto_refresh = false;
    app.task_board_settings.refresh_interval_seconds = 0;
    app.task_board_settings.scheduler_tick_interval_seconds = 0;
    app.task_board_settings.auto_promote_tick_interval_seconds = 0;
    app.task_board_settings.auto_promote_delay_seconds = 0;

    let low = super::build_task_settings_modal(&app);
    assert_eq!(low.as_widget().size().width, Length::Fixed(580.0));
    keep_element(low);

    app.task_board_settings.refresh_interval_seconds = 9_000;
    app.task_board_settings.scheduler_tick_interval_seconds = 99;
    app.task_board_settings.auto_promote_tick_interval_seconds = 9_000;
    app.task_board_settings.auto_promote_delay_seconds = 9_000;

    let high = super::build_task_settings_modal(&app);
    assert_eq!(high.as_widget().size().width, Length::Fixed(580.0));
    keep_element(high);
}

#[test]
fn settings_modal_builds_refresh_and_scheduling_tabs_with_midrange_values() {
    let mut app = test_app();
    app.task_board_settings.max_concurrent = 4;
    app.task_board_settings.refresh_interval_seconds = 30;
    app.task_board_settings.scheduler_tick_interval_seconds = 15;
    app.task_board_settings.auto_promote_tick_interval_seconds = 45;
    app.task_board_settings.auto_promote_delay_seconds = 120;
    app.task_board_settings.failed_retry_minutes = 30;
    app.task_board_settings.failed_retry_max_attempts = 3;
    app.task_board_settings.running_timeout_minutes = 90;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 300;

    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;
    let refresh = super::build_task_settings_modal(&app);
    assert_eq!(refresh.as_widget().size().width, Length::Fixed(580.0));
    keep_element(refresh);

    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Scheduling;
    let scheduling = super::build_task_settings_modal(&app);
    assert_eq!(scheduling.as_widget().size().width, Length::Fixed(580.0));
    keep_element(scheduling);
}

#[test]
fn settings_modal_builds_each_scheduler_dispatch_mode() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;

    for mode in [
        AgentSchedulerDispatchModeDto::Local,
        AgentSchedulerDispatchModeDto::Matrix,
        AgentSchedulerDispatchModeDto::Auto,
    ] {
        app.task_board_settings.agent_scheduler_dispatch_mode = mode;

        let modal = super::build_task_settings_modal(&app);

        assert_eq!(modal.as_widget().size().width, Length::Fixed(580.0));
        keep_element(modal);
    }
}

#[test]
fn settings_modal_builds_scheduling_tab_with_clamped_edges() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Scheduling;
    app.task_board_settings.max_concurrent = 0;
    app.task_board_settings.failed_retry_minutes = 0;
    app.task_board_settings.failed_retry_max_attempts = 0;
    app.task_board_settings.running_timeout_minutes = 0;
    app.task_board_settings.recycle_worktree_on_task_finish = true;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 0;

    let low = super::build_task_settings_modal(&app);
    assert_eq!(low.as_widget().size().width, Length::Fixed(580.0));
    keep_element(low);

    app.task_board_settings.max_concurrent = 99;
    app.task_board_settings.failed_retry_minutes = 9_999;
    app.task_board_settings.failed_retry_max_attempts = 99;
    app.task_board_settings.running_timeout_minutes = 9_999;
    app.task_board_settings.recycle_worktree_on_task_finish = false;
    app.task_board_settings.pr_submitted_stall_timeout_seconds = 9_999;

    let high = super::build_task_settings_modal(&app);
    assert_eq!(high.as_widget().size().width, Length::Fixed(580.0));
    keep_element(high);
}

#[test]
fn task_settings_modal_no_longer_contains_capability_management_tab() {
    let mut app = test_app();
    app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::Refresh;

    let modal = super::build_task_settings_modal(&app);
    assert_eq!(modal.as_widget().size().width, Length::Fixed(580.0));
    keep_element(modal);

    let source = include_str!("modals.rs");
    let update_source = include_str!("../../message/task_board/update.rs");
    assert!(!source.contains("TaskBoardSettingsModalTab::Capabilities"));
    assert!(!source.contains("保存能力目录"));
    assert!(!update_source.contains("TaskBoardSettingsModalTab::Capabilities"));
}

#[test]
fn context_menu_builds_regular_and_failed_task_variants() {
    let mut regular = Task::new(10);
    regular.id = "regular-task".to_string();

    let regular_menu = super::build_context_menu(regular);
    assert_eq!(regular_menu.as_widget().children().len(), 3);
    keep_element(regular_menu);

    let mut failed = Task::new(20);
    failed.id = "failed-task".to_string();
    failed.status = TaskStatus::Failed;

    let failed_menu = super::build_context_menu(failed);
    assert_eq!(failed_menu.as_widget().children().len(), 4);
    keep_element(failed_menu);

    let mut running = Task::new(30);
    running.id = "running-task".to_string();
    running.status = TaskStatus::Running;
    let running_menu = super::build_context_menu(running);
    assert_eq!(running_menu.as_widget().children().len(), 3);
    keep_element(running_menu);
}
