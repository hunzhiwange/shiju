//! Git 面板通用按钮组件。
//!
//! 本模块封装 Git 面板中复用的图标、字形按钮、禁用态和提示样式。

use iced::Element;
/// 重新导出 use iced::widget::{Space, button}，让上层模块通过稳定路径访问。
use iced::widget::button;
/// 重新导出 use iced::{Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Length, Theme};

/// 重新导出 use desktop_assets::assets::Icon，让上层模块通过稳定路径访问。
use desktop_assets::assets::Icon;

/// 重新导出 use super::shared::{，让上层模块通过稳定路径访问。
use super::shared::{
    compact_outlined_button_style, icon_svg, icon_svg_sized, outlined_button_style,
    subtle_button_style, themed_icon_svg, with_tooltip,
};

/// 构建 icon button 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn icon_button<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(icon_svg(icon))
        .on_press(on)
        .style(|theme: &Theme, status| outlined_button_style(theme, status, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 square icon button 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn square_icon_button<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(icon_svg_sized(icon, 18.0))
        .on_press(on)
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(|theme: &Theme, status| outlined_button_style(theme, status, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 square icon button small 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn square_icon_button_small<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(icon_svg_sized(icon, 16.0))
        .on_press(on)
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(|theme: &Theme, status| outlined_button_style(theme, status, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 square icon button tiny 控件，并绑定既有消息或样式。
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
pub fn square_icon_button_tiny<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(themed_icon_svg(icon, 12.0))
        .on_press(on)
        .padding(6)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(|theme: &Theme, status| compact_outlined_button_style(theme, status, 7.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 square icon button micro 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn square_icon_button_micro<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(themed_icon_svg(icon, 10.0))
        .on_press(on)
        .padding(4)
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .style(|theme: &Theme, status| compact_outlined_button_style(theme, status, 6.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 medium icon button 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn medium_icon_button<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(icon_svg_sized(icon, 14.0))
        .on_press(on)
        .padding(4)
        .style(|_theme: &Theme, status| subtle_button_style(status, 0.0));
    with_tooltip(button, tip, true, 6.0)
}

/// 构建 small icon button 控件，并绑定既有消息或样式。
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
#[allow(dead_code)]
pub fn small_icon_button<Message: Clone + 'static>(icon: Icon, tip: String, on: Message) -> Element<'static, Message> {
    let button = button(icon_svg_sized(icon, 12.0))
        .on_press(on)
        .padding(2)
        .style(|_theme: &Theme, status| subtle_button_style(status, 0.0));
    with_tooltip(button, tip, true, 4.0)
}

/// 构建 small plain icon button 控件，并绑定既有消息或样式。
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
pub use super::small_plain_icon_button;
