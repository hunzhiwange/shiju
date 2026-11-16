use crate::global_events::{Message, map_global_event};
use iced::keyboard::{self, Modifiers};
use std::path::PathBuf;
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
    assert!(matches!(moved, Some(Message::PointerMoved(12.5, 33.0))));

    let hovered = map_global_event(
        iced::Event::Window(iced::window::Event::FileHovered(PathBuf::from("/tmp/file.txt"))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        hovered,
        Some(Message::HoveredFilePath(path)) if path == "/tmp/file.txt"
    ));

    let left = map_global_event(
        iced::Event::Window(iced::window::Event::FilesHoveredLeft),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(left, Some(Message::HoveredFilesLeft)));
}

#[test]
fn map_global_event_maps_mouse_and_window_events() {
    let id = window_id();

    let released = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(released, Some(Message::GlobalMouseReleased)));

    let cursor_left = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::CursorLeft),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(cursor_left, Some(Message::GlobalCursorLeft)));

    let resized = map_global_event(
        iced::Event::Window(iced::window::Event::Resized(iced::Size::new(800.0, 600.0))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        resized,
        Some(Message::WindowResized(message_id, 800.0, 600.0)) if message_id == id
    ));

    let moved = map_global_event(
        iced::Event::Window(iced::window::Event::Moved(iced::Point::new(4.0, 7.0))),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        moved,
        Some(Message::WindowMoved(message_id, 4.0, 7.0)) if message_id == id
    ));

    let close_requested = map_global_event(
        iced::Event::Window(iced::window::Event::CloseRequested),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        close_requested,
        Some(Message::CloseRequested(message_id)) if message_id == id
    ));

    let closed = map_global_event(
        iced::Event::Window(iced::window::Event::Closed),
        iced::event::Status::Ignored,
        id,
    );
    assert!(matches!(
        closed,
        Some(Message::WindowClosed(message_id)) if message_id == id
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
        Message::WindowUnfocused(window_id) if *window_id == id
    )));
    assert!(
        unfocused_messages.iter().any(|message| matches!(message, Message::PreviewWindowUnfocused))
    );
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
    assert!(matches!(messages.first(), Some(Message::ContextMenuClosed)));
    assert!(messages.iter().any(|message| matches!(message, Message::CloseSearch)));
    assert!(messages.iter().any(|message| matches!(
        message,
        Message::GlobalKeyPressed(keyboard::Key::Named(keyboard::key::Named::Escape), _)
    )));
    #[cfg(not(target_arch = "wasm32"))]
    assert!(messages.iter().any(|message| matches!(message, Message::LspCompletionClosed)));

    assert!(messages.iter().any(|message| matches!(message, Message::FileTreeRenameCancel)));

    let search = map_global_event(
        key_pressed(keyboard::Key::Character("f".into()), Modifiers::COMMAND),
        iced::event::Status::Captured,
        id,
    );
    assert!(matches!(search, Some(Message::OpenSearch)));

    let replace = map_global_event(
        key_pressed(keyboard::Key::Character("F".into()), Modifiers::COMMAND | Modifiers::ALT),
        iced::event::Status::Captured,
        id,
    );
    assert!(matches!(replace, Some(Message::OpenReplace)));

    let find_modifiers = Modifiers::COMMAND | Modifiers::ALT;
    for (key, expected) in [
        ("c", Message::FileTreeFindCaseSensitiveShortcut),
        ("W", Message::FileTreeFindWholeWordShortcut),
        ("x", Message::FileTreeFindRegexShortcut),
    ] {
        let shortcut = map_global_event(
            key_pressed(keyboard::Key::Character(key.into()), find_modifiers),
            iced::event::Status::Captured,
            id,
        );
        assert!(
            matches!(shortcut, Some(message) if std::mem::discriminant(&message) == std::mem::discriminant(&expected))
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
        Message::GlobalKeyPressed(
            keyboard::Key::Named(keyboard::key::Named::Escape),
            modifiers
        ) if *modifiers == Modifiers::COMMAND
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
        Some(Message::GlobalKeyPressed(
            keyboard::Key::Character(_),
            modifiers
        )) if modifiers == Modifiers::SHIFT
    ));
}

#[test]
fn map_global_event_ignores_unhandled_mouse_and_forwards_window_focus() {
    let id = window_id();

    let pressed = map_global_event(
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::event::Status::Ignored,
        id,
    );
    assert!(pressed.is_none());

    for status in [iced::event::Status::Ignored, iced::event::Status::Captured] {
        let focused =
            map_global_event(iced::Event::Window(iced::window::Event::Focused), status, id);
        assert!(matches!(
            focused,
            Some(Message::WindowFocused(window)) if window == id
        ));
    }
}

#[test]
fn file_tree_move_drop_escape_and_unfocus_cancel_tree_origin() {
    for event in [
        iced::Event::Window(iced::window::Event::Unfocused),
        key_pressed(keyboard::Key::Named(keyboard::key::Named::Escape), Modifiers::empty()),
    ] {
        let Some(Message::Batch(messages)) =
            map_global_event(event, iced::event::Status::Captured, window_id())
        else {
            panic!("cancel must be delivered even when captured");
        };
        assert!(messages.iter().any(|message| matches!(message, Message::FileTreeDragCancel)));
    }
}

#[test]
fn file_tree_external_drop_native_event_is_observed_without_hover() {
    assert!(
        map_global_event(
            iced::Event::Window(iced::window::Event::FileDropped("/tmp/文件夹".into())),
            iced::event::Status::Ignored,
            iced::window::Id::unique(),
        )
        .is_some()
    );
}
