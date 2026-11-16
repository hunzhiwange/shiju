use super::fixture::Fixture;
use crate::components::editor::Editor;
use crate::preview::PreviewTab;
use iced::widget::Id;
use iced_code_editor::Message as Event;

fn send(app: &mut Fixture, event: Event) {
    let _ = app.send(event);
}
fn fixture() -> Fixture {
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    app.preview_auto_save_mode = config_ui::PreviewAutoSaveMode::Off;
    app
}
#[test]
fn ecp02_menu_action_changes_only_active_document() {
    let mut app = fixture();
    let actions =
        crate::components::preview_panel::view::editor_actions::actions(false, false, true);
    assert!(actions.iter().any(|e| matches!(e, Event::DuplicateLineDown)));
    send(&mut app, Event::DuplicateLineDown);
    assert_ne!(app.preview_tabs[0].editor.content(), "fn main() {}");
    assert_eq!(app.preview_tabs[1].editor.content(), "fn main() {}");
}
#[test]
fn ecp02_mutating_actions_respect_read_only() {
    let mut app = fixture();
    app.preview_tabs[0].read_only = true;
    for event in
        [Event::DuplicateLineDown, Event::MoveLineDown, Event::ToggleComment, Event::VimKey('x')]
    {
        send(&mut app, event);
        assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
        assert!(!app.preview_tabs[0].is_dirty);
    }
    assert!(
        !crate::components::preview_panel::view::editor_actions::actions(true, false, true)
            .iter()
            .any(|e| matches!(e, Event::DuplicateLineDown))
    );
}
#[test]
fn ecp02_line_action_updates_dirty_revision_and_undo() {
    let mut app = fixture();
    send(&mut app, Event::DuplicateLineDown);
    assert!(app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 1);
    send(&mut app, Event::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
}
fn tab(path: &str, server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: path.rsplit('/').next().unwrap_or(path).to_string(),
        content: "fn main() {}".to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: Editor::new("fn main() {}", "rust"),
        scroll_id: Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: server_key,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: Some(format!("file://{path}")),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: Some("rust".to_string()),
    }
}

#[test]
fn ecp02_selection_multicursor_folding_and_menu_visibility() {
    let mut app = fixture();
    app.preview_tabs[0].editor = Editor::new("fn a() {\n    foo();\n}\nfoo();", "rust");
    app.preview_tabs[0].editor.inner.request_focus();
    send(&mut app, Event::CanvasFocusGained);
    send(&mut app, Event::SelectAll);
    assert_eq!(
        app.preview_tabs[0].editor.inner.selected_text().as_deref(),
        Some("fn a() {\n    foo();\n}\nfoo();")
    );
    send(&mut app, Event::GotoPosition(1, 4));
    send(&mut app, Event::SelectNextOccurrence);
    assert_eq!(app.preview_tabs[0].editor.inner.selected_text().as_deref(), Some("foo"));
    send(&mut app, Event::GotoPosition(0, 0));
    send(&mut app, Event::AddCursorBelow);
    send(&mut app, Event::CharacterInput('x'));
    assert_eq!(app.preview_tabs[0].editor.content().matches('x').count(), 2);
    send(&mut app, Event::Undo);
    send(&mut app, Event::FoldAll);
    assert!(app.preview_tabs[0].editor.inner.is_folded(0));
    send(&mut app, Event::UnfoldAll);
    assert!(!app.preview_tabs[0].editor.inner.is_folded(0));
    let hidden =
        crate::components::preview_panel::view::editor_actions::actions(false, true, false);
    assert!(!hidden.iter().any(|e| matches!(
        e,
        Event::AddCursorAbove
            | Event::AddCursorBelow
            | Event::SelectNextOccurrence
            | Event::FoldAll
    )));
    assert_eq!(app.preview_tabs[1].editor.content(), "fn main() {}");
}
#[test]
fn ecp02_move_comment_close_menu_and_resume_editing() {
    let mut app = fixture();
    app.preview_tabs[0].editor = Editor::new("first\nsecond", "rust");
    app.editor_popover = Some(crate::message::editor::EditorPopover::Actions);
    send(&mut app, Event::MoveLineDown);
    assert_eq!(app.preview_tabs[0].editor.content(), "second\nfirst");
    assert_eq!(app.editor_popover, None);
    send(&mut app, Event::ToggleComment);
    assert!(app.preview_tabs[0].editor.content().contains("//"));
    send(&mut app, Event::CharacterInput('x'));
    assert!(app.preview_tabs[0].editor.content().contains('x'));
    assert_eq!(app.preview_tabs[1].auto_save_revision, 0);
}

#[tokio::test]
async fn ecp02_line_action_schedules_existing_auto_save_revision() {
    use iced_runtime::futures::futures::StreamExt;
    let mut app = fixture();
    app.preview_auto_save_mode = config_ui::PreviewAutoSaveMode::AfterDelay;
    let task = app.send(Event::DuplicateLineDown);
    let mut stream = iced_runtime::task::into_stream(task).expect("auto save delay task");
    let mut scheduled = false;
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(crate::preview::editor::Effect::AutoSave {
            path,
            revision,
        }) = action
        {
            assert_eq!(path, "/tmp/a.rs");
            assert_eq!(revision, app.preview_tabs[0].auto_save_revision);
            scheduled = true;
        }
    }
    assert!(scheduled);
}
