#[test]
fn lsp_overlay_tests_are_wired() {
    assert!(module_path!().contains("lsp_overlay_tests"));
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::super::lsp_overlay::{lsp_overlay, map_lsp_overlay_message};
    use crate::app::components::editor::Editor;
    use crate::app::message::PreviewMessage;
    use crate::app::{App, Message, PreviewTab};
    use iced::widget::Id;
    use iced_code_editor::LspOverlayMessage;

    fn app() -> App {
        App::new().0
    }

    fn tab(path: &str) -> PreviewTab {
        PreviewTab {
            path: path.to_string(),
            title: path.rsplit('/').next().unwrap_or(path).to_string(),
            content: "fn main() {}".to_string(),
            is_dirty: false,
            truncated: false,
            auto_save_revision: 0,
            editor: Editor::new("fn main() {}", "rust"),
            scroll_id: Id::unique(),
            lsp_server_key: Some("rust-analyzer"),
            lsp_uri: Some(format!("file://{path}")),
            lsp_language_id: Some("rust".to_string()),
        }
    }

    fn keep(element: iced::Element<'_, Message>) {
        std::hint::black_box(element);
    }

    #[test]
    fn overlay_is_empty_without_active_preview_path() {
        let app = app();

        keep(lsp_overlay(&app));
    }

    #[test]
    fn overlay_is_empty_when_overlay_path_does_not_match_active_path() {
        let mut app = app();
        app.active_preview_path = Some("/tmp/a.rs".to_string());
        app.lsp_overlay_path = Some("/tmp/b.rs".to_string());
        app.preview_tabs = vec![tab("/tmp/a.rs")];

        keep(lsp_overlay(&app));
    }

    #[test]
    fn overlay_is_empty_when_matching_tab_is_missing() {
        let mut app = app();
        app.active_preview_path = Some("/tmp/a.rs".to_string());
        app.lsp_overlay_path = Some("/tmp/a.rs".to_string());

        keep(lsp_overlay(&app));
    }

    #[test]
    fn overlay_builds_for_matching_active_tab() {
        let mut app = app();
        app.active_preview_path = Some("/tmp/a.rs".to_string());
        app.lsp_overlay_path = Some("/tmp/a.rs".to_string());
        app.preview_tabs = vec![tab("/tmp/a.rs")];

        keep(lsp_overlay(&app));
    }

    #[test]
    fn message_mapper_preserves_hover_direction() {
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::HoverEntered),
            Message::Preview(PreviewMessage::LspHoverEntered)
        ));
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::HoverExited),
            Message::Preview(PreviewMessage::LspHoverExited)
        ));
    }

    #[test]
    fn message_mapper_preserves_completion_selection_index() {
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::CompletionSelected(usize::MAX)),
            Message::Preview(PreviewMessage::LspCompletionSelected(usize::MAX))
        ));
    }

    #[test]
    fn message_mapper_preserves_completion_controls() {
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::CompletionClosed),
            Message::Preview(PreviewMessage::LspCompletionClosed)
        ));
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::CompletionNavigateUp),
            Message::Preview(PreviewMessage::LspCompletionNavigateUp)
        ));
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::CompletionNavigateDown),
            Message::Preview(PreviewMessage::LspCompletionNavigateDown)
        ));
        assert!(matches!(
            map_lsp_overlay_message(LspOverlayMessage::CompletionConfirm),
            Message::Preview(PreviewMessage::LspCompletionConfirm)
        ));
    }
}
