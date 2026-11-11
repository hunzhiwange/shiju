//! 视图消息总路由测试，覆盖全局快捷键、工作流按键和任务宠物窗口交互。
//! 测试确保不同屏幕的消息只进入对应分支，并保持窗口配置与输入状态契约。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_window_settings_are_fixed_transparent_and_always_on_top() {
    let size = iced::Size::new(320.0, 180.0);
    let position = iced::Point::new(48.0, 72.0);

    let settings = super::task_pet_window_settings(size, position);

    assert_eq!(settings.size, size);
    assert_eq!(settings.min_size, None);
    assert_eq!(settings.max_size, None);
    match settings.position {
        iced::window::Position::Specific(actual) => assert_eq!(actual, position),
        other => panic!("expected specific position, got {other:?}"),
    }
    assert!(!settings.resizable);
    assert!(!settings.decorations);
    assert!(settings.transparent);
    assert_eq!(settings.level, iced::window::Level::AlwaysOnTop);
    assert!(!settings.exit_on_close_request);
}

#[test]
fn workflow_command_shortcuts_map_zoom_file_and_reload_actions() {
    use crate::apps::workflow::WorkflowMessage;
    use iced::keyboard::{Key, Modifiers};

    for key in ["+", "="] {
        assert!(matches!(
            super::workflow_key_message(&Key::Character(key.into()), Modifiers::COMMAND, false),
            Some(WorkflowMessage::Zoom(factor, None)) if (factor - 1.1).abs() < f32::EPSILON
        ));
    }
    assert!(matches!(
        super::workflow_key_message(&Key::Character("-".into()), Modifiers::COMMAND, false),
        Some(WorkflowMessage::Zoom(factor, None))
            if (factor - 1.0 / 1.1).abs() < f32::EPSILON
    ));
    assert!(matches!(
        super::workflow_key_message(&Key::Character("0".into()), Modifiers::COMMAND, false),
        Some(WorkflowMessage::ZoomFit)
    ));
    assert!(matches!(
        super::workflow_key_message(&Key::Character("O".into()), Modifiers::COMMAND, false),
        Some(WorkflowMessage::OpenFile)
    ));
    assert!(matches!(
        super::workflow_key_message(&Key::Character("r".into()), Modifiers::COMMAND, false),
        Some(WorkflowMessage::Reload)
    ));
}

#[test]
fn workflow_navigation_shortcuts_map_pan_and_cancel_actions() {
    use crate::apps::workflow::WorkflowMessage;
    use iced::Vector;
    use iced::keyboard::{Key, Modifiers, key::Named};

    let cases = [
        (Named::ArrowLeft, Vector::new(-48.0, 0.0)),
        (Named::ArrowRight, Vector::new(48.0, 0.0)),
        (Named::ArrowUp, Vector::new(0.0, -48.0)),
        (Named::ArrowDown, Vector::new(0.0, 48.0)),
    ];
    for (key, expected) in cases {
        assert!(matches!(
            super::workflow_key_message(&Key::Named(key), Modifiers::NONE, false),
            Some(WorkflowMessage::PanBy(delta)) if delta == expected
        ));
    }
    assert!(matches!(
        super::workflow_key_message(&Key::Named(Named::Escape), Modifiers::NONE, false),
        Some(WorkflowMessage::CancelInteraction)
    ));
}

#[test]
fn workflow_delete_shortcut_targets_selected_node_before_edge() {
    use crate::apps::workflow::WorkflowMessage;
    use iced::keyboard::{Key, Modifiers, key::Named};

    for key in [Named::Backspace, Named::Delete] {
        assert!(matches!(
            super::workflow_key_message(&Key::Named(key), Modifiers::NONE, true),
            Some(WorkflowMessage::DeleteSelectedNode)
        ));
        assert!(matches!(
            super::workflow_key_message(&Key::Named(key), Modifiers::NONE, false),
            Some(WorkflowMessage::DeleteSelectedEdge)
        ));
    }
}

#[test]
fn workflow_key_mapping_rejects_unrecognized_or_modified_navigation_keys() {
    use iced::keyboard::{Key, Modifiers, key::Named};

    assert!(
        super::workflow_key_message(&Key::Character("x".into()), Modifiers::COMMAND, false)
            .is_none()
    );
    assert!(
        super::workflow_key_message(&Key::Character("+".into()), Modifiers::NONE, false).is_none()
    );
    assert!(
        super::workflow_key_message(&Key::Named(Named::ArrowLeft), Modifiers::COMMAND, false)
            .is_none()
    );
}

#[test]
fn global_command_shortcuts_are_case_insensitive_and_require_command_modifier() {
    use crate::app::Message;
    use iced::keyboard::{Key, Modifiers};

    assert!(matches!(
        super::global_command_message(&Key::Character("P".into()), Modifiers::COMMAND),
        Some(Message::Search(crate::app::message::SearchMessage::Open))
    ));
    assert!(matches!(
        super::global_command_message(&Key::Character("c".into()), Modifiers::COMMAND),
        Some(Message::CopyShortcut)
    ));
    assert!(super::global_command_message(&Key::Character("p".into()), Modifiers::NONE).is_none());
    assert!(
        super::global_command_message(&Key::Character("x".into()), Modifiers::COMMAND).is_none()
    );
}

#[test]
fn task_pet_view_messages_update_interaction_state() {
    use crate::app::message::view::ViewMessage;
    use crate::app::state::TaskPetAvatarKind;

    let mut app = crate::app::App::new().0;
    assert!(app.task_pet_collapsed);
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::Robot);

    let _ = super::update(&mut app, ViewMessage::TaskPetToggleCollapsed);
    let _ = super::update(&mut app, ViewMessage::TaskPetRobotHover(true));
    let _ = super::update(&mut app, ViewMessage::TaskPetAvatarCycle);
    let _ = super::update(&mut app, ViewMessage::TaskPetHover(Some(41)));
    let _ = super::update(&mut app, ViewMessage::TaskPetReplyPressed(41));
    let _ =
        super::update(&mut app, ViewMessage::TaskPetReplyInputChanged(" 继续执行 ".to_string()));
    let _ = super::update(&mut app, ViewMessage::TaskPetContextMenuOpened(12.0, 24.0));

    assert!(!app.task_pet_collapsed);
    assert!(app.task_pet_robot_hovered);
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::Beauty);
    assert_eq!(app.task_pet_hovered_request_id, Some(41));
    assert_eq!(app.task_pet_reply_request_id, Some(41));
    assert_eq!(app.task_pet_reply_input, " 继续执行 ");
    assert_eq!(app.task_pet_context_menu_pos, Some((12.0, 24.0)));

    let _ = super::update(&mut app, ViewMessage::TaskPetContextMenuClosed);
    assert_eq!(app.task_pet_context_menu_pos, None);
}

#[test]
fn task_pet_reply_pressed_again_closes_editor_and_clears_draft() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    let _ = super::update(&mut app, ViewMessage::TaskPetReplyPressed(9));
    let _ = super::update(&mut app, ViewMessage::TaskPetReplyInputChanged("draft".to_string()));

    let _ = super::update(&mut app, ViewMessage::TaskPetReplyPressed(9));

    assert_eq!(app.task_pet_reply_request_id, None);
    assert!(app.task_pet_reply_input.is_empty());
}

#[test]
fn update_routes_panel_layout_theme_and_activity_messages() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;
    app.show_settings = false;

    assert!(!app.show_settings);
    let _ = super::update(&mut app, ViewMessage::ToggleSettingsPanel);
    assert!(app.show_settings);

    assert!(!app.dragging_settings);
    let _ = super::update(&mut app, ViewMessage::SettingsDragStarted);
    assert!(app.dragging_settings);
    assert_eq!(app.settings_drag_start_width, app.settings_panel_width);

    let _ = super::update(&mut app, ViewMessage::AppThemeSelected(iced::Theme::Light));
    assert_eq!(app.app_theme, iced::Theme::Light);

    app.status_animation_frame = 7;
    let _ = super::update(&mut app, ViewMessage::ActivityAnimationTick);
    assert_eq!(app.status_animation_frame, 8);
}

#[test]
fn update_handles_task_pet_drag_remove_and_empty_reply_submit() {
    use crate::app::message::view::ViewMessage;

    let mut app = crate::app::App::new().0;

    let _ = super::update(&mut app, ViewMessage::TaskPetDragStarted);
    assert!(app.task_pet_dragging);

    let _ = super::update(&mut app, ViewMessage::TaskPetRemove(77));
    assert!(app.task_pet_dismissed_request_ids.contains(&77));

    let _ = super::update(&mut app, ViewMessage::TaskPetReplySubmit);
    assert!(app.task_pet_reply_request_id.is_none());
}

#[test]
fn global_key_update_routes_to_global_command_outside_workflow_screen() {
    use crate::app::message::view::ViewMessage;
    use iced::keyboard::{Key, Modifiers};

    let mut app = crate::app::App::new().0;

    let _ = super::update(
        &mut app,
        ViewMessage::GlobalKeyPressed(Key::Character("p".into()), Modifiers::COMMAND),
    );
    let _ = super::update(
        &mut app,
        ViewMessage::GlobalKeyPressed(Key::Character("x".into()), Modifiers::COMMAND),
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct WorkBrowserProtocolProbe {
    protocol: Vec<u8>,
    running: bool,
    terminations: usize,
}

#[cfg(not(target_arch = "wasm32"))]
struct WorkBrowserMemoryProcess {
    probe: std::sync::Arc<std::sync::Mutex<WorkBrowserProtocolProbe>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::app::webview_runtime::BrowserProcess for WorkBrowserMemoryProcess {
    fn write_protocol(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.probe.lock().expect("work browser probe lock").protocol.extend_from_slice(bytes);
        Ok(())
    }

    fn take_events(&mut self) -> Vec<crate::app::webview_runtime::WorkBrowserEvent> {
        Vec::new()
    }

    fn is_running(&mut self) -> std::io::Result<bool> {
        Ok(self.probe.lock().expect("work browser probe lock").running)
    }

    fn terminate(&mut self) -> std::io::Result<()> {
        let mut probe = self.probe.lock().expect("work browser probe lock");
        probe.running = false;
        probe.terminations += 1;
        Ok(())
    }

    fn terminate_now(&mut self) -> std::io::Result<()> {
        self.terminate()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn system_settings_messages_hide_and_restore_running_work_browser() {
    use crate::app::Screen;
    use crate::app::components::system_settings::SystemTab;
    use crate::app::message::view::ViewMessage;
    use crate::app::state::{WorkResourceMode, WorkspaceMode};
    use crate::app::webview_runtime::WebviewBounds;

    let mut app = crate::app::App::new().0;
    app.screen = Screen::Project;
    app.workspace_mode = WorkspaceMode::Work;
    app.work_resource_mode = WorkResourceMode::Browser;
    app.window_size = (1_400.0, 900.0);
    let probe = std::sync::Arc::new(std::sync::Mutex::new(WorkBrowserProtocolProbe {
        running: true,
        ..WorkBrowserProtocolProbe::default()
    }));
    let mut spawn_count = 0;
    app.work_browser_controller
        .ensure_started_with(
            "https://example.com/",
            WebviewBounds { x: 0, y: 0, width: 800, height: 600 },
            |_| {
                spawn_count += 1;
                Ok(Box::new(WorkBrowserMemoryProcess { probe: std::sync::Arc::clone(&probe) }))
            },
        )
        .expect("controlled work browser should start");
    app.work_browser_controller.set_visible(true).expect("work browser should be visible");

    for open_message in [
        ViewMessage::ToggleSystemSettings,
        ViewMessage::OpenSystemSettingsTab(SystemTab::Browser),
        ViewMessage::OpenSystemSettingsModelDetail("provider".into(), "model".into()),
    ] {
        probe.lock().expect("work browser probe lock").protocol.clear();

        let _ = super::update(&mut app, open_message);
        assert!(app.show_system_settings);
        assert_eq!(
            String::from_utf8_lossy(&probe.lock().expect("work browser probe lock").protocol),
            "hide\n"
        );

        let _ = super::update(&mut app, ViewMessage::ToggleSystemSettings);
        assert!(!app.show_system_settings);
        assert!(
            String::from_utf8_lossy(&probe.lock().expect("work browser probe lock").protocol)
                .ends_with("show\n")
        );
    }

    let probe = probe.lock().expect("work browser probe lock");
    assert_eq!(spawn_count, 1);
    assert_eq!(probe.terminations, 0);
    assert!(!String::from_utf8_lossy(&probe.protocol).contains("close\n"));
}

#[test]
fn mini_companion_hidden_and_avatar_are_independent() {
    use crate::app::{App, message::ViewMessage, state::TaskPetAvatarKind};
    let mut app = App::new().0;
    assert!(!app.task_pet_visible);
    let _ = super::update(&mut app, ViewMessage::TaskPetAvatarSelected(TaskPetAvatarKind::None));
    assert!(!app.task_pet_visible);
    assert!(app.task_pet_window_id.is_none());
    let _ = super::update(&mut app, ViewMessage::TaskPetToggleWindow);
    let window = app.task_pet_window_id.unwrap();
    assert!(app.task_pet_visible);
    let _ = super::update(&mut app, ViewMessage::TaskPetScaleChanged(1.25));
    assert_eq!(app.task_pet_window_id, Some(window));
    let _ = super::update(&mut app, ViewMessage::TaskPetToggleWindow);
    assert!(!app.task_pet_visible);
    // Iced 的 Closed 事件到达前仍保留窗口 ID。
    assert_eq!(app.task_pet_window_id, Some(window));
    let _ = super::update(&mut app, ViewMessage::WindowClosed(window));
    assert!(app.task_pet_window_id.is_none());
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::None);
    assert_eq!(app.task_pet_scale, 1.25);
}

#[test]
fn mini_companion_close_request_keeps_window_until_closed() {
    use crate::app::{App, message::ViewMessage, state::TaskPetAvatarKind};
    let mut app = App::new().0;
    let main = iced::window::Id::unique();
    let pet = iced::window::Id::unique();
    app.register_window_ids(main, Some(pet));
    app.task_pet_visible = true;
    app.task_pet_avatar_kind = TaskPetAvatarKind::Handsome;
    let _ = super::update(&mut app, ViewMessage::CloseRequested(pet));
    assert!(!app.task_pet_visible);
    assert_eq!(app.task_pet_window_id, Some(pet));
    let _ = super::update(&mut app, ViewMessage::WindowClosed(pet));
    assert_eq!(app.main_window_id, Some(main));
    assert!(app.task_pet_window_id.is_none());
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::Handsome);
}

#[test]
fn mini_companion_invalid_scale_does_not_enqueue_a_save() {
    use crate::app::{App, message::ViewMessage};
    let mut app = App::new().0;
    let _ = super::update(&mut app, ViewMessage::TaskPetScaleChanged(f32::NAN));
    assert_eq!(app.task_pet_scale, 1.0);
    assert!(app.active_toast.as_ref().unwrap().message.contains("有限数值"));
    let mut late = serde_json::json!({"mini_companion":{"scale":1.5}});
    app.retain_unedited_desktop_preferences(&mut late);
    assert!(late.get("mini_companion").is_some());
}

#[test]
fn mini_companion_show_is_idempotent() {
    use super::ViewMessage;
    let mut app = crate::app::App::new().0;
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    let id = app.task_pet_window_id.expect("native Mini opens");
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    assert!(app.task_pet_visible);
    assert_eq!(app.task_pet_window_id, Some(id));
}

#[test]
fn mini_companion_hide_preserves_avatar_and_tasks() {
    use super::ViewMessage;
    let mut app = crate::app::App::new().0;
    app.main_window_id = Some(iced::window::Id::unique());
    app.task_pet_avatar_kind = crate::app::state::TaskPetAvatarKind::None;
    app.task_pet_scale = 1.25;
    app.task_pet_dismissed_request_ids.insert(42);
    app.task_pet_items.push(crate::app::state::TaskPetItem {
        request_id: 7,
        session_id: "current".into(),
        title: "任务".into(),
        detail: "执行中".into(),
        detail_feedback: None,
        project: None,
        status: crate::app::state::TaskPetStatus::Running,
        last_click_at: None,
    });
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    let id = app.task_pet_window_id.unwrap();
    let _ = super::update(&mut app, ViewMessage::TaskPetHideWindow);
    assert!(!app.task_pet_visible);
    let _ = super::update(&mut app, ViewMessage::WindowClosed(id));
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    assert_ne!(app.task_pet_window_id, Some(id));
    assert_eq!(app.task_pet_avatar_kind, crate::app::state::TaskPetAvatarKind::None);
    assert_eq!(app.task_pet_scale, 1.25);
    assert!(app.task_pet_dismissed_request_ids.contains(&42));
    assert_eq!(app.task_pet_items.len(), 1);
    assert_eq!(app.task_pet_items[0].request_id, 7);
}

#[test]
fn mini_companion_show_while_closing_reopens_after_callback() {
    use super::ViewMessage;
    let mut app = crate::app::App::new().0;
    let main = iced::window::Id::unique();
    app.main_window_id = Some(main);
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    let old = app.task_pet_window_id.unwrap();
    let _ = super::update(&mut app, ViewMessage::TaskPetHideWindow);
    let _ = super::update(&mut app, ViewMessage::TaskPetShowWindow);
    assert_eq!(app.task_pet_window_id, Some(old));
    let _ = super::update(&mut app, ViewMessage::WindowClosed(old));
    assert!(app.task_pet_visible);
    assert_ne!(app.task_pet_window_id, Some(old));
    assert!(app.task_pet_window_id.is_some());
    assert_eq!(app.main_window_id, Some(main));
}

#[test]
fn mini_companion_write_keeps_current_session_and_draft() {
    use super::ViewMessage;
    let mut app = crate::app::App::new().0;
    app.active_session_id = Some("current".into());
    app.task_pet_reply_input = "draft".into();
    let _ = super::update(&mut app, ViewMessage::TaskPetWriteMessage);
    assert_eq!(app.active_session_id.as_deref(), Some("current"));
    assert_eq!(app.task_pet_reply_input, "draft");
    assert!(!app.task_pet_dragging);
}
