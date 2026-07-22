// LSP 叠加层组件
//
// 本模块负责渲染和管理 LSP（语言服务器协议）相关的叠加层 UI 元素，
// 包括代码悬停提示、自动补全列表等交互式功能。
//
// # 主要功能
//
// - 悬停提示叠加层：当用户将鼠标悬停在代码元素上时，显示类型信息、文档说明等
// - 代码补全叠加层：显示智能代码补全建议列表，支持键盘导航和选择
// - 路径匹配验证：确保叠加层仅在对应的预览标签页上显示
//
// # 条件编译
//
// 本模块中的功能仅在非 WASM 目标架构上可用，因为 LSP 功能需要原生环境支持。

#[cfg(not(target_arch = "wasm32"))]
use iced::widget::{Space, container};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Element, Length};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::PreviewMessage;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::{App, Message};

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
pub fn lsp_overlay(app: &App) -> Element<'_, Message> {
    // 记录当前 LSP 叠加层路径和活动预览路径的调试信息
    tracing::debug!(
        "[LSP OVERLAY] lsp_overlay_path={:?}, active_preview_path={:?}",
        app.lsp_overlay_path,
        app.active_preview_path
    );

    let Some(path) = app.active_preview_path.as_ref() else {
        tracing::debug!("[LSP OVERLAY] no active_preview_path");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    };

    // 验证路径匹配：确保 LSP 叠加层显示在正确的文件上
    // 如果路径不匹配，返回空的占位元素以避免叠加层显示在错误的文件上
    if app.lsp_overlay_path.as_ref() != Some(path) {
        tracing::debug!("[LSP OVERLAY] path mismatch, returning empty");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    }

    // 在预览标签页集合中查找对应路径的标签页
    // 标签页包含编辑器实例，这是渲染叠加层所必需的上下文
    let Some(tab) = app.preview_tabs.iter().find(|t| &t.path == path) else {
        tracing::debug!("[LSP OVERLAY] tab not found");
        return container(Space::new().width(Length::Shrink).height(Length::Shrink)).into();
    };

    // 记录叠加层状态信息，用于调试悬停和补全功能的渲染
    tracing::debug!(
        "[LSP OVERLAY] calling view_lsp_overlay: hover_visible={}, hover_position={:?}",
        app.lsp_overlay.hover_visible,
        app.lsp_overlay.hover_position
    );

    // 调用 iced_code_editor 库的视图函数构建 LSP 叠加层
    // 参数说明：
    // - &app.lsp_overlay: LSP 叠加层的状态数据
    // - &tab.editor.inner: 底层编辑器实例，用于计算叠加层的位置
    // - &app.app_theme: 主题配置，用于样式化叠加层
    // - app.current_font_size: 字体大小，影响叠加层的尺寸计算
    // - app.current_line_height: 行高，影响叠加层的位置定位
    // - map_lsp_overlay_message：将底层 LSP 消息转换为应用程序消息
    iced_code_editor::view_lsp_overlay(
        &app.lsp_overlay,
        &tab.editor.inner,
        app.effective_editor_theme_ref(),
        app.current_font_size,
        app.current_line_height,
        map_lsp_overlay_message,
    )
}
