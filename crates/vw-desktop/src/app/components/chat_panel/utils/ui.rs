//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;
/// 重新导出 use crate::app::message::ChatMessage，让上层模块通过稳定路径访问。
use crate::app::message::ChatMessage;
use iced::widget::{button, column, container, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length, Theme};

/// 重新导出 use super::theme::is_dark_theme，让上层模块通过稳定路径访问。
use super::theme::is_dark_theme;

/// 处理 copy tooltip content 对应的局部职责。
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
pub fn copy_tooltip_content<'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(11)).padding([4, 8]).style(copy_tooltip_style).into()
}

pub(super) fn copy_tooltip_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: Some(Color::from_rgb8(0xF3, 0xF4, 0xF6)),
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(Color::from_rgb8(0x0B, 0x0B, 0x0B))),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border {
            // width 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            width: 1.0,
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::from_rgb8(0x14, 0x14, 0x14),
            // radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            radius: 6.0.into(),
        },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(0.25),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: iced::Vector::new(0.0, 2.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 6.0,
        },
        ..Default::default()
    }
}

/// 处理 chat context target key 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn chat_context_target_key(msg_idx: usize, sub_idx: Option<usize>) -> u64 {
    match sub_idx {
        Some(sub_idx) => ((msg_idx as u64) << 32) | ((sub_idx as u64) + 1),
        None => (msg_idx as u64) << 32,
    }
}

/// 处理 chat context menu 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn chat_context_menu<'a>(is_open: bool) -> Option<Element<'a, Message>> {
    if !is_open {
        return None;
    }

    let menu_item = |label: &'static str, message: Message| {
        button(
            container(text(label).size(12))
                .width(Length::Fill)
                .padding([5, 9])
                .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(chat_context_menu_button_style)
        .on_press(message)
    };

    let menu = container(
        column![
            menu_item("复制", Message::Chat(ChatMessage::CopyContextMenuText)),
            menu_item("添加到对话", Message::Chat(ChatMessage::AppendContextMenuText)),
            menu_item("百度搜索", Message::Chat(ChatMessage::SearchContextMenuWithBaidu)),
            menu_item("Google 搜索", Message::Chat(ChatMessage::SearchContextMenuWithGoogle)),
            menu_item("Bing 搜索", Message::Chat(ChatMessage::SearchContextMenuWithBing)),
        ]
        .spacing(0)
        .width(Length::Fixed(134.0)),
    )
    .padding([3, 3])
    .style(chat_context_menu_style);

    Some(menu.into())
}

pub(super) fn chat_context_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let hovered_bg = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xEA, 0xED, 0xF1, 0.85)
    };
    let pressed_bg = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xE2, 0xE6, 0xEB, 0.95)
    };

    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: match status {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Hovered => Some(Background::Color(hovered_bg)),
            _ => None,
        },
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub(super) fn chat_context_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let background = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0x22, 0x22, 0x24, 0.98)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xF3, 0xF4, 0xF6, 0.98)
    };
    let border = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0x3A, 0x3A, 0x3F, 0.95)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xDD, 0xE1, 0xE6, 1.0)
    };

    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(background)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border, radius: 8.0.into() },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.20 } else { 0.08 }),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: iced::Vector::new(0.0, 4.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

/// 处理 bold font 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn bold_font() -> iced::Font {
    // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }
}

/// 处理 capped scroll height 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn capped_scroll_height(text: &str, max_height: f32) -> Length {
    /// THINK_LINE_HEIGHT 是当前模块共享的固定参数。
    const THINK_LINE_HEIGHT: f32 = 18.0;
    /// THINK_VERTICAL_PADDING 是当前模块共享的固定参数。
    const THINK_VERTICAL_PADDING: f32 = 16.0;
    let lines = text.lines().count().max(1) as f32;
    let estimated = (lines * THINK_LINE_HEIGHT) + THINK_VERTICAL_PADDING;
    if estimated >= max_height { Length::Fixed(max_height) } else { Length::Shrink }
}
