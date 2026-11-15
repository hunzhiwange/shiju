//! LSP 覆盖层的桌面消息装配。
#[cfg(not(target_arch = "wasm32"))]
use crate::app::{App, Message, message::PreviewMessage};

#[cfg(not(target_arch = "wasm32"))]
/// 将编辑器叠加层消息映射为应用预览消息。
pub(super) fn map_lsp_overlay_message(msg: iced_code_editor::LspOverlayMessage) -> Message {
    use iced_code_editor::LspOverlayMessage;

    match msg {
        LspOverlayMessage::HoverEntered => Message::Preview(PreviewMessage::LspHoverEntered),
        LspOverlayMessage::HoverExited => Message::Preview(PreviewMessage::LspHoverExited),
        LspOverlayMessage::CompletionSelected(index) => {
            Message::Preview(PreviewMessage::LspCompletionSelected(index))
        }
        LspOverlayMessage::CompletionClosed => {
            Message::Preview(PreviewMessage::LspCompletionClosed)
        }
        LspOverlayMessage::CompletionNavigateUp => {
            Message::Preview(PreviewMessage::LspCompletionNavigateUp)
        }
        LspOverlayMessage::CompletionNavigateDown => {
            Message::Preview(PreviewMessage::LspCompletionNavigateDown)
        }
        LspOverlayMessage::CompletionConfirm => {
            Message::Preview(PreviewMessage::LspCompletionConfirm)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lsp_overlay(app: &App) -> iced::Element<'_, Message> {
    let editor = app.active_preview_path.as_ref().and_then(|path| {
        app.preview_tabs.iter().find(|tab| &tab.path == path).map(|tab| &tab.editor.inner)
    });
    desktop_editor::components::preview_panel::lsp_overlay::lsp_overlay(
        app.active_preview_path.as_deref(), app.lsp_overlay_path.as_deref(), editor,
        &app.lsp_overlay, app.effective_editor_theme_ref(), app.current_font_size, app.current_line_height,
    ).map(map_lsp_overlay_message)
}
