use crate::message::editor::EditorOption;
use iced_code_editor::{CodeEditor, IndentStyle, Message};

#[test]
fn ecp04_options_change_only_active_editor() {
    let mut active = CodeEditor::new("hello", "txt");
    let other = CodeEditor::new("world", "txt");
    for option in EditorOption::ALL {
        let previous = option.enabled(&active);
        let untouched = option.enabled(&other);
        option.set(&mut active, !previous);
        assert_eq!(option.enabled(&active), !previous);
        assert_eq!(option.enabled(&other), untouched);
        option.set(&mut active, previous);
        assert_eq!(option.enabled(&active), previous);
    }
    assert_eq!(active.content(), "hello");
}

#[test]
fn ecp04_vim_shortcut_and_menu_share_state() {
    let mut editor = CodeEditor::new("hello", "txt");
    EditorOption::Vim.set(&mut editor, true);
    assert!(format!("{:?}", editor.vim_mode()).contains("Normal"));
    let _ = editor.update(&Message::VimKey('i'));
    assert!(format!("{:?}", editor.vim_mode()).contains("Insert"));
    let _ = editor.update(&Message::ToggleVimMode);
    assert!(!EditorOption::Vim.enabled(&editor));
    assert_eq!(editor.content(), "hello");
}

#[test]
fn ecp04_tab_uses_selected_indent_style() {
    for style in IndentStyle::ALL {
        let mut editor = CodeEditor::new("", "txt");
        editor.set_indent_style(style);
        let _ = editor.update(&Message::Tab);
        let expected = match style {
            IndentStyle::Spaces(n) => " ".repeat(usize::from(n)),
            IndentStyle::Tab => "\t".into(),
        };
        assert_eq!(editor.content(), expected);
    }
}
