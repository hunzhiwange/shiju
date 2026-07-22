//! Git 面板通用按钮组件。
//!
//! 本模块封装 Git 面板中复用的图标、字形按钮、禁用态和提示样式。

use iced::Element;
/// 重新导出 use iced::widget::{button, text}，让上层模块通过稳定路径访问。
use iced::widget::{button, text};
/// 重新导出 use iced::{Background, Border, Color, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Theme};

/// 重新导出 use crate::app::Message，让上层模块通过稳定路径访问。
use crate::app::Message;

/// 重新导出 use super::shared::{header_plain_glyph_button_style, subtle_button_style, with_tooltip}，让上层模块通过稳定路径访问。
use super::shared::{header_plain_glyph_button_style, subtle_button_style, with_tooltip};

/// 返回中号字形按钮的交互样式。
pub(super) fn medium_glyph_button_style(status: button::Status) -> button::Style {
    subtle_button_style(status, 8.0)
}

/// 返回小号字形按钮的交互样式。
pub(super) fn small_glyph_button_style(status: button::Status) -> button::Style {
    subtle_button_style(status, 6.0)
}

/// 返回中号无提示字形按钮的主题样式。
pub(super) fn medium_plain_glyph_button_style(theme: &Theme) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { radius: 8.0.into(), ..Default::default() },
        text_color: theme.extended_palette().background.strong.text,
        ..Default::default()
    }
}

/// 返回小号无提示字形按钮的主题样式。
pub(super) fn small_plain_glyph_button_style(theme: &Theme) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border { radius: 6.0.into(), ..Default::default() },
        text_color: theme.extended_palette().background.strong.text,
        ..Default::default()
    }
}

/// 构建 medium glyph button 控件，并绑定既有消息或样式。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub fn medium_glyph_button(
    glyph: &'static str,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    let button = button(text(glyph).size(14))
        .on_press(on)
        .padding([4, 8])
        .style(|_theme: &Theme, status| medium_glyph_button_style(status));
    with_tooltip(button, tip, true, 6.0)
}

/// 构建 small glyph button 控件，并绑定既有消息或样式。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub fn small_glyph_button(
    glyph: &'static str,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    let button = button(text(glyph).size(12))
        .on_press(on)
        .padding([2, 6])
        .style(|_theme: &Theme, status| small_glyph_button_style(status));
    with_tooltip(button, tip, true, 4.0)
}

/// 构建 medium plain glyph button 控件，并绑定既有消息或样式。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub fn medium_plain_glyph_button(
    glyph: &'static str,
    // _tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    button(text(glyph).size(14))
        .on_press(on)
        .padding([4, 8])
        .style(|theme: &Theme, _status| medium_plain_glyph_button_style(theme))
        .into()
}

/// 构建 small plain glyph button 控件，并绑定既有消息或样式。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[allow(dead_code)]
pub fn small_plain_glyph_button(
    glyph: &'static str,
    // _tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    button(text(glyph).size(12))
        .on_press(on)
        .padding([1, 4])
        .style(|theme: &Theme, _status| small_plain_glyph_button_style(theme))
        .into()
}

/// 构建 header plain glyph button 控件，并绑定既有消息或样式。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn header_plain_glyph_button(
    glyph: &'static str,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
    // on 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on: Message,
) -> Element<'static, Message> {
    let button = button(text(glyph).size(14))
        .on_press(on)
        .padding([4, 8])
        .style(header_plain_glyph_button_style);
    with_tooltip(button, tip, true, 6.0)
}
