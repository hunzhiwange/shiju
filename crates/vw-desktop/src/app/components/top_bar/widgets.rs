//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::{self, Icon};
use crate::app::message::view::MenuType;
use crate::app::{Message, message};
use iced::widget::svg::{self, Svg};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Color, Element, Length, Theme};

/// 构建或处理 `color_with_alpha` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn color_with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

/// 构建或处理 `icon_svg` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0))
}

/// 根据按钮可用状态生成工具栏图标样式。
pub(super) fn icon_svg_style(theme: &Theme, enabled: bool) -> svg::Style {
    let alpha = if enabled { 1.0 } else { 0.35 };

    svg::Style { color: Some(theme.palette().text.scale_alpha(alpha)) }
}

/// 根据交互状态生成工具栏图标按钮样式。
pub(super) fn icon_toggle_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
    active: bool,
) -> iced::widget::button::Style {
    let icon_alpha = if enabled { 1.0 } else { 0.35 };
    let background = if !enabled {
        None
    } else {
        match status {
            iced::widget::button::Status::Hovered => {
                Some(color_with_alpha(theme.palette().text, 0.14).into())
            }
            iced::widget::button::Status::Pressed => {
                Some(color_with_alpha(theme.palette().text, 0.22).into())
            }
            _ if active => Some(color_with_alpha(theme.palette().text, 0.10).into()),
            _ => None,
        }
    };

    iced::widget::button::Style {
        background,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
        text_color: theme.palette().text.scale_alpha(icon_alpha),
        ..Default::default()
    }
}

/// 生成工具栏图标提示框样式。
pub(super) fn icon_tooltip_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(iced::Background::Color(theme.palette().background)),
        border: iced::Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 8.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

/// 构建或处理 `icon_button` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn icon_button<'a>(
    icon: Icon,
    tip: &'a str,
    position: TooltipPosition,
    on: Message,
) -> Element<'a, Message> {
    icon_toggle_button(icon, tip, position, false, on)
}

/// 构建或处理 `icon_toggle_button` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn icon_toggle_button<'a>(
    icon: Icon,
    tip: &'a str,
    position: TooltipPosition,
    active: bool,
    on: Message,
) -> Element<'a, Message> {
    icon_toggle_button_opt(icon, tip, position, active, Some(on))
}

/// 构建或处理 `icon_toggle_button_opt` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn icon_toggle_button_opt<'a>(
    icon: Icon,
    tip: &'a str,
    position: TooltipPosition,
    active: bool,
    on: Option<Message>,
) -> Element<'a, Message> {
    let enabled = on.is_some();

    let btn =
        button(icon_svg(icon).style(move |theme: &Theme, _status| icon_svg_style(theme, enabled)))
            .height(Length::Fixed(24.0))
            .padding([4, 6])
            .style(move |theme: &Theme, status| {
                icon_toggle_button_style(theme, status, enabled, active)
            });

    let btn = if let Some(on) = on { btn.on_press(on) } else { btn };
    let tip_content = container(text(tip.to_string())).padding([6, 10]).style(icon_tooltip_style);

    Tooltip::new(btn, tip_content, position).gap(10).into()
}

/// 根据菜单选中态和交互状态生成顶部菜单按钮样式。
pub(super) fn menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_active: bool,
) -> iced::widget::button::Style {
    let background = if is_active || status == iced::widget::button::Status::Hovered {
        color_with_alpha(theme.palette().text, 0.10)
    } else {
        Color::TRANSPARENT
    };

    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: theme.palette().text,
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 根据弹层状态生成代码/工作模式按钮样式。
pub(super) fn workspace_mode_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    open: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => {
            palette.background.strong.color.scale_alpha(0.72)
        }
        iced::widget::button::Status::Pressed => {
            palette.background.strong.color.scale_alpha(0.88)
        }
        _ if open => palette.background.strong.color.scale_alpha(0.68),
        _ => palette.background.weak.color.scale_alpha(0.72),
    };

    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.72),
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}

/// 构建或处理 `menu_btn` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn menu_btn<'a>(
    label: &'a str,
    menu_type: MenuType,
    active_menu: Option<MenuType>,
) -> Element<'a, Message> {
    let is_active = active_menu == Some(menu_type);

    let btn = button(
        text(label).size(13).height(Length::Fill).align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::View(message::ViewMessage::ToggleMenu(Some(menu_type))))
    .height(Length::Fill)
    .style(move |theme: &Theme, status| menu_button_style(theme, status, is_active))
    .padding([0, 10]);

    mouse_area(btn).on_enter(Message::View(message::ViewMessage::MenuHovered(menu_type))).into()
}

/// 生成顶部菜单浮层样式。
pub(super) fn menu_container_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(iced::Background::Color(color_with_alpha(
            theme.palette().background,
            0.96,
        ))),
        border: iced::Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color.scale_alpha(0.70),
            radius: 8.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.18),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 26.0,
        },
        snap: false,
    }
}

/// 构建或处理 `menu_container` 对应的界面片段与交互数据。
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
pub(super) fn menu_container<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).padding(4).width(Length::Fixed(220.0)).style(menu_container_style).into()
}

/// 根据可用状态和交互状态生成菜单项按钮样式。
pub(super) fn menu_item_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = if !enabled {
        None
    } else if status == iced::widget::button::Status::Hovered {
        Some(palette.primary.base.color)
    } else {
        None
    };

    iced::widget::button::Style {
        background: background.map(iced::Background::Color),
        text_color: if enabled && status == iced::widget::button::Status::Hovered {
            Color::WHITE
        } else if enabled {
            theme.palette().text
        } else {
            theme.palette().text.scale_alpha(0.35)
        },
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
        ..Default::default()
    }
}

/// 构建或处理 `menu_item_btn` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn menu_item_btn<'a>(
    label: &'a str,
    shortcut: Option<&'a str>,
    msg: Option<Message>,
) -> Element<'a, Message> {
    let enabled = msg.is_some();
    let shortcut_el: Element<'a, Message> =
        if let Some(s) = shortcut { text(s).size(12).into() } else { Space::new().into() };
    let content = row![text(label).size(13), Space::new().width(Length::Fill), shortcut_el]
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Center);

    let base = button(content)
        .style(move |theme: &Theme, status| menu_item_button_style(theme, status, enabled))
        .width(Length::Fill)
        .padding([4, 12]);

    if let Some(msg) = msg {
        base.on_press(Message::View(message::ViewMessage::MenuAction(Box::new(msg)))).into()
    } else {
        base.into()
    }
}

/// 生成菜单分隔线样式。
pub(super) fn menu_separator_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(
            theme.extended_palette().background.strong.color.scale_alpha(0.70),
        )),
        ..Default::default()
    }
}

/// 构建或处理 `menu_separator` 对应的界面片段与交互数据。
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
pub(super) fn menu_separator<'a>() -> Element<'a, Message> {
    container(
        container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
            .style(menu_separator_style),
    )
    .padding([4, 12])
    .into()
}

/// 构建或处理 `menu_item_icon_btn` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn menu_item_icon_btn<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    shortcut: Option<&'a str>,
    msg: Option<Message>,
) -> Element<'a, Message> {
    let enabled = msg.is_some();
    let icon_el: Element<'a, Message> =
        container(icon).width(Length::Fixed(18.0)).height(Length::Fixed(18.0)).into();
    let shortcut_el: Element<'a, Message> =
        if let Some(s) = shortcut { text(s).size(12).into() } else { Space::new().into() };
    let content = row![
        icon_el,
        Space::new().width(Length::Fixed(6.0)),
        text(label).size(13),
        Space::new().width(Length::Fill),
        shortcut_el,
    ]
    .width(Length::Fill)
    .align_y(iced::alignment::Vertical::Center);

    let base = button(content)
        .style(move |theme: &Theme, status| menu_item_button_style(theme, status, enabled))
        .width(Length::Fill)
        .padding([6, 12]);

    if let Some(msg) = msg {
        base.on_press(Message::View(message::ViewMessage::MenuAction(Box::new(msg)))).into()
    } else {
        base.into()
    }
}
