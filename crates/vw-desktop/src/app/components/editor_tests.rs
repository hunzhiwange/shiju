use super::editor::{
    APP_CONTEXT_MENU_INSERT_SELECTION, APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS, Editor,
};
use iced::Theme;
use iced_code_editor::{ContextMenuEntry, i18n::Language};

#[test]
fn editor_starts_with_initial_content() {
    let editor = Editor::new("fn main() {}", "rust");

    assert_eq!(editor.content(), "fn main() {}");
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
}

#[test]
fn editor_setters_are_callable() {
    let mut editor = Editor::new("", "rust");

    editor.set_font(iced::Font::MONOSPACE);
    editor.set_font_size(14.0);
    editor.set_line_height(1.4);
    editor.set_theme(Theme::Dark);
    editor.set_ui_language(iced_code_editor::i18n::Language::ChineseSimplified);

    assert_eq!(editor.inner.font_size(), 14.0);
    assert_eq!(editor.inner.line_height(), 1.4);
    assert_eq!(editor.inner.language(), iced_code_editor::i18n::Language::ChineseSimplified);
}

#[test]
fn editor_toggles_delegate_to_inner_editor_state() {
    let mut editor = Editor::new("", "rust");

    editor.set_line_numbers_enabled(false);
    editor.set_search_replace_enabled(false);

    assert!(!editor.inner.line_numbers_enabled());
    assert!(!editor.inner.search_replace_enabled());

    editor.set_line_numbers_enabled(true);
    editor.set_search_replace_enabled(true);

    assert!(editor.inner.line_numbers_enabled());
    assert!(editor.inner.search_replace_enabled());
}

#[test]
fn editor_search_dialog_tasks_are_constructible() {
    let mut editor = Editor::new("needle", "text");

    let _ = editor.open_search_dialog();
    let _ = editor.open_search_replace_dialog();
    let _ = editor.close_search_dialog();
}

#[test]
fn editor_content_view_is_constructible() {
    let editor = Editor::new("", "rust");

    let _ = editor.content_view(|_| ());
}

#[test]
fn preview_context_menu_uses_stable_actions_default_entries_and_localized_labels() {
    let mut editor = Editor::new("", "rust");

    editor.configure_preview_context_menu(Language::English, true);

    assert!(editor.inner.default_context_menu_enabled());
    assert!(editor.inner.reveal_in_file_manager_enabled());
    assert_eq!(
        editor.inner.custom_context_menu_entries(),
        &[
            ContextMenuEntry::item(APP_CONTEXT_MENU_INSERT_SELECTION, "Insert selection into chat"),
            ContextMenuEntry::item(
                APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS,
                "Insert selection positions",
            ),
        ]
    );

    editor.set_ui_language(Language::ChineseSimplified);

    assert_eq!(
        editor.inner.custom_context_menu_entries(),
        &[
            ContextMenuEntry::item(APP_CONTEXT_MENU_INSERT_SELECTION, "插入选择到聊天"),
            ContextMenuEntry::item(APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS, "插入选区位置信息"),
        ]
    );
}

#[cfg(not(target_arch = "wasm32"))]
mod lsp_tests {
    use super::Editor;
    use iced_code_editor::{LspClient, LspDocument, LspPosition, LspTextChange};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct RecordingClient {
        events: Rc<RefCell<Vec<String>>>,
    }

    impl RecordingClient {
        fn new(events: Rc<RefCell<Vec<String>>>) -> Self {
            Self { events }
        }
    }

    impl LspClient for RecordingClient {
        fn did_open(&mut self, document: &LspDocument, text: &str) {
            self.events.borrow_mut().push(format!(
                "open:{}:{}:{}:{}",
                document.uri, document.language_id, document.version, text
            ));
        }

        fn did_change(&mut self, document: &LspDocument, changes: &[LspTextChange]) {
            self.events.borrow_mut().push(format!("change:{}:{}", document.uri, changes.len()));
        }

        fn did_save(&mut self, document: &LspDocument, text: &str) {
            self.events.borrow_mut().push(format!("save:{}:{}", document.uri, text));
        }

        fn did_close(&mut self, document: &LspDocument) {
            self.events.borrow_mut().push(format!("close:{}", document.uri));
        }

        fn request_hover(&mut self, document: &LspDocument, position: LspPosition) {
            self.events
                .borrow_mut()
                .push(format!("hover:{}:{}:{}", document.uri, position.line, position.character));
        }

        fn request_completion(&mut self, document: &LspDocument, position: LspPosition) {
            self.events.borrow_mut().push(format!(
                "completion:{}:{}:{}",
                document.uri, position.line, position.character
            ));
        }

        fn request_definition(&mut self, document: &LspDocument, position: LspPosition) {
            self.events.borrow_mut().push(format!(
                "definition:{}:{}:{}",
                document.uri, position.line, position.character
            ));
        }
    }

    #[test]
    fn lsp_state_starts_detached_and_returns_false_without_client() {
        let mut editor = Editor::new("fn main() {}", "rust");

        assert!(!editor.has_lsp_attached());
        assert!(!editor.lsp_request_hover_at_position(LspPosition { line: 0, character: 0 }));
        assert!(!editor.lsp_request_definition_at(iced::Point::new(0.0, 0.0)));
        assert_eq!(editor.cursor_position(), (0, 0));

        let _ = editor.cursor_screen_position();
        let _ = editor.lsp_hover_anchor_at_point(iced::Point::new(0.0, 0.0));
    }

    #[test]
    fn lsp_attach_open_save_request_and_detach_delegate_to_client() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut editor = Editor::new("alpha", "rust");

        editor.attach_lsp(
            Box::new(RecordingClient::new(Rc::clone(&events))),
            LspDocument::new("file:///first.rs", "rust"),
        );

        assert!(editor.has_lsp_attached());

        editor.lsp_did_save();
        assert!(editor.lsp_request_hover_at_position(LspPosition { line: 2, character: 3 }));
        editor.lsp_request_completion();
        editor.set_lsp_auto_flush(false);
        editor.lsp_flush_pending_changes();
        editor.lsp_open_document(LspDocument::new("file:///second.rs", "rust"));
        editor.detach_lsp();

        assert!(!editor.has_lsp_attached());
        let expected = vec![
            "open:file:///first.rs:rust:1:alpha".to_string(),
            "save:file:///first.rs:alpha".to_string(),
            "hover:file:///first.rs:2:3".to_string(),
            "completion:file:///first.rs:0:0".to_string(),
            "close:file:///first.rs".to_string(),
            "open:file:///second.rs:rust:1:alpha".to_string(),
            "close:file:///second.rs".to_string(),
        ];
        assert_eq!(events.borrow().clone(), expected);
    }
}
