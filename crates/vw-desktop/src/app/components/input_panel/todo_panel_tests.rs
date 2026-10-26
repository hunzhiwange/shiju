//! 验证待办面板的数据选择、排序、完成计数与折叠展示边界。
//! 会话匹配、数字和文本编号混排以及活动请求回退共同约束数据来源优先级。

use super::todo_panel::{
    TodoPanelSurface, compute_todo_data, read_todos_for_panel, todo_id_display, todo_panel,
    todo_status_pill_colors, todo_task_text_color,
};
use crate::app::state::AcpHistoryReplayMode;
use crate::app::{AgentRequest, App, Message, TodoPanelPlacement};
use iced::{Color, Theme};
use vw_shared::todo::Todo;

fn test_app() -> App {
    App::new().0
}

fn todo(id: &str, status: &str, content: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: status.to_string(),
        content: content.to_string(),
        priority: "medium".to_string(),
    }
}

fn agent_request(query: &str) -> std::sync::Arc<AgentRequest> {
    std::sync::Arc::new(AgentRequest {
        id: 1,
        session: "session".to_string(),
        query: query.to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    })
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
fn todo_panel_placement_labels_match_ui_options() {
    assert_eq!(TodoPanelPlacement::ChatTopRight.label(), "右上角");
    assert_eq!(TodoPanelPlacement::InputBottom.label(), "输入底部");
}

#[test]
fn compute_todo_data_counts_completed_and_prefers_running_todo() {
    let app = test_app();
    let items = vec![
        todo("2", "completed", "done"),
        todo("1", "in_progress", "running task"),
        todo("3", "pending", "later"),
    ];

    let data = compute_todo_data(&app, &items);

    assert_eq!(data.total, 3);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "running task");
}

#[test]
fn compute_todo_data_uses_default_text_without_running_work() {
    let app = test_app();
    let items = vec![todo("1", "pending", "later"), todo("2", "completed", "done")];

    let data = compute_todo_data(&app, &items);

    assert_eq!(data.total, 2);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_uses_trimmed_first_line_of_active_request() {
    let mut app = test_app();
    app.active_session_id = Some("session".to_string());
    app.get_session_runtime_mut("session").active_agent_request =
        Some(agent_request("  active request  \nignored second line"));

    let data = compute_todo_data(&app, &[]);

    assert_eq!(data.running_task, "active request");
}

#[test]
fn compute_todo_data_rejects_blank_active_request_first_line() {
    let mut app = test_app();
    app.active_session_id = Some("session".to_string());
    app.get_session_runtime_mut("session").active_agent_request =
        Some(agent_request("   \nnonempty second line"));

    let data = compute_todo_data(&app, &[]);

    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_does_not_replace_blank_running_todo_with_active_request() {
    let mut app = test_app();
    app.active_session_id = Some("session".to_string());
    app.get_session_runtime_mut("session").active_agent_request =
        Some(agent_request("active request"));
    let items = vec![todo("1", "in_progress", "")];

    let data = compute_todo_data(&app, &items);

    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn compute_todo_data_uses_first_running_todo_and_counts_only_exact_completed_status() {
    let app = test_app();
    let items = vec![
        todo("1", "in_progress", "first"),
        todo("2", "in_progress", "second"),
        todo("3", "Completed", "wrong case"),
        todo("4", "completed", "done"),
    ];

    let data = compute_todo_data(&app, &items);

    assert_eq!(data.total, 4);
    assert_eq!(data.completed, 1);
    assert_eq!(data.running_task, "first");
}

#[test]
fn compute_todo_data_ignores_runtime_when_there_is_no_active_session() {
    let mut app = test_app();
    app.get_session_runtime_mut("inactive").active_agent_request = Some(agent_request("hidden"));

    let data = compute_todo_data(&app, &[]);

    assert_eq!(data.running_task, "无执行中任务");
}

#[test]
fn read_todos_for_panel_returns_empty_when_session_does_not_match() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("other".to_string());
    app.chat_todo_items = vec![todo("1", "pending", "hidden")];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert!(items.is_empty());
    assert_eq!(error, None);
}

#[test]
fn read_todos_for_panel_sorts_numeric_ids_before_text_ids() {
    let mut app = test_app();
    app.active_session_id = Some("session".to_string());
    app.chat_todo_session_id = Some("session".to_string());
    app.chat_todo_items = vec![
        todo("b", "pending", "b"),
        todo("10", "pending", "ten"),
        todo("2", "pending", "two"),
        todo("a", "pending", "a"),
    ];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["2", "10", "a", "b"]
    );
}

#[test]
fn read_todos_for_panel_reads_global_todos_when_both_sessions_are_absent() {
    let mut app = test_app();
    app.chat_todo_items = vec![todo("task_2", "pending", "second"), todo("1", "pending", "first")];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>(),
        ["first", "second"]
    );
}

#[test]
fn read_todos_for_panel_preserves_input_order_for_equal_numeric_ids() {
    let mut app = test_app();
    app.active_session_id = Some("session".to_string());
    app.chat_todo_session_id = Some("session".to_string());
    app.chat_todo_items = vec![
        todo("02", "pending", "zero padded"),
        todo("2", "pending", "plain"),
        todo("1", "pending", "first"),
    ];

    let (items, error) = read_todos_for_panel(&app).expect("panel data");

    assert_eq!(error, None);
    assert_eq!(
        items.iter().map(|item| item.content.as_str()).collect::<Vec<_>>(),
        ["first", "zero padded", "plain"]
    );
}

#[test]
fn todo_id_display_trims_numeric_suffix_and_preserves_non_numeric_values() {
    assert_eq!(todo_id_display(" task_42 "), "42");
    assert_eq!(todo_id_display("story_task_0007"), "0007");
    assert_eq!(todo_id_display("42"), "42");
    assert_eq!(todo_id_display("task_alpha"), "task_alpha");
    assert_eq!(todo_id_display(""), "");
}

#[test]
fn todo_id_display_preserves_empty_suffix_and_non_ascii_digits() {
    assert_eq!(todo_id_display(" task_ "), "task_");
    assert_eq!(todo_id_display("task_１２"), "task_１２");
}

#[test]
fn completed_task_text_stays_readable_in_dark_theme() {
    let color = todo_task_text_color(&Theme::Dark, true);

    assert_eq!(color.r, Theme::Dark.palette().text.r);
    assert_eq!(color.g, Theme::Dark.palette().text.g);
    assert_eq!(color.b, Theme::Dark.palette().text.b);
    assert!(color.a >= 0.78);
}

#[test]
fn task_text_colors_cover_incomplete_and_light_completed_states() {
    assert_eq!(todo_task_text_color(&Theme::Dark, false), Theme::Dark.palette().text);
    assert_eq!(todo_task_text_color(&Theme::Light, false), Theme::Light.palette().text);
    assert_eq!(
        todo_task_text_color(&Theme::Light, true),
        Theme::Light.palette().text.scale_alpha(0.78)
    );
}

#[test]
fn status_pill_colors_cover_completed_running_and_pending_states() {
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, true, false),
        (Color::from_rgb8(46, 194, 126), Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, false, true),
        (Theme::Light.palette().primary, Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Light, false, false),
        (Color::from_rgb8(71, 85, 105), Theme::Light.palette().text)
    );
    assert_eq!(
        todo_status_pill_colors(&Theme::Dark, false, false),
        (Color::from_rgb8(96, 165, 250), Color::WHITE)
    );
}

#[test]
fn todo_panel_builds_expanded_and_collapsed_surfaces() {
    let items = vec![
        todo("1", "completed", "done"),
        todo("2", "in_progress", "running"),
        todo("3", "pending", "later"),
    ];
    let mut app = test_app();
    app.git_changed_files = vec!["src/main.rs".to_string(), "README.md".to_string()];
    app.chat_todo_placement = TodoPanelPlacement::ChatTopRight;

    app.chat_todo_expanded = true;
    keep(todo_panel(&app, &items, 0, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(&app, &items, 1, TodoPanelSurface::InputBottom));
    keep(todo_panel(&app, &items, 2, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(&app, &items, 3, TodoPanelSurface::InputBottom));

    app.chat_todo_expanded = false;
    keep(todo_panel(&app, &items, 0, TodoPanelSurface::ChatTopRight));
    keep(todo_panel(&app, &items, 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_empty_expanded_list() {
    let mut app = test_app();
    app.chat_todo_expanded = true;

    keep(todo_panel(&app, &[], 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_list_beyond_visible_row_limit() {
    let mut app = test_app();
    app.chat_todo_expanded = true;
    let items = (1..=6)
        .map(|id| todo(&id.to_string(), "pending", &format!("task {id}")))
        .collect::<Vec<_>>();

    keep(todo_panel(&app, &items, 0, TodoPanelSurface::InputBottom));
}

#[test]
fn todo_panel_builds_collapsed_chat_badge_with_count_above_display_cap() {
    let mut app = test_app();
    app.chat_todo_expanded = false;
    let items = (1..=120)
        .map(|id| todo(&id.to_string(), "pending", &format!("task {id}")))
        .collect::<Vec<_>>();

    keep(todo_panel(&app, &items, u8::MAX, TodoPanelSurface::ChatTopRight));
}
