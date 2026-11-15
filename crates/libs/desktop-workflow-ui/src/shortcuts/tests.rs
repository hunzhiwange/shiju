#[test]
fn workflow_command_shortcuts_map_zoom_file_and_reload_actions() {
    use crate::message::WorkflowMessage;
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
    use crate::message::WorkflowMessage;
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
    use crate::message::WorkflowMessage;
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
