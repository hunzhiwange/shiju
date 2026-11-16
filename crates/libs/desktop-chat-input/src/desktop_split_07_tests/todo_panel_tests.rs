use crate::input_panel::todo_panel::{
    Input, Message, TodoPanelSurface, compute_todo_data, todo_panel,
};
use todo_types::Todo;
fn test_input() -> Input<'static> {
    Input {
        i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        active_query: None,
        expanded: false,
        top_right: false,
        changed_files: 0,
    }
}
fn todo(id: &str, status: &str, content: &str) -> Todo {
    Todo {
        id: id.into(),
        status: status.into(),
        content: content.into(),
        priority: "medium".into(),
    }
}
fn keep(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn task_740_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("todo_panel_tests.rs"));
}

#[test]
fn compute_todo_data_counts_completed_and_prefers_running_todo() {
    let app = test_input();
    let items = vec![
        todo("2", "completed", "done"),
        todo("1", "in_progress", "running task"),
        todo("3", "pending", "later"),
    ];

    let data = compute_todo_data(app, &items);

    assert_eq!(data.total, 3);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "running task");
}

#[test]
fn compute_todo_data_uses_default_text_without_running_work() {
    let app = test_input();
    let items = vec![todo("1", "pending", "later"), todo("2", "completed", "done")];

    let data = compute_todo_data(app, &items);

    assert_eq!(data.total, 2);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_uses_trimmed_first_line_of_active_request() {
    let mut app = test_input();

    app.active_query = Some("  active request  \nignored second line");

    let data = compute_todo_data(app, &[]);

    assert_eq!(data.running_task, "active request");
}

#[test]
fn compute_todo_data_rejects_blank_active_request_first_line() {
    let mut app = test_input();

    app.active_query = Some("   \nnonempty second line");

    let data = compute_todo_data(app, &[]);

    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_does_not_replace_blank_running_todo_with_active_request() {
    let mut app = test_input();

    app.active_query = Some("active request");
    let items = vec![todo("1", "in_progress", "")];

    let data = compute_todo_data(app, &items);

    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_uses_first_running_todo_and_counts_only_exact_completed_status() {
    let app = test_input();
    let items = vec![
        todo("1", "in_progress", "first"),
        todo("2", "in_progress", "second"),
        todo("3", "Completed", "wrong case"),
        todo("4", "completed", "done"),
    ];

    let data = compute_todo_data(app, &items);

    assert_eq!(data.total, 4);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "first");
}

#[test]
fn todo_panel_builds_expanded_and_collapsed_surfaces() {
    let items = vec![
        todo("1", "completed", "done"),
        todo("2", "in_progress", "running"),
        todo("3", "pending", "later"),
    ];
    let mut app = test_input();
    app.changed_files = 2;
    app.top_right = true;

    app.expanded = true;
    keep(todo_panel(app, &items, 0, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(app, &items, 1, TodoPanelSurface::InputBottom));
    keep(todo_panel(app, &items, 2, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(app, &items, 3, TodoPanelSurface::InputBottom));

    app.expanded = false;
    keep(todo_panel(app, &items, 0, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(app, &items, 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_empty_expanded_list() {
    let mut app = test_input();
    app.expanded = true;

    keep(todo_panel(app, &[], 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_list_beyond_visible_row_limit() {
    let mut app = test_input();
    app.expanded = true;
    let items = (1..=6)
        .map(|id| todo(&id.to_string(), "pending", &format!("task {id}")))
        .collect::<Vec<_>>();

    keep(todo_panel(app, &items, 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_collapsed_chat_badge_with_count_above_display_cap() {
    let mut app = test_input();
    app.expanded = false;
    let items = (1..=120)
        .map(|id| todo(&id.to_string(), "pending", &format!("task {id}")))
        .collect::<Vec<_>>();

    keep(todo_panel(app, &items, u8::MAX, TodoPanelSurface::ChatTopRight));
}
