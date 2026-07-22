use super::{
    acp_history_controls, acp_selector_button, bottom_bar_round_toggle, dark_tooltip_content,
    popover_item_style, reasoning_effort_selector, selected_context_card, session_control_button,
    skill_title, summarize_names, view, with_dark_tooltip,
};
use crate::app::assets::Icon;
use crate::app::components::input_panel::icons::icon_svg;
use crate::app::state::{
    AcpHistoryReplayMode, ChatSendBehavior, SkillsCatalogItem, SkillsCatalogKind,
};
use crate::app::{Message, QueueItem, TodoPanelPlacement, message};
use iced::widget::{button, text};
use iced::{Element, Length, Theme};
use vw_shared::todo::Todo;

fn catalog_skill(id: &str, title: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: title.to_string(),
        description: String::new(),
        kind: SkillsCatalogKind::Personal,
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "workspace".to_string(),
        source_path: None,
    }
}

fn queue_item(query: &str) -> QueueItem {
    QueueItem {
        created_ms: 0,
        query: query.to_string(),
        attachments: Vec::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        acp_force_new_session: false,
        acp_history_mode: AcpHistoryReplayMode::Discard,
        acp_recent_count: 3,
        full_access_enabled: false,
        send_behavior: ChatSendBehavior::Queue,
        request_history_override: None,
        resume_history_only: false,
        workflow_mode_enabled: false,
    }
}

fn todo(id: &str, status: &str, content: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: status.to_string(),
        content: content.to_string(),
        priority: "medium".to_string(),
    }
}

#[test]
fn summarize_names_lists_up_to_three_names_and_hidden_count() {
    assert_eq!(summarize_names(&[]), "");
    assert_eq!(summarize_names(&["a".into(), "b".into()]), "a、b");
    assert_eq!(summarize_names(&["a".into(), "b".into(), "c".into()]), "a、b、c");
    assert_eq!(summarize_names(&["a".into(), "b".into(), "c".into(), "d".into()]), "a、b、c +1");
}

#[test]
fn skill_title_uses_catalog_title_and_falls_back_to_id() {
    let mut app = crate::app::App::new().0;
    app.skills_settings.catalog = vec![catalog_skill("review", "代码审查")];

    assert_eq!(skill_title(&app, "review"), "代码审查");
    assert_eq!(skill_title(&app, "missing"), "missing");
}

#[test]
fn popover_item_style_delegates_selected_and_unselected_states() {
    let selected = popover_item_style(&Theme::Dark, iced::widget::button::Status::Hovered, true);
    let unselected = popover_item_style(&Theme::Dark, iced::widget::button::Status::Active, false);

    assert_ne!(selected.background, unselected.background);
}

#[test]
fn tooltip_helpers_wrap_content_with_shrink_size() {
    let tip = dark_tooltip_content("提示");
    assert_eq!(tip.as_widget().size().width, Length::Shrink);

    let wrapped = with_dark_tooltip(button(text("content")).into(), "tip");
    assert_eq!(wrapped.as_widget().children().len(), 2);
}

#[test]
fn bottom_bar_round_toggle_builds_tooltip_wrapped_icon_button() {
    let icon: Element<'_, Message> = icon_svg(Icon::Sliders, 14.0).into();
    let element = bottom_bar_round_toggle(
        icon,
        "toggle",
        Message::View(message::ViewMessage::ToggleSessionToolSelectorPopover),
    );

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn acp_history_controls_render_all_modes_and_recent_input() {
    for mode in [
        AcpHistoryReplayMode::Discard,
        AcpHistoryReplayMode::Summary,
        AcpHistoryReplayMode::Full,
        AcpHistoryReplayMode::Recent,
    ] {
        let element = acp_history_controls(mode, 5);

        assert_eq!(element.as_widget().size().width, Length::Fill);
    }
}

#[test]
fn optional_selector_buttons_reflect_app_state() {
    let mut app = crate::app::App::new().0;

    assert!(session_control_button(&app).is_some());
    assert!(acp_selector_button(&app, None).is_some());
    assert!(reasoning_effort_selector(&app, None, None).is_none());

    app.acp_agents = vec!["Codex".to_string(), "Claude Code".to_string()];
    app.show_acp_popover = true;
    let acp = acp_selector_button(&app, Some("Codex".to_string()));
    assert!(acp.is_some());
    let missing_acp = acp_selector_button(&app, Some("Missing Agent".to_string()));
    assert!(missing_acp.is_some());
    assert!(reasoning_effort_selector(&app, Some("codex"), None).is_some());
}

#[test]
fn selected_context_card_is_absent_without_runtime_manual_context() {
    let app = crate::app::App::new().0;

    assert!(selected_context_card(&app).is_none());
}

#[test]
fn input_panel_view_falls_back_when_active_session_has_no_runtime() {
    let mut app = crate::app::App::new().0;
    app.active_session_id = Some("missing-runtime".to_string());

    assert!(app.current_session_runtime_ref().is_none());
    assert!(selected_context_card(&app).is_none());

    let element = view(&app);
    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn selected_context_card_renders_each_manual_selection_shape() {
    let mut app = crate::app::App::new().0;
    app.skills_settings.catalog = vec![catalog_skill("review", "代码审查")];

    app.current_session_runtime_mut().tool_selector.toggle_manual_tool("read");
    assert!(selected_context_card(&app).is_some());

    {
        let selector = &mut app.current_session_runtime_mut().tool_selector;
        selector.toggle_manual_tool("read");
        selector.toggle_manual_skill("review");
    }
    assert!(selected_context_card(&app).is_some());

    app.current_session_runtime_mut().tool_selector.toggle_manual_tool("read");
    assert!(selected_context_card(&app).is_some());
}

#[test]
fn input_panel_view_renders_default_task_queue_and_overlay_states() {
    let mut app = crate::app::App::new().0;
    let default = view(&app);
    assert_eq!(default.as_widget().size().width, Length::Fill);
    drop(default);

    app.input_editor = iced::widget::text_editor::Content::with_text("@src/main.rs:1\nhello");
    app.files.push("/tmp/a.txt".to_string());
    app.show_model_popover = true;
    app.show_file_search = true;
    app.project_path = Some("/repo".to_string());
    app.file_search_query = "src".to_string();
    app.set_file_index("/repo", vec!["src/main.rs".to_string()]);
    let with_overlays = view(&app);
    assert_eq!(with_overlays.as_widget().size().width, Length::Fill);
}

#[test]
fn input_panel_view_renders_drag_hover_and_send_mode_state() {
    let mut app = crate::app::App::new().0;
    app.dragging_file_paths = vec!["src/main.rs".to_string()];
    app.input_drop_hovered = true;
    app.show_send_mode_popover = true;
    app.chat_send_behavior = ChatSendBehavior::StopAndSend;

    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn input_panel_view_renders_app_level_acp_history_and_pending_drop_hover() {
    let mut app = crate::app::App::new().0;
    app.acp_agent = Some("codex".to_string());
    app.acp_history_mode = AcpHistoryReplayMode::Recent;
    app.acp_recent_count = 7;
    app.pending_drop_file_paths = vec!["src/lib.rs".to_string()];
    app.input_drop_hovered = true;
    app.chat_todo_session_id = Some("other-session".to_string());
    app.chat_todo_items = vec![todo("1", "pending", "hidden because session differs")];
    app.input_editor = iced::widget::text_editor::Content::with_text("hello");

    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn input_panel_view_renders_nonempty_queue_while_requesting() {
    let mut app = crate::app::App::new().0;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.queue = vec![queue_item("queued request")];
        runtime.is_requesting = true;
        runtime.submit_anim = 2;
        runtime.input_editor = iced::widget::text_editor::Content::with_text("next request");
        runtime.acp_agent = Some("codex".to_string());
    }

    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn input_panel_view_renders_task_mode_instead_of_nonempty_queue() {
    let mut app = crate::app::App::new().0;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.queue = vec![queue_item("queued request")];
        runtime.task_mode_enabled = true;
        runtime.input_editor = iced::widget::text_editor::Content::with_text("planned task");
    }

    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}

#[test]
fn input_panel_view_renders_matching_todos_at_input_bottom() {
    let mut app = crate::app::App::new().0;
    app.active_session_id = Some("session".to_string());
    app.chat_todo_session_id = Some("session".to_string());
    app.chat_todo_items = vec![todo("1", "in_progress", "working")];
    app.chat_todo_placement = TodoPanelPlacement::InputBottom;
    app.chat_todo_expanded = true;
    {
        let runtime = app.current_session_runtime_mut();
        runtime.acp_agent = Some("codex".to_string());
        runtime.tool_selector.toggle_manual_tool("read");
    }

    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
}
