use super::{cursor_from_point, editor_message_may_change_content, normalize_pos};
use crate::components::editor::Editor;
#[test]
fn content_change_classifier_matches_editing_events() {
    assert!(editor_message_may_change_content(&iced_code_editor::Message::CharacterInput('x')));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Backspace));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Paste("x".to_string())));
    assert!(editor_message_may_change_content(&iced_code_editor::Message::Undo));
    assert!(!editor_message_may_change_content(&iced_code_editor::Message::MouseRelease));
    assert!(!editor_message_may_change_content(&iced_code_editor::Message::MouseHover(
        iced::Point::new(1.0, 1.0)
    )));
}

#[test]
fn content_change_classifier_covers_all_remaining_editing_events() {
    let editing_events = [
        iced_code_editor::Message::Delete,
        iced_code_editor::Message::Enter,
        iced_code_editor::Message::Tab,
        iced_code_editor::Message::Cut,
        iced_code_editor::Message::DeleteSelection,
        iced_code_editor::Message::Redo,
        iced_code_editor::Message::ReplaceNext,
        iced_code_editor::Message::ReplaceAll,
        iced_code_editor::Message::ImeCommit("输入".to_string()),
    ];

    for event in editing_events {
        assert!(editor_message_may_change_content(&event));
    }
}

#[test]
fn normalize_pos_orders_line_and_column_pairs() {
    assert_eq!(normalize_pos((1, 1), (2, 1)), ((1, 1), (2, 1)));
    assert_eq!(normalize_pos((2, 1), (1, 9)), ((1, 9), (2, 1)));
    assert_eq!(normalize_pos((3, 8), (3, 2)), ((3, 2), (3, 8)));
    assert_eq!(normalize_pos((4, 5), (4, 5)), ((4, 5), (4, 5)));
}

#[test]
fn cursor_from_point_respects_gutter_lines_and_wide_chars() {
    let editor = Editor::new("abc\n中文", "text");

    assert_eq!(cursor_from_point(&editor.inner, iced::Point::new(1.0, 1.0)), None);

    let first = cursor_from_point(&editor.inner, iced::Point::new(55.0, 1.0)).expect("first line");
    assert_eq!(first.0, 1);
    assert!(first.1 >= 1);

    let second =
        cursor_from_point(&editor.inner, iced::Point::new(55.0, editor.inner.line_height() + 1.0))
            .expect("second line");
    assert_eq!(second.0, 2);
    assert!(second.1 >= 1);
}

#[test]
fn cursor_from_point_clamps_beyond_content_and_uses_character_midpoints() {
    let editor = Editor::new("ab\n中", "text");
    let text_start = 50.0;

    assert_eq!(
        cursor_from_point(
            &editor.inner,
            iced::Point::new(text_start + editor.inner.char_width() * 0.4, 1.0),
        ),
        Some((1, 1))
    );
    assert_eq!(
        cursor_from_point(
            &editor.inner,
            iced::Point::new(text_start + editor.inner.char_width() * 0.6, 1.0),
        ),
        Some((1, 2))
    );
    assert_eq!(
        cursor_from_point(&editor.inner, iced::Point::new(10_000.0, 10_000.0)),
        Some((2, 2))
    );
}

#[test]
fn cursor_from_point_accepts_origin_when_line_numbers_are_disabled() {
    let mut editor = Editor::new("abc", "text");
    editor.set_line_numbers_enabled(false);

    assert_eq!(cursor_from_point(&editor.inner, iced::Point::new(0.0, 0.0)), Some((1, 1)));
}
