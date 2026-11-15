//! Git 面板通用按钮组件。
//!
//! 本模块封装 Git 面板中复用的图标、字形按钮、禁用态和提示样式。

use iced::Element;
/// 重新导出 use iced::widget::button，让上层模块通过稳定路径访问。
use iced::widget::button;
/// 重新导出 use iced::{Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Length, Theme};

/// 重新导出 use desktop_assets::assets::Icon，让上层模块通过稳定路径访问。
use desktop_assets::assets::Icon;

/// 重新导出 use super::shared::{disabled_button_style, disabled_icon_svg, with_tooltip}，让上层模块通过稳定路径访问。
use super::shared::{disabled_button_style, disabled_icon_svg, with_tooltip};

/// 构建 disabled icon button 控件，并绑定既有消息或样式。
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
pub fn disabled_icon_button<Message: Clone + 'static>(icon: Icon, tip: String) -> Element<'static, Message> {
    let button = button(disabled_icon_svg(icon, 18.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 disabled square icon button 控件，并绑定既有消息或样式。
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
pub fn disabled_square_icon_button<Message: Clone + 'static>(icon: Icon, tip: String) -> Element<'static, Message> {
    let button = button(disabled_icon_svg(icon, 18.0))
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 disabled square content button 控件，并绑定既有消息或样式。
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
pub fn disabled_square_content_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
) -> Element<'a, Message> {
    let button = button(content)
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 disabled square content button tiny 控件，并绑定既有消息或样式。
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
pub fn disabled_square_content_button_tiny<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    // tip 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tip: String,
) -> Element<'a, Message> {
    let button = button(content)
        .padding(6)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 7.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 disabled square icon button small 控件，并绑定既有消息或样式。
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
pub fn disabled_square_icon_button_small<Message: Clone + 'static>(icon: Icon, tip: String) -> Element<'static, Message> {
    let button = button(disabled_icon_svg(icon, 16.0))
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 8.0));
    with_tooltip(button, tip, false, 6.0)
}

/// 构建 disabled square icon button tiny 控件，并绑定既有消息或样式。
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
pub fn disabled_square_icon_button_tiny<Message: Clone + 'static>(icon: Icon, tip: String) -> Element<'static, Message> {
    let button = button(disabled_icon_svg(icon, 12.0))
        .padding(6)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(|theme: &Theme, _status| disabled_button_style(theme, 7.0));
    with_tooltip(button, tip, false, 6.0)
}
