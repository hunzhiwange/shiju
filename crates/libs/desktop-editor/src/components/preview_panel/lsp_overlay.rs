//! 基于匹配路径和编辑器引用构建原生 LSP 叠加层。
#![cfg(not(target_arch = "wasm32"))]
use iced::widget::{Space, container};
use iced::{Element, Length, Theme};
use iced_code_editor::{CodeEditor, LspOverlayMessage, LspOverlayState};

pub fn lsp_overlay<'a>(
    active_path: Option<&str>, overlay_path: Option<&str>, editor: Option<&'a CodeEditor>,
    overlay: &'a LspOverlayState, theme: &'a Theme, font_size: f32, line_height: f32,
) -> Element<'a, LspOverlayMessage> {
    tracing::debug!("[LSP OVERLAY] lsp_overlay_path={:?}, active_preview_path={:?}", overlay_path, active_path);
    let Some(path) = active_path else {
        tracing::debug!("[LSP OVERLAY] no active_preview_path");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    };
    if overlay_path != Some(path) {
        tracing::debug!("[LSP OVERLAY] path mismatch, returning empty");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    }
    let Some(editor) = editor else {
        tracing::debug!("[LSP OVERLAY] tab not found");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    };
    tracing::debug!("[LSP OVERLAY] calling view_lsp_overlay: hover_visible={}, hover_position={:?}", overlay.hover_visible, overlay.hover_position);
    iced_code_editor::view_lsp_overlay(overlay, editor, theme, font_size, line_height, |message| message)
}
