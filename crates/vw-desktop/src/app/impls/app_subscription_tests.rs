//! 验证应用订阅根据平台、窗口和运行状态正确组合事件源。
//!
//! 订阅测试组合窗口、主题、文件监听和后台服务状态，确认只注册当前环境需要的事件源。

use std::{path::PathBuf, sync::Arc};

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

fn agent_request(id: u64, session: &str) -> Arc<crate::app::AgentRequest> {
    Arc::new(crate::app::AgentRequest {
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
    })
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
fn requesting_sessions_animate_on_conversation_screen_and_stop_when_idle() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.active_session_id = Some("active".to_string());
    assert!(!needs_activity_animation_tick(&app, false));

    let mut active = SessionRuntimeState::new();
    active.is_requesting = true;
    app.session_runtime_states.insert("active".to_string(), active);
    assert!(needs_activity_animation_tick(&app, false));
    app.session_runtime_states.get_mut("active").unwrap().is_requesting = false;
    assert!(!needs_activity_animation_tick(&app, false));

    let mut background = SessionRuntimeState::new();
    background.is_requesting = true;
    app.session_runtime_states.insert("background".to_string(), background);
    // The session list still displays the background request's running indicator.
    assert!(needs_activity_animation_tick(&app, false));
    app.screen = Screen::TaskBoard;
    assert!(!needs_activity_animation_tick(&app, false));
    assert!(needs_activity_animation_tick(&app, true));
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
fn subscription_request_collection_reuses_shared_request_allocation() {
    let mut app = test_app();
    app.active_session_id = Some("active".to_string());

    let mut active_request = agent_request(1, "active");
    Arc::make_mut(&mut active_request).history = (0..1_024)
        .map(|index| crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: format!("large history message {index}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    let mut active_runtime = SessionRuntimeState::new();
    active_runtime.active_agent_request = Some(active_request);
    app.session_runtime_states.insert("active".to_string(), active_runtime);

    let mut background_runtime = SessionRuntimeState::new();
    background_runtime.active_agent_request = Some(agent_request(2, "background"));
    app.session_runtime_states.insert("background".to_string(), background_runtime);

    let first = collect_agent_subscription_requests(&app);
    let second = collect_agent_subscription_requests(&app);

    assert_eq!(first.iter().map(|request| request.id).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(second.iter().map(|request| request.id).collect::<Vec<_>>(), vec![1, 2]);
    assert!(Arc::ptr_eq(&first[0], &second[0]));
    assert!(Arc::ptr_eq(&first[1], &second[1]));
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
fn subscription_builds_task_board_ui_tick() {
    let mut app = test_app();

    app.terminal.is_visible = true;
    subscription_builds(&app);

    app.screen = Screen::TaskBoard;
    subscription_builds(&app);
}

#[test]
fn project_subscription_does_not_schedule_periodic_task_or_session_refresh() {
    let source = include_str!("app_subscription.rs");

    assert!(!source.contains(".map(|_| Message::TaskBoard(message::TaskBoardMessage::LoadTasks))"));
    assert!(!source.contains("ProjectMessage::ProjectSessionsRefreshTick"));
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
