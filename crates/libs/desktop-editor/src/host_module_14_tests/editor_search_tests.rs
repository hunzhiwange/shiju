use super::fixture::{Fixture, update};
use crate::components::editor::Editor;
use crate::message::editor::{EditorMessage, EditorPopover};
use crate::preview::PreviewTab;
use iced::widget::Id;
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
fn ecp06_search_closes_surrounding_panels() {
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    app.show_preview_settings = true;
    app.editor_popover = Some(EditorPopover::Help);
    let _ = update(&mut app, EditorMessage::OpenSearch);
    assert!(!app.show_preview_settings);
    assert_eq!(app.editor_popover, None);
}

fn send(app: &mut Fixture, event: iced_code_editor::Message) {
    let _ = app.send(event);
}

#[test]
fn ecp06_search_replace_updates_only_active_document() {
    use iced_code_editor::Message as E;
    let mut app = Fixture::default();
    app.preview_auto_save_mode = config_ui::PreviewAutoSaveMode::Off;
    app.preview_tabs = vec![tab("/tmp/a.rs", None), tab("/tmp/b.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = update(&mut app, EditorMessage::OpenReplace);
    send(&mut app, E::SearchQueryChanged("main".into()));
    send(&mut app, E::ReplaceQueryChanged("start".into()));
    send(&mut app, E::ReplaceNext);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn start() {}");
    assert_eq!(app.preview_tabs[1].editor.content(), "fn main() {}");
    assert!(app.preview_tabs[0].is_dirty);
    assert!(app.preview_tabs[0].auto_save_revision > 0);
    send(&mut app, E::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    send(&mut app, E::SearchQueryChanged("FN".into()));
    send(&mut app, E::ToggleCaseSensitive);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    send(&mut app, E::ToggleCaseSensitive);
    send(&mut app, E::FindNext);
    send(&mut app, E::FindPrevious);
    send(&mut app, E::SearchDialogTab);
    send(&mut app, E::SearchDialogShiftTab);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "start main() {}");
    send(&mut app, E::Undo);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    let _ = update(&mut app, EditorMessage::CloseSearch);
}

#[test]
fn ecp06_read_only_blocks_replace() {
    use iced_code_editor::Message as E;
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.preview_tabs[0].read_only = true;
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = update(&mut app, EditorMessage::OpenReplace);
    send(&mut app, E::OpenSearchReplace);
    send(&mut app, E::SearchQueryChanged("main".into()));
    send(&mut app, E::ReplaceQueryChanged("changed".into()));
    send(&mut app, E::ReplaceNext);
    send(&mut app, E::ReplaceAll);
    assert_eq!(app.preview_tabs[0].editor.content(), "fn main() {}");
    assert!(!app.preview_tabs[0].is_dirty);
    assert_eq!(app.preview_tabs[0].auto_save_revision, 0);
}
