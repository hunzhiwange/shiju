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
fn ecp01_popovers_close_on_escape_outside_and_tab_switch() {
    let mut app = Fixture::default();
    app.preview_tabs = vec![tab("/tmp/a.rs", None)];
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, Some(EditorPopover::Help));
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, None);
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Settings));
    let _ = update(&mut app, EditorMessage::ClosePopover);

    assert_eq!(app.editor_popover, None);
    for kind in [EditorPopover::Actions, EditorPopover::Settings, EditorPopover::Help] {
        let _ = update(&mut app, EditorMessage::TogglePopover(kind));
        assert_eq!(app.editor_popover, Some(kind));
    }
    app.tab(crate::preview::message::PreviewMessage::Select(String::new()));
    assert_eq!(app.editor_popover, None);
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    assert_eq!(app.editor_popover, None);
    app.active_preview_path = Some("/tmp/a.rs".into());
    let _ = update(&mut app, EditorMessage::TogglePopover(EditorPopover::Help));
    app.tab(crate::preview::message::PreviewMessage::Close("/tmp/a.rs".into()));
    assert_eq!(app.editor_popover, None);
}
