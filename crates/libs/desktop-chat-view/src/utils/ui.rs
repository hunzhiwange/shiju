//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。


use desktop_ui_controls::text_editor_context_menu::TextContextAction;

use iced::widget::{container, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Theme, Length}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Theme, Length};


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
pub fn copy_tooltip_content<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    container(text(label).size(11)).padding([4, 8]).style(copy_tooltip_style).into()
}

pub fn copy_tooltip_style(_theme: &Theme) -> iced::widget::container::Style {
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
pub use desktop_chat_tools::card_style::chat_context_target_key;

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
pub fn chat_context_menu<'a>(
    i18n: desktop_i18n::I18n,
    is_open: bool,
) -> Option<Element<'a, TextContextAction>> {
    if !is_open {
        return None;
    }

    Some(text_context_menu(i18n, |action| action))
}

pub use crate::message_view::selection_menu::{text_context_menu, chat_context_menu_button_style, chat_context_menu_style};

pub use crate::presentation::bold_font;

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
