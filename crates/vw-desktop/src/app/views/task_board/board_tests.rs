//! 任务看板主体的测试，覆盖列生成、卡片排序、状态标签与拖拽绘制。
//! fixture 包含自定义工作流和归档任务，用于验证筛选及未知配置回退。
use crate::app::task::{
    OwnedStatusDefinition, OwnedTaskWorkflow, StatusRole, SubTask, Task, TaskStatus,
    register_runtime_task_workflow,
};
use crate::app::{App, Message};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Element, Font, Pixels, Rectangle, Size, Theme};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("board_tests"));
}

#[test]
fn duration_and_running_dots_format_boundary_values() {
    assert_eq!(super::format_duration_ms(0), "0s");
    assert_eq!(super::format_duration_ms(1_250), vw_shared::util::format_duration(1));

    assert_eq!(super::running_dots(0), "·");
    assert_eq!(super::running_dots(1_000), "··");
    assert_eq!(super::running_dots(2_000), "···");
    assert_eq!(super::running_dots(3_999), "·");
}

#[test]
fn task_card_preview_handles_blank_compaction_and_unicode_truncation() {
    let mut blank = task("blank", TaskStatus::Pool, 999);
    blank.prompt = " \n\t ".to_string();
    assert_eq!(super::task_card_preview(&blank), "（无内容）");

    let mut compact = task("compact", TaskStatus::Pool, 999);
    compact.prompt = " first\r\n\n second\tthird ".to_string();
    assert_eq!(super::task_card_preview(&compact), "first second third");

    let mut long = task("long", TaskStatus::Pool, 999);
    long.prompt = "界".repeat(super::CARD_CONTENT_PREVIEW_MAX_CHARS + 3);
    let preview = super::task_card_preview(&long);
    assert!(preview.ends_with('…'));
    assert_eq!(preview.chars().count(), super::CARD_CONTENT_PREVIEW_MAX_CHARS + 1);
}

#[test]
fn task_status_tag_colors_cover_builtin_and_custom_statuses() {
    let cases = [
        (TaskStatus::Pool, rgb(107, 114, 128), rgb(243, 244, 246)),
        (TaskStatus::Pending, rgb(37, 99, 235), rgb(219, 234, 254)),
        (TaskStatus::Planning, rgb(79, 70, 229), rgb(224, 231, 255)),
        (TaskStatus::Running, rgb(147, 51, 234), rgb(243, 232, 255)),
        (TaskStatus::Failed, rgb(220, 38, 38), rgb(254, 226, 226)),
        (TaskStatus::Paused, rgb(202, 138, 4), rgb(254, 249, 195)),
        (TaskStatus::CodeComplete, rgb(5, 150, 105), rgb(209, 250, 229)),
        (TaskStatus::CodeReview, rgb(217, 119, 6), rgb(254, 243, 199)),
        (TaskStatus::PrSubmitted, rgb(8, 145, 178), rgb(207, 250, 254)),
        (TaskStatus::Completed, rgb(22, 163, 74), rgb(220, 252, 231)),
        (TaskStatus::Archived, rgb(100, 116, 139), rgb(241, 245, 249)),
        (TaskStatus::Custom("waiting"), rgb(107, 114, 128), rgb(243, 244, 246)),
    ];

    for (status, expected_text, expected_background) in cases {
        let (text, background) = super::task_status_tag_colors(status);
        assert_color_eq(text, expected_text);
        assert_color_eq(background, expected_background);
    }
}

#[test]
fn bulk_actions_exclude_only_archived_status() {
    for status in TaskStatus::all() {
        assert_eq!(super::bulk_actions_supported(status), status != TaskStatus::Archived);
    }
    assert!(super::bulk_actions_supported(TaskStatus::Custom("queued")));
}

#[test]
fn kanban_columns_follow_selected_workflow() {
    let _cache_lock = crate::app::task::runtime_task_lifecycle_cache_test_lock();
    let custom_status = custom_status("kanban_waiting_review");
    let workflow = OwnedTaskWorkflow {
        id: "board_custom_workflow".to_string(),
        label: "Board Custom Workflow".to_string(),
        statuses: vec![
            owned_status("pool", "Inbox", StatusRole::Backlog, Some(custom_status)),
            owned_status(
                "kanban_waiting_review",
                "Waiting Review",
                StatusRole::Review,
                Some(TaskStatus::Completed),
            ),
            owned_status("completed", "Shipped", StatusRole::Done, None),
            owned_status("archived", "Cold Storage", StatusRole::Archived, None),
        ],
    };
    let _home = TempHome::new();
    register_runtime_task_workflow(workflow).expect("custom workflow should register");

    let mut app = app();
    app.task_board_settings.workflow_id = "board_custom_workflow".to_string();

    let columns = super::kanban_columns(&app);

    assert_eq!(
        columns.iter().map(|column| column.status).collect::<Vec<_>>(),
        vec![TaskStatus::Pool, custom_status, TaskStatus::Completed]
    );
    assert_eq!(
        columns.iter().map(|column| column.label.as_str()).collect::<Vec<_>>(),
        vec!["Inbox", "Waiting Review", "Shipped"]
    );
}

#[test]
fn kanban_falls_back_to_default_on_unknown_workflow() {
    let mut app = app();
    app.task_board_settings.workflow_id = "missing_workflow".to_string();

    let columns = super::kanban_columns(&app);
    let default_statuses =
        TaskStatus::all().into_iter().filter(|status| *status != TaskStatus::Archived);

    assert_eq!(
        columns.iter().map(|column| column.status).collect::<Vec<_>>(),
        default_statuses.collect::<Vec<_>>()
    );
    assert_eq!(columns[0].label, TaskStatus::Pool.label());
}

#[test]
fn task_board_column_scrollbar_reserves_requested_gutter() {
    assert_eq!(super::TASK_BOARD_SCROLLBAR_WIDTH, 4.0);
    assert_eq!(super::TASK_BOARD_SCROLLBAR_GUTTER, 7.0);
    assert!(super::TASK_BOARD_SCROLLBAR_GUTTER >= super::TASK_BOARD_SCROLLBAR_WIDTH);
}

#[test]
fn task_cards_sort_by_created_desc_without_updated_at() {
    let mut old_updated_newer = Task::new(30);
    old_updated_newer.id = "old-updated-newer".to_string();
    old_updated_newer.created_at_ms = 100;
    old_updated_newer.updated_at_ms = 900;

    let mut newest = Task::new(10);
    newest.id = "newest".to_string();
    newest.created_at_ms = 300;
    newest.updated_at_ms = 100;

    let mut middle = Task::new(999);
    middle.id = "middle".to_string();
    middle.created_at_ms = 200;
    middle.updated_at_ms = 200;

    let mut tasks = vec![&old_updated_newer, &newest, &middle];

    super::sort_task_cards(&mut tasks);

    assert_eq!(
        tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["newest", "middle", "old-updated-newer"]
    );
}

#[test]
fn task_cards_sort_by_priority_order_and_id_when_created_matches() {
    let mut order_two = task("order-two", TaskStatus::Pool, 100);
    order_two.created_at_ms = 500;
    order_two.order = 2;

    let mut high_priority = task("high-priority", TaskStatus::Pool, 200);
    high_priority.created_at_ms = 500;
    high_priority.order = 1;

    let mut id_b = task("id-b", TaskStatus::Pool, 100);
    id_b.created_at_ms = 500;
    id_b.order = 1;

    let mut id_a = task("id-a", TaskStatus::Pool, 100);
    id_a.created_at_ms = 500;
    id_a.order = 1;

    let mut tasks = vec![&order_two, &high_priority, &id_b, &id_a];

    super::sort_task_cards(&mut tasks);

    assert_eq!(
        tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
        vec!["high-priority", "id-a", "id-b", "order-two"]
    );
}

#[test]
fn kanban_board_builds_columns_and_filters_archived_tasks() {
    let mut app = app();
    let mut visible = task("visible", TaskStatus::Pending, 50);
    visible.prompt = "Visible task".to_string();
    let mut deleted = task("deleted", TaskStatus::Pending, 10);
    deleted.deleted = true;
    let mut archived = task("archived", TaskStatus::Pending, 5);
    archived.archived = true;
    app.task_board_tasks = vec![archived, deleted, visible];

    keep_element(super::build_kanban_board(&app));
    keep_element(super::build_status_column(&app, TaskStatus::Pending, 1_000));
    keep_element(super::build_status_column(&app, TaskStatus::Archived, 1_000));
}

#[tokio::test(flavor = "multi_thread")]
async fn kanban_board_draws_dragging_card_and_column_styles() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = app();
    let mut running = task("dragging", TaskStatus::Running, 80);
    running.model = "auto".to_string();
    running.execution_started_at_ms = Some(1_000);
    app.task_board_tasks = vec![running];
    app.task_board_dragging = Some(("dragging".to_string(), TaskStatus::Running));
    let mut element = super::build_kanban_board(&app);

    draw_element(&mut element, &mut renderer, &Theme::Dark);
}

#[test]
fn status_column_builds_empty_and_selected_bulk_modes() {
    let mut app = app();
    app.task_board_bulk_active_status = Some(TaskStatus::Planning);
    keep_element(super::build_status_column(&app, TaskStatus::Planning, 1_000));

    let selected = task("selected-planning", TaskStatus::Planning, 20);
    app.task_board_selected_tasks.insert(selected.id.clone());
    app.task_board_tasks = vec![selected];
    app.task_board_bulk_priority_input = "42".to_string();
    app.task_board_bulk_model_input = "openai/gpt-4.1".to_string();
    app.task_board_bulk_agent = "agent-specialist".to_string();
    app.task_board_bulk_acp_agent = Some("codex".to_string());

    keep_element(super::build_status_column(&app, TaskStatus::Planning, 1_000));
}

#[test]
fn status_column_builds_pr_submitted_bulk_merge_and_drag_targets() {
    let mut app = app();
    let selected = task("selected-pr", TaskStatus::PrSubmitted, 20);
    app.task_board_selected_tasks.insert(selected.id.clone());
    app.task_board_tasks = vec![selected];
    app.task_board_bulk_active_status = Some(TaskStatus::PrSubmitted);
    app.task_board_dragging = Some(("selected-pr".to_string(), TaskStatus::PrSubmitted));
    app.task_board_column_has_vertical_scrollbar.insert(TaskStatus::PrSubmitted, true);

    keep_element(super::build_status_column(&app, TaskStatus::PrSubmitted, 2_000));
}

#[test]
fn drop_slot_builds_idle_and_active_release_targets() {
    keep_element(super::build_drop_slot(TaskStatus::Pool, 0, false));
    keep_element(super::build_drop_slot(TaskStatus::Pool, 3, true));
}

#[test]
fn task_card_builds_running_failed_paused_and_context_menu_variants() {
    let mut app = app();
    app.task_board_bulk_active_status = Some(TaskStatus::Running);
    app.task_board_selected_tasks.insert("running".to_string());

    let mut running = task("running", TaskStatus::Running, 80);
    running.prompt = "Run with auto model".to_string();
    running.model = "auto".to_string();
    running.execution_started_at_ms = Some(1_000);
    running.retry_count = 2;
    running.subtasks = vec![completed_subtask("done"), SubTask::new("todo".to_string())];
    keep_element(super::build_task_card(
        &app,
        &running,
        TaskStatus::Running,
        false,
        false,
        iced::Point::new(10.0, 20.0),
        None,
        4_000,
    ));

    let mut failed = task("failed", TaskStatus::Failed, 501);
    failed.prompt = "Failure card".to_string();
    failed.model = "openai/gpt-4.1".to_string();
    failed.last_error = Some("boom".to_string());
    failed.last_execution_duration_ms = Some(65_000);
    keep_element(super::build_task_card(
        &app,
        &failed,
        TaskStatus::Failed,
        true,
        true,
        iced::Point::new(1.0, 2.0),
        Some(("failed".to_string(), 30.0, 40.0)),
        9_000,
    ));

    let mut paused = task("paused", TaskStatus::Paused, 100);
    paused.model = "local-model".to_string();
    paused.pause_reason = Some("manual review".to_string());
    keep_element(super::build_task_card(
        &app,
        &paused,
        TaskStatus::Paused,
        false,
        false,
        iced::Point::new(3.0, 4.0),
        Some(("other".to_string(), 0.0, 0.0)),
        9_000,
    ));
}

#[test]
fn task_card_builds_completed_code_complete_and_plain_priority_variants() {
    let app = app();

    let mut completed = task("completed", TaskStatus::Completed, 999);
    completed.model = "anthropic/claude".to_string();
    completed.last_execution_duration_ms = Some(1_500);
    keep_element(super::build_task_card(
        &app,
        &completed,
        TaskStatus::Completed,
        false,
        false,
        iced::Point::new(0.0, 0.0),
        None,
        3_000,
    ));

    let mut code_complete = task("code-complete", TaskStatus::CodeComplete, 500);
    code_complete.model = "standalone-model".to_string();
    code_complete.subtasks.clear();
    keep_element(super::build_task_card(
        &app,
        &code_complete,
        TaskStatus::CodeComplete,
        false,
        true,
        iced::Point::new(5.0, 6.0),
        Some(("different".to_string(), 1.0, 1.0)),
        3_000,
    ));
}

fn app() -> App {
    App::new().0
}

fn task(id: &str, status: TaskStatus, priority: u32) -> Task {
    let mut task = Task::new(priority);
    task.id = id.to_string();
    task.status = status;
    task.created_at_ms = 100;
    task.updated_at_ms = 100;
    task.last_active_at_ms = 100;
    task.prompt = format!("Prompt for {id}");
    task
}

fn completed_subtask(content: &str) -> SubTask {
    let mut subtask = SubTask::new(content.to_string());
    subtask.completed = true;
    subtask
}

fn custom_status(key: &'static str) -> TaskStatus {
    TaskStatus::parse_key(key).expect("custom status key should parse")
}

fn owned_status(
    key: &'static str,
    label: &'static str,
    role: StatusRole,
    next: Option<TaskStatus>,
) -> OwnedStatusDefinition {
    OwnedStatusDefinition {
        status: TaskStatus::parse_key(key).expect("status key should parse"),
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        is_terminal: next.is_none(),
        auto_execution_candidate: !matches!(role, StatusRole::Done | StatusRole::Archived),
        occupies_execution_slot: matches!(role, StatusRole::Executing | StatusRole::Planning),
        is_scheduling_backlog: matches!(
            role,
            StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning
        ),
        next,
        transitions: Vec::new(),
    }
}

struct TempHome {
    _dir: tempfile::TempDir,
    previous_home: Option<std::ffi::OsString>,
}

impl TempHome {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp home should be created");
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", dir.path()) };
        Self { _dir: dir, previous_home }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

fn keep_element(element: Element<'_, Message>) {
    let _ = std::hint::black_box(element);
}

fn draw_element(element: &mut Element<'_, Message>, renderer: &mut iced::Renderer, theme: &Theme) {
    let bounds = Size::new(1_200.0, 800.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(bounds);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}

fn rgb(r: u8, g: u8, b: u8) -> iced::Color {
    iced::Color::from_rgb8(r, g, b)
}

fn assert_color_eq(actual: iced::Color, expected: iced::Color) {
    assert!((actual.r - expected.r).abs() < f32::EPSILON);
    assert!((actual.g - expected.g).abs() < f32::EPSILON);
    assert!((actual.b - expected.b).abs() < f32::EPSILON);
    assert!((actual.a - expected.a).abs() < f32::EPSILON);
}
