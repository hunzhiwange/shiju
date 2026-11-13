use super::editor_help::entries;
use iced_code_editor::{CodeEditor, Message};

#[test]
fn ecp05_help_shortcuts_match_platform_and_mode() {
    for mac in [true, false] {
        let rows = entries(mac, true, true);
        assert!(rows.iter().any(|(label, key)| *label == "搜索" && key == if mac { "Cmd+F" } else { "Ctrl+F" }));
        assert!(rows.iter().any(|(label, key)| label.contains("焦点导航") && key == "Shift+Tab"));
        assert!(rows.iter().any(|(label, _)| label.contains("字段循环")));
        let mut editor = CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);
        for key in [':', 'q', '\n'] {
            let _ = editor.update(&Message::VimKey(key));
        }
        assert!(!editor.vim_enabled());
        assert_eq!(editor.content(), "one\ntwo");
        assert!(rows.iter().any(|(label, key)| label.contains("退出 Vim") && key == ":q"));
    }
}

#[test]
fn ecp05_help_omits_unsupported_features() {
    let rows = entries(false, false, false);
    assert!(!rows.is_empty());
    assert!(!rows.iter().any(|(label, _)| {
        ["正则", "整词", "Helix", "全选匹配", "折叠"]
            .iter()
            .any(|unsupported| label.contains(unsupported))
    }));
    assert!(!rows.iter().any(|(_, key)| key == "hjkl"));
    let mut editor = CodeEditor::new("one", "txt");
    let _ = editor.update(&Message::SelectAll);
    assert_eq!(editor.content(), "one");
}

fn route(
    editor: &mut CodeEditor,
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Message> {
    use iced::{Event, Point, Rectangle, Size, keyboard, mouse};
    editor.request_focus();
    let _ = editor.update(&Message::CanvasFocusGained);
    let event = Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Tab),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    });
    iced::widget::canvas::Program::<Message>::update(
        editor,
        &mut Default::default(),
        &event,
        Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0)),
        mouse::Cursor::Unavailable,
    )
    .and_then(|action| action.into_inner().0)
}

#[test]
fn ecp05_help_keyboard_paths_match_native_editor() {
    use iced::keyboard::{Key, Modifiers, key::Named};
    let mut editor = CodeEditor::new("one", "txt");
    assert!(matches!(
        route(&mut editor, Key::Character("f".into()), Modifiers::COMMAND),
        Some(Message::OpenSearch)
    ));
    assert!(matches!(
        route(&mut editor, Key::Named(Named::Tab), Modifiers::SHIFT),
        Some(Message::Backtab)
    ));
    let _ = editor.update(&Message::OpenSearchReplace);
    assert!(matches!(
        route(&mut editor, Key::Named(Named::Tab), Modifiers::NONE),
        Some(Message::SearchDialogTab)
    ));
    assert!(matches!(
        route(&mut editor, Key::Named(Named::Tab), Modifiers::SHIFT),
        Some(Message::SearchDialogShiftTab)
    ));
    let _ = editor.update(&Message::CloseSearch);
    let message = route(&mut editor, Key::Named(Named::Tab), Modifiers::NONE)
        .expect("Tab inserts indentation");
    let _ = editor.update(&message);
    assert_ne!(editor.content(), "one");
}
