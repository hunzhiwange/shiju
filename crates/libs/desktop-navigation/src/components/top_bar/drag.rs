//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::Message;
use iced::widget::{Space, container, mouse_area};
use iced::{Element, Length};

#[cfg(target_os = "macos")]
/// 构建或处理 `traffic_light_spacer` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn traffic_light_spacer() -> Element<'static, Message> {
    let content: Element<'static, Message> =
        container(Space::new()).width(Length::Fixed(75.0)).height(Length::Fill).into();
    mouse_area(content).on_press(Message::WindowDragPressed).into()
}

#[cfg(not(target_os = "macos"))]
/// 构建或处理 `traffic_light_spacer` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn traffic_light_spacer() -> Element<'static, Message> {
    Space::new().width(Length::Fixed(0.0)).into()
}

/// 构建或处理 `drag_spacer` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn drag_spacer() -> Element<'static, Message> {
    let content: Element<'static, Message> =
        container(Space::new()).width(Length::Fill).height(Length::Fill).into();
    mouse_area(content).on_press(Message::WindowDragPressed).into()
}
