use std::path::PathBuf;

use iced::keyboard::{self, Modifiers};

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::preview::PreviewTab;
use crate::app::state::{AcpHistoryReplayMode, SessionRuntimeState};

fn test_app() -> App {
    App::new().0
}

fn window_id() -> iced::window::Id {
    iced::window::Id::unique()
}

fn key_pressed(key: keyboard::Key, modifiers: Modifiers) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        key,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn subscription_builds(app: &App) {
    let _subscription = app.subscription();
}

fn recent_project_meta(path: &str, interval: u64) -> crate::app::RecentProjectMeta {
    crate::app::RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: interval,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn preview_tab(path: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: "main.rs".to_string(),
        content: "fn main() {}\n".to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: crate::app::components::editor::Editor::new("fn main() {}", "rust"),
        scroll_id: iced::widget::Id::unique(),
        lsp_server_key: Some("rust-analyzer"),
        lsp_uri: Some("file:///workspace/src/main.rs".to_string()),
        lsp_language_id: Some("rust".to_string()),
    }
}

fn agent_request(id: u64, session: &str) -> crate::app::AgentRequest {
    crate::app::AgentRequest {
        id,
        session: session.to_string(),
        query: "hello".to_string(),
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
    }
}

#[test]
fn map_global_event_maps_pointer_and_file_events() {
    let id = window_id();

    let moved = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: iced::Point::new(12.5, 33.0),
        }),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(moved, Some(Message::View(message::ViewMessage::PointerMoved(12.5, 33.0)))));

    let hovered = map_global_event(
        iced::Event::Window(iced::window::Event::FileHovered(PathBuf::from("/tmp/file.txt"))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        hovered,
        Some(Message::View(message::ViewMessage::HoveredFilePath(path))) if path == "/tmp/file.txt"
    ));

    let left = map_global_event(
        iced::Event::Window(iced::window::Event::FilesHoveredLeft),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(left, Some(Message::View(message::ViewMessage::HoveredFilesLeft))));
}

#[test]
fn map_global_event_maps_mouse_and_window_events() {
    let id = window_id();

    let released = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(released, Some(Message::View(message::ViewMessage::GlobalMouseReleased))));

    let cursor_left = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::CursorLeft),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(cursor_left, Some(Message::View(message::ViewMessage::GlobalCursorLeft))));

    let resized = map_global_event(
        iced::Event::Window(iced::window::Event::Resized(iced::Size::new(800.0, 600.0))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        resized,
        Some(Message::View(message::ViewMessage::WindowResized(message_id, 800.0, 600.0))) if message_id == id
    ));

    let moved = map_global_event(
        iced::Event::Window(iced::window::Event::Moved(iced::Point::new(4.0, 7.0))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        moved,
        Some(Message::View(message::ViewMessage::WindowMoved(message_id, 4.0, 7.0))) if message_id == id
    ));

    let close_requested = map_global_event(
        iced::Event::Window(iced::window::Event::CloseRequested),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        close_requested,
        Some(Message::View(message::ViewMessage::CloseRequested(message_id))) if message_id == id
    ));

    let closed = map_global_event(
        iced::Event::Window(iced::window::Event::Closed),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        closed,
        Some(Message::View(message::ViewMessage::WindowClosed(message_id))) if message_id == id
    ));

    let unfocused = map_global_event(
        iced::Event::Window(iced::window::Event::Unfocused),
        iced::event::Status::Ignored,
        id,
    );
    let Some(Message::Batch(unfocused_messages)) = unfocused else {
        panic!("window unfocus should hide docked surfaces and notify preview");
    };
    assert!(unfocused_messages.iter().any(|message| matches!(
        message,
        Message::View(message::ViewMessage::WindowUnfocused(window_id)) if *window_id == id
    )));
    assert!(unfocused_messages.iter().any(|message| matches!(
        message,
        Message::Preview(message::PreviewMessage::WindowUnfocused)
    )));
}

#[test]
fn map_global_event_maps_keyboard_shortcuts() {
    let id = window_id();

    let escape = map_global_event(
        key_pressed(keyboard::Key::Named(keyboard::key::Named::Escape), Modifiers::empty()),
        iced::event::Status::Ignored,
        id,
    );
    let Some(Message::Batch(messages)) = escape else {
        panic!("escape should close global transient UI");
    };
    assert!(matches!(
        messages.first(),
        Some(Message::TaskBoard(message::TaskBoardMessage::ContextMenuClosed))
    ));
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Editor(message::EditorMessage::CloseSearch)))
    );
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::View(message::ViewMessage::GlobalKeyPressed(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            _
        ))
    )));
    #[cfg(not(target_arch = "wasm32"))]
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::Preview(message::PreviewMessage::LspCompletionClosed)
    )));

    let search = map_global_event(
        key_pressed(keyboard::Key::Character("f".into()), Modifiers::COMMAND),
        iced::event::Status::Captured,
        id,
    );
    assert!(matches!(search, Some(Message::Editor(message::EditorMessage::OpenSearch))));

    let replace = map_global_event(
        key_pressed(keyboard::Key::Character("F".into()), Modifiers::COMMAND | Modifiers::ALT),
        iced::event::Status::Captured,
        id,
    );
    assert!(matches!(replace, Some(Message::Editor(message::EditorMessage::OpenReplace))));

    let find_modifiers = Modifiers::COMMAND | Modifiers::ALT;
    for (key, expected) in [
        ("c", message::ProjectMessage::FileTreeFindCaseSensitiveShortcut),
        ("W", message::ProjectMessage::FileTreeFindWholeWordShortcut),
        ("x", message::ProjectMessage::FileTreeFindRegexShortcut),
    ] {
        let shortcut = map_global_event(
            key_pressed(keyboard::Key::Character(key.into()), find_modifiers),
            iced::event::Status::Captured,
            id,
        );
        assert!(
            matches!(shortcut, Some(Message::Project(message)) if std::mem::discriminant(&message) == std::mem::discriminant(&expected))
        );
    }

    let shifted = map_global_event(
        key_pressed(keyboard::Key::Character("c".into()), find_modifiers | Modifiers::SHIFT),
        iced::event::Status::Captured,
        id,
    );
    assert!(shifted.is_none());
}

#[test]
fn map_global_event_handles_escape_before_captured_filter() {
    let result = map_global_event(
        key_pressed(keyboard::Key::Named(keyboard::key::Named::Escape), Modifiers::COMMAND),
        iced::event::Status::Captured,
        window_id(),
    );

    let Some(Message::Batch(messages)) = result else {
        panic!("captured escape should still close global transient UI");
    };
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::View(message::ViewMessage::GlobalKeyPressed(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            modifiers
        )) if *modifiers == Modifiers::COMMAND
    )));
}

#[test]
fn map_global_event_ignores_captured_non_shortcut_keys() {
    let id = window_id();

    let captured = map_global_event(
        key_pressed(keyboard::Key::Character("x".into()), Modifiers::empty()),
        iced::event::Status::Captured,
        id,
    );
    assert!(captured.is_none());

    let ignored = map_global_event(
        iced::Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::SHIFT)),
        iced::event::Status::Ignored,
        id,
    );
    assert!(ignored.is_none());

    let uncaptured = map_global_event(
        key_pressed(keyboard::Key::Character("x".into()), Modifiers::SHIFT),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        uncaptured,
        Some(Message::View(message::ViewMessage::GlobalKeyPressed(
            keyboard::Key::Character(_),
            modifiers
        ))) if modifiers == Modifiers::SHIFT
    ));
}

#[test]
fn map_global_event_ignores_unhandled_mouse_and_window_events() {
    let id = window_id();

    let pressed = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::event::Status::Ignored,
        id,
    );
    assert!(pressed.is_none());

    let focused = map_global_event(
        iced::Event::Window(iced::window::Event::Focused),
        iced::event::Status::Ignored,
        id,
    );
    assert!(focused.is_none());
}

#[test]
fn subscription_builds_default_and_polling_states() {
    let mut app = test_app();
    subscription_builds(&app);

    app.question_modal_request_id = Some("question".to_string());
    app.permission_modal_request_id = Some("permission".to_string());
    app.task_pet_window_id = Some(window_id());
    subscription_builds(&app);

    app.active_session_id = Some("active".to_string());
    app.session_runtime_states.insert("active".to_string(), SessionRuntimeState::new());
    app.session_runtime_states.get_mut("active").unwrap().is_requesting = true;
    subscription_builds(&app);
}

#[test]
fn subscription_builds_agent_runtime_states() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());

    let mut active_runtime = SessionRuntimeState::new();
    active_runtime.active_agent_request = Some(agent_request(1, "active"));
    app.session_runtime_states.insert("active".to_string(), active_runtime);

    let mut inactive_runtime = SessionRuntimeState::new();
    inactive_runtime.active_agent_request = Some(agent_request(2, "inactive"));
    app.session_runtime_states.insert("inactive".to_string(), inactive_runtime);

    subscription_builds(&app);
}

#[test]
fn workflow_run_subscription_is_active_only_while_running() {
    let mut app = test_app();
    assert!(workflow_run_subscription_request(&app).is_none());

    app.workflow_state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        [("topic".to_string(), serde_json::json!("rust"))].into(),
    );
    let request =
        workflow_run_subscription_request(&app).expect("running workflow should subscribe");
    assert_eq!(request.uuid, "workflow-1");
    assert_eq!(request.query, "hello");
    assert_eq!(request.inputs.get("topic"), Some(&serde_json::json!("rust")));
    assert_eq!(request.user, "desktop-workflow");

    app.workflow_state.finish_saved_app_run();
    assert!(workflow_run_subscription_request(&app).is_none());
}

#[test]
fn subscription_builds_tool_and_project_ticks() {
    let mut app = test_app();

    app.terminal.is_visible = true;
    app.task_board_settings.auto_refresh = true;
    app.task_board_settings.refresh_interval_seconds = 0;
    subscription_builds(&app);

    app.screen = Screen::TaskBoard;
    app.task_board_settings.refresh_interval_seconds = 4000;
    subscription_builds(&app);

    app.show_settings = true;
    app.project_path = Some("/tmp/project".to_string());
    app.recent_projects_meta = vec![recent_project_meta("/tmp/project", 0)];
    subscription_builds(&app);

    app.show_settings = false;
    app.hovered_recent_project = Some("/tmp/hovered".to_string());
    app.recent_projects_meta = vec![recent_project_meta("/tmp/hovered", 4000)];
    subscription_builds(&app);

    app.hovered_recent_project = Some("/tmp/missing".to_string());
    subscription_builds(&app);
}

#[test]
fn subscription_builds_settings_ticks() {
    let mut app = test_app();

    app.show_settings = true;
    app.provider_settings.models_syncing = true;
    subscription_builds(&app);
}

#[test]
fn subscription_builds_workflow_and_update_progress_states() {
    let mut app = test_app();
    let (_progress_tx, progress_rx) = std::sync::mpsc::channel();
    app.app_update_progress_rx = Some(progress_rx);
    app.workflow_state.begin_saved_app_run(
        "workflow-1".to_string(),
        "hello".to_string(),
        Default::default(),
    );

    subscription_builds(&app);

    app.app_update_progress_rx = None;
    app.workflow_state.finish_saved_app_run();
    subscription_builds(&app);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn subscription_builds_preview_lsp_enabled_and_disabled_states() {
    let mut app = test_app();
    let path = "/workspace/src/main.rs";
    app.active_preview_path = Some(path.to_string());
    app.preview_tabs.push(preview_tab(path));

    app.lsp_disabled = false;
    subscription_builds(&app);

    app.lsp_disabled = true;
    subscription_builds(&app);
}
