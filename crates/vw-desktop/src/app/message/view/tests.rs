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
    assert_eq!(settings.min_size, Some(size));
    assert_eq!(settings.max_size, Some(size));
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

    assert!(!app.task_pet_collapsed);
    assert!(app.task_pet_robot_hovered);
    assert_eq!(app.task_pet_avatar_kind, TaskPetAvatarKind::Beauty);
    assert_eq!(app.task_pet_hovered_request_id, Some(41));
    assert_eq!(app.task_pet_reply_request_id, Some(41));
    assert_eq!(app.task_pet_reply_input, " 继续执行 ");
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
