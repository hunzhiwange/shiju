//! 预览面板视图组件。
//!
//! 本模块负责预览内容、菜单、面包屑、LSP 标识或浮层宿主的局部构建。

use super::ContentMessage as Message;
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
pub fn build_content_base<'a, External: Clone + 'static>(tab: Option<ContentTab<'a>>, active_path: bool, i18n: desktop_i18n::I18n, empty: Element<'a, Message<External>>, #[cfg(not(target_arch = "wasm32"))] overlay: Element<'a, Message<External>>) -> Element<'a, Message<External>> {
    if active_path {
        if let Some(tab) = tab {
            let ext = std::path::Path::new(tab.path)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let is_raster = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");
            let is_svg = ext.as_str() == "svg";
            if is_raster {
                let img = Image::new(ImageHandle::from_path(tab.path))
                    .width(Length::Fill)
                    .height(Length::Fill);
                scrollable(container(img).width(Length::Fill))
                    .height(Length::Fill)
                    .id(tab.scroll_id)
                    .into()
            } else if is_svg {
                let svg = Svg::new(SvgHandle::from_path(tab.path))
                    .width(Length::Fill)
                    .height(Length::Fill);
                scrollable(container(svg).width(Length::Fill))
                    .height(Length::Fill)
                    .id(tab.scroll_id)
                    .into()
            } else {
                if let Some(message) = tab.message {
                    return container(text(message))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
                }
                let editor_view = tab
                    .editor
                    .content_view(|e| Message::EditorEvent(e));

                let editor_view: Element<'_, Message<External>> = if tab.truncated {
                    iced::widget::column![
                        text(
                            i18n
                                .__("文件较大，预览已截断。")
                        ),
                        editor_view
                    ]
                    .into()
                } else {
                    editor_view
                };
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let editor_stack = iced::widget::stack![editor_view, overlay];
                    let editor_stack = mouse_area(editor_stack)
                        .on_enter(Message::MouseEntered)
                        .on_exit(Message::MouseExited);
                    container(editor_stack).width(Length::Fill).height(Length::Fill).into()
                }

                #[cfg(target_arch = "wasm32")]
                {
                    container(editor_view).width(Length::Fill).height(Length::Fill).into()
                }
            }
        } else {
            container(text(i18n.__("未找到预览")))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    } else { empty }
}

pub struct ContentTab<'a> {
    pub path: &'a str,
    pub editor: &'a crate::components::editor::Editor,
    pub scroll_id: iced::widget::Id,
    pub truncated: bool,
    pub message: Option<String>,
}
