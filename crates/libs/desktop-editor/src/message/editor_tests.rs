use super::{EditorMessage, update as domain_update};
use crate::components::editor::Editor;
use iced_code_editor::{ArrowDirection, Message as CodeEditorMessage};
fn update(editor: &mut Editor, message: EditorMessage) -> iced::Task<super::Event> {
    domain_update(Some(editor), &mut None, &mut false, message)
}
#[test]
fn cut_and_delete_remove_the_active_editor_selection() {
    for message in [EditorMessage::Cut, EditorMessage::Delete] {
        let mut app = Editor::new("abcd", "text");
        let editor = &mut app.inner;
        let _ = editor.update(&CodeEditorMessage::GotoPosition(0, 0));
        let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));
        let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));

        let _ = update(&mut app, message);

        assert_eq!(app.content(), "cd");
    }
}

#[test]
fn copy_preserves_the_active_editor_content() {
    let mut app = Editor::new("copy me", "text");
    let editor = &mut app.inner;
    let _ = editor.update(&CodeEditorMessage::GotoPosition(0, 0));
    let _ = editor.update(&CodeEditorMessage::ArrowKey(ArrowDirection::Right, true));

    let _ = update(&mut app, EditorMessage::Copy);

    assert_eq!(app.content(), "copy me");
}

#[test]
fn paste_request_and_empty_clipboard_result_preserve_editor_content() {
    let mut app = Editor::new("keep", "text");

    let _ = update(&mut app, EditorMessage::Paste);
    let _ = update(&mut app, EditorMessage::ClipboardContentReceived(None));

    assert_eq!(app.content(), "keep");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn search_dialog_actions_preserve_editor_content() {
    let mut app = Editor::new("find me", "text");

    let _ = update(&mut app, EditorMessage::OpenSearch);
    assert_eq!(app.content(), "find me");

    let _ = update(&mut app, EditorMessage::OpenReplace);
    assert_eq!(app.content(), "find me");

    let _ = update(&mut app, EditorMessage::CloseSearch);
    assert_eq!(app.content(), "find me");
}
