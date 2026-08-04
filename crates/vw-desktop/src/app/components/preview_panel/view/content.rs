//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

#[cfg(not(target_arch = "wasm32"))]
use crate::app::components::preview_panel::lsp_overlay;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};
/// 重新导出 use iced::widget::image::{Handle as ImageHandle, Image}，让上层模块通过稳定路径访问。
use iced::widget::image::{Handle as ImageHandle, Image};
/// 重新导出 use iced::widget::mouse_area，让上层模块通过稳定路径访问。
#[cfg(not(target_arch = "wasm32"))]
use iced::widget::mouse_area;
/// 重新导出 use iced::widget::svg::{Handle as SvgHandle, Svg}，让上层模块通过稳定路径访问。
use iced::widget::svg::{Handle as SvgHandle, Svg};
/// 重新导出 use iced::widget::{container, scrollable, text}，让上层模块通过稳定路径访问。
use iced::widget::{container, scrollable, text};
/// 重新导出 use iced::{Element, Length}，让上层模块通过稳定路径访问。
use iced::{Element, Length};

/// 构建 content base 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn build_content_base(app: &App) -> Element<'_, Message> {
    if let Some(path) = app.active_preview_path.as_deref() {
        if let Some(tab) = app.preview_tabs.iter().find(|t| t.path == path) {
            let ext = std::path::Path::new(&tab.path)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let is_raster = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");
            let is_svg = ext.as_str() == "svg";
            if is_raster {
                let img = Image::new(ImageHandle::from_path(&tab.path))
                    .width(Length::Fill)
                    .height(Length::Fill);
                scrollable(container(img).width(Length::Fill))
                    .height(Length::Fill)
                    .id(tab.scroll_id.clone())
                    .into()
            } else if is_svg {
                let svg = Svg::new(SvgHandle::from_path(&tab.path))
                    .width(Length::Fill)
                    .height(Length::Fill);
                scrollable(container(svg).width(Length::Fill))
                    .height(Length::Fill)
                    .id(tab.scroll_id.clone())
                    .into()
            } else {
                let editor_view = tab
                    .editor
                    .content_view(|e| Message::Preview(message::PreviewMessage::EditorEvent(e)));

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let overlay = lsp_overlay::lsp_overlay(app);
                    let editor_stack = iced::widget::stack![editor_view, overlay];
                    let editor_stack = mouse_area(editor_stack)
                        .on_enter(Message::Preview(message::PreviewMessage::EditorMouseEntered))
                        .on_exit(Message::Preview(message::PreviewMessage::EditorMouseExited));
                    container(editor_stack).width(Length::Fill).height(Length::Fill).into()
                }

                #[cfg(target_arch = "wasm32")]
                {
                    container(editor_view).width(Length::Fill).height(Length::Fill).into()
                }
            }
        } else {
            container(text("未找到预览")).width(Length::Fill).height(Length::Fill).into()
        }
    } else if matches!(app.screen, crate::app::Screen::Preview) {
        container(text("未选择文件"))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else if !app.file_manager_show_changes {
        container(text("未选择文件"))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else {
        // crate 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        crate::app::components::git_panel::view(app)
    }
}
