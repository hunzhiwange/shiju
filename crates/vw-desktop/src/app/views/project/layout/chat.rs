//! 聊天区域布局模块
//!
//! 本模块负责构建项目视图中的聊天界面区域，包括聊天面板、待办事项和输入面板的布局。
//! 该模块是项目视图布局系统的核心组成部分，为用户提供交互式聊天体验。

use iced::border::Border;
use iced::widget::{column, container};
use iced::{Background, Element, Length, Theme};

use crate::app::components::{chat_panel, input_panel};
use crate::app::theme_catalog::{ChromeStyleOverrides, resolve_chrome_style_from_skin};
use crate::app::{App, Message};

/// 构建聊天区域的完整布局
///
/// 该函数创建聊天界面区域的完整视图，包含聊天消息面板、待办事项浮动栏、
/// 待办面板和输入面板。布局采用垂直排列，聊天区域占据主要空间，
/// 输入面板根据内容自适应高度。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含聊天消息、待办事项等数据
/// * `spacing` - 垂直布局中各元素之间的间距（像素）
/// * `corner_radius` - 聊天区域圆角半径基值
/// * `chat_content_pad` - 聊天内容区域的内边距（像素）
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，代表完整的聊天区域视图，可直接嵌入到更大的布局中
///
/// # 布局结构
///
/// ```text
/// ┌─────────────────────────────────┐
/// │  聊天视图区域 (chat_view)        │
/// │  包含聊天消息面板                │
/// │  待办浮动栏 (底部对齐)           │
/// ├─────────────────────────────────┤
/// │  待办面板 (todo_panel)           │
/// ├─────────────────────────────────┤
/// │  输入面板 (input_panel)          │
/// └─────────────────────────────────┘
/// ```
///
/// # 样式特性
///
/// - 背景色使用主题的背景色
/// - 四角使用统一的圆角，避免聊天区左上角出现直角接缝
pub fn chat_area(
    app: &App,
    _spacing: f32,
    corner_radius: f32,
    chat_content_pad: f32,
) -> Element<'_, Message> {
    // 构建聊天消息视图容器
    let chat_view_base = container(chat_panel::view(app))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(chat_content_pad);
    let chat_view: Element<'_, Message> = chat_view_base.into();

    let radius = corner_radius + 4.0;
    let chrome = resolve_chrome_style_from_skin(app.active_theme_skin.as_ref(), &app.app_theme);

    // 组装完整的聊天区域布局
    let chat_area = column![
        chat_view,
        container(input_panel::view(app)).width(Length::Fill).padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 8.0,
            left: 16.0
        })
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    // 应用样式并返回最终布局
    container(chat_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |theme: &Theme| chat_area_style(theme, radius, Some(&chrome)))
        .into()
}

fn chat_area_style(
    theme: &Theme,
    radius: f32,
    chrome: Option<&ChromeStyleOverrides>,
) -> container::Style {
    let palette = theme.extended_palette();
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let default_bg = crate::app::theme_catalog::panel_surface_color(theme, 0.985);
    let background = chrome.and_then(|c| c.panel_bg).unwrap_or(default_bg);
    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.74)
            } else {
                palette.background.strong.color.scale_alpha(0.55)
            },
            radius: iced::border::Radius {
                top_left: radius,
                top_right: radius,
                bottom_right: radius,
                bottom_left: radius,
            },
        },
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
