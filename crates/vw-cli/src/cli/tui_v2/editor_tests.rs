use super::editor::{
    EditorCommand, apply_editor_command, byte_index_for_grapheme, editor_viewport,
    grapheme_count,
};
use super::model::{PromptMotion, PromptState};

#[test]
fn tui_editor_handles_graphemes_multiline_undo_and_viewport() {
    let mut prompt = PromptState::default();
    assert!(apply_editor_command(
        &mut prompt,
        EditorCommand::Insert("中👨‍👩‍👧‍👦e\u{301}\r\nsecond".to_string()),
    ));
    assert_eq!(prompt.value, "中👨‍👩‍👧‍👦e\u{301}\nsecond");
    assert_eq!(grapheme_count(&prompt.value), 10);
    assert_eq!(prompt.cursor.char_index, 10);
    assert_eq!(
        byte_index_for_grapheme(&prompt.value, 2),
        Some("中👨‍👩‍👧‍👦".len()),
        "ZWJ emoji is one cursor unit"
    );

    for _ in 0..7 {
        apply_editor_command(
            &mut prompt,
            EditorCommand::Move { motion: PromptMotion::Left, select: false },
        );
    }
    assert_eq!(prompt.cursor.char_index, 3, "cursor stops after the combining grapheme");
    apply_editor_command(
        &mut prompt,
        EditorCommand::Move { motion: PromptMotion::Left, select: true },
    );
    assert_eq!(prompt.editor.selection.expect("selection").ordered(), (2, 3));
    apply_editor_command(&mut prompt, EditorCommand::Backspace);
    assert_eq!(prompt.value, "中👨‍👩‍👧‍👦\nsecond");
    assert!(prompt.value.is_char_boundary(prompt.value.len()));

    assert!(apply_editor_command(&mut prompt, EditorCommand::Undo));
    assert_eq!(prompt.value, "中👨‍👩‍👧‍👦e\u{301}\nsecond");
    assert!(apply_editor_command(&mut prompt, EditorCommand::Redo));
    assert_eq!(prompt.value, "中👨‍👩‍👧‍👦\nsecond");

    let mut merged = PromptState::default();
    apply_editor_command(&mut merged, EditorCommand::Insert("你".to_string()));
    apply_editor_command(&mut merged, EditorCommand::Insert("好".to_string()));
    assert_eq!(merged.editor.undo_depth(), 1, "adjacent inserts merge into one undo group");
    apply_editor_command(&mut merged, EditorCommand::Undo);
    assert!(merged.value.is_empty());
    assert_eq!(merged.editor.redo_depth(), 1);
    apply_editor_command(&mut merged, EditorCommand::Redo);
    assert_eq!(merged.value, "你好");

    let mut multiline = PromptState::new("第一行\nwide🙂line\nthird\nfourth");
    apply_editor_command(&mut multiline, EditorCommand::ViewportHeight(2));
    assert_eq!(editor_viewport(&multiline).top_line, 2);
    apply_editor_command(
        &mut multiline,
        EditorCommand::Move { motion: PromptMotion::Home, select: false },
    );
    let viewport = editor_viewport(&multiline);
    assert_eq!(viewport.cursor_row, 1);
    assert_eq!(viewport.cursor_column, 0);
    assert_eq!(viewport.visible_lines, vec!["third", "fourth"]);

    apply_editor_command(&mut multiline, EditorCommand::KillLine);
    assert_eq!(multiline.editor.kill_buffer, "fourth");
    assert!(!multiline.value.ends_with("fourth"));
    apply_editor_command(&mut multiline, EditorCommand::Yank);
    assert!(multiline.value.ends_with("fourth"));

    let before = multiline.value.clone();
    multiline.cursor.char_index = usize::MAX;
    assert!(!apply_editor_command(
        &mut multiline,
        EditorCommand::Insert("unsafe".to_string()),
    ));
    assert_eq!(multiline.value, before, "invalid offsets are rejected without mutation");
}

#[test]
fn selection_replacement_and_deletion_each_create_one_undo_snapshot() {
    let original = "中e\u{301}🙂\n下一行";

    let mut inserted = PromptState::new(original);
    apply_editor_command(&mut inserted, EditorCommand::SelectAll);
    assert!(apply_editor_command(
        &mut inserted,
        EditorCommand::Insert("替".to_string()),
    ));
    assert_eq!(inserted.value, "替");
    assert_eq!(inserted.cursor.char_index, 1);
    assert_eq!(inserted.editor.undo_depth(), 1);
    assert!(apply_editor_command(&mut inserted, EditorCommand::Undo));
    assert_eq!(inserted.value, original);
    assert_eq!(inserted.editor.selection.expect("restored selection").ordered(), (0, 7));

    let mut backspaced = PromptState::new(original);
    apply_editor_command(&mut backspaced, EditorCommand::SelectAll);
    assert!(apply_editor_command(&mut backspaced, EditorCommand::Backspace));
    assert!(backspaced.value.is_empty());
    assert_eq!(backspaced.cursor.char_index, 0);
    assert_eq!(backspaced.editor.undo_depth(), 1);
    assert!(apply_editor_command(&mut backspaced, EditorCommand::Undo));
    assert_eq!(backspaced.value, original);
    assert_eq!(backspaced.editor.selection.expect("restored selection").ordered(), (0, 7));

    let mut deleted = PromptState::new(original);
    apply_editor_command(&mut deleted, EditorCommand::SelectAll);
    assert!(apply_editor_command(&mut deleted, EditorCommand::Delete));
    assert!(deleted.value.is_empty());
    assert_eq!(deleted.cursor.char_index, 0);
    assert_eq!(deleted.editor.undo_depth(), 1);
    assert!(apply_editor_command(&mut deleted, EditorCommand::Undo));
    assert_eq!(deleted.value, original);
    assert_eq!(deleted.editor.selection.expect("restored selection").ordered(), (0, 7));
}
