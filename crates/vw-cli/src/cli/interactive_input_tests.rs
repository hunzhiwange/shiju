use super::interactive_input::{InputEditResult, apply_key_to_input, insert_newline};
use crossterm::event::KeyCode;

#[test]
fn apply_key_to_input_inserts_and_removes_unicode_characters() {
    let mut input = String::from("你b");
    let mut cursor = 1;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Char('好')),
        InputEditResult::Updated
    );
    assert_eq!(input, "你好b");
    assert_eq!(cursor, 2);

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Backspace),
        InputEditResult::Updated
    );
    assert_eq!(input, "你b");
    assert_eq!(cursor, 1);
}

#[test]
fn apply_key_to_input_keeps_cursor_within_horizontal_bounds() {
    let mut input = String::from("你ab");
    let mut cursor = 0;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Left),
        InputEditResult::Updated
    );
    assert_eq!(cursor, 0);

    for expected_cursor in 1..=3 {
        assert_eq!(
            apply_key_to_input(&mut input, &mut cursor, KeyCode::Right),
            InputEditResult::Updated
        );
        assert_eq!(cursor, expected_cursor);
    }

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Right),
        InputEditResult::Updated
    );
    assert_eq!(cursor, 3);
    assert_eq!(input, "你ab");
}

#[test]
fn apply_key_to_input_moves_home_and_end_by_character_count() {
    let mut input = String::from("你ab");
    let mut cursor = 1;

    assert_eq!(apply_key_to_input(&mut input, &mut cursor, KeyCode::End), InputEditResult::Updated);
    assert_eq!(cursor, 3);

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Home),
        InputEditResult::Updated
    );
    assert_eq!(cursor, 0);
    assert_eq!(input, "你ab");
}

#[test]
fn apply_key_to_input_backspace_at_start_preserves_input() {
    let mut input = String::from("abc");
    let mut cursor = 0;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Backspace),
        InputEditResult::Updated
    );
    assert_eq!(input, "abc");
    assert_eq!(cursor, 0);
}

#[test]
fn apply_key_to_input_delete_removes_unicode_character_at_cursor() {
    let mut input = String::from("a你b");
    let mut cursor = 1;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Delete),
        InputEditResult::Updated
    );
    assert_eq!(input, "ab");
    assert_eq!(cursor, 1);
}

#[test]
fn apply_key_to_input_delete_at_end_preserves_input() {
    let mut input = String::from("abc");
    let mut cursor = 3;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Delete),
        InputEditResult::Updated
    );
    assert_eq!(input, "abc");
    assert_eq!(cursor, 3);
}

#[test]
fn apply_key_to_input_normalizes_line_endings_to_newline() {
    for line_ending in ['\n', '\r'] {
        let mut input = String::from("ab");
        let mut cursor = 1;

        assert_eq!(
            apply_key_to_input(&mut input, &mut cursor, KeyCode::Char(line_ending)),
            InputEditResult::Updated
        );
        assert_eq!(input, "a\nb");
        assert_eq!(cursor, 2);
    }
}

#[test]
fn apply_key_to_input_inserts_character_into_empty_buffer() {
    let mut input = String::new();
    let mut cursor = 0;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::Char('界')),
        InputEditResult::Updated
    );
    assert_eq!(input, "界");
    assert_eq!(cursor, 1);
}

#[test]
fn apply_key_to_input_rejects_non_editing_key() {
    let mut input = String::from("abc");
    let mut cursor = 1;

    assert_eq!(
        apply_key_to_input(&mut input, &mut cursor, KeyCode::F(1)),
        InputEditResult::NoChange
    );
    assert_eq!(input, "abc");
    assert_eq!(cursor, 1);
}

#[test]
fn insert_newline_inserts_at_unicode_cursor_and_advances_cursor() {
    let mut input = String::from("你b");
    let mut cursor = 1;

    insert_newline(&mut input, &mut cursor);
    assert_eq!(input, "你\nb");
    assert_eq!(cursor, 2);
}
