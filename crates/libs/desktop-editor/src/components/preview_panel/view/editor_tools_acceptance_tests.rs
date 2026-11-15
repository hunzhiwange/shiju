#[test]
fn ecp07_chinese_input_and_undo_preserve_document() {
    use iced_code_editor::{CodeEditor, Message};
    let mut editor = CodeEditor::new("", "txt");
    let _ = editor.update(&Message::Paste("中文输入，编辑器".into()));
    assert_eq!(editor.content(), "中文输入，编辑器");
    let _ = editor.update(&Message::Undo);
    assert_eq!(editor.content(), "");
}
