//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::Message;
use crate::app::components::system_settings_common::styles::{
    settings_modal_backdrop_style, settings_modal_card_style, settings_muted_text_style,
    settings_panel_style,
};
use crate::app::components::system_settings_common::theme::is_dark_theme;
use iced::widget::{Space, column, container, mouse_area, opaque, stack, text};
use iced::{Background, Border, Color, Element, Length, Theme};

/// 构建或处理 `settings_panel` 对应的界面片段与交互数据。
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
pub fn settings_panel<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into()).padding([18, 20]).width(Length::Fill).style(settings_panel_style)
}

/// 构建或处理 `settings_modal_card` 对应的界面片段与交互数据。
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
pub fn settings_modal_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(content.into()).padding([22, 24]).style(settings_modal_card_style)
}

/// 构建或处理 `settings_modal_overlay` 对应的界面片段与交互数据。
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
pub fn settings_modal_overlay<'a>(
    base: Option<Element<'a, Message>>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let overlay = opaque(
        mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .style(settings_modal_backdrop_style),
        )
        .on_press(close_message.clone()),
    );

    let modal_layer: Element<'a, Message> = opaque(
        container(card.into())
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    );

    let modal_stack: Element<'a, Message> = stack![overlay, modal_layer].into();

    if let Some(base) = base { stack![base, modal_stack].into() } else { modal_stack }
}

/// 构建或处理 `settings_section_card` 对应的界面片段与交互数据。
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
pub fn settings_section_card<'a>(
    title: &'a str,
    description: &'a str,
) -> iced::widget::Container<'a, Message> {
    container(
        column![text(title).size(14), text(description).size(12).style(settings_muted_text_style),]
            .spacing(6),
    )
    .padding([14, 16])
    .width(Length::Fill)
    .style(settings_section_card_style)
}

/// 构建或处理 `settings_page_intro` 对应的界面片段与交互数据。
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
pub fn settings_page_intro<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text(title).size(18), text(description).size(12).style(settings_muted_text_style),]
        .spacing(4)
        .into()
}

/// 构建或处理 `settings_divider` 对应的界面片段与交互数据。
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
pub fn settings_divider() -> Element<'static, Message> {
    container(Space::new()).width(Length::Fill).height(1.0).style(settings_divider_style).into()
}

/// 构建或处理 `settings_value_badge` 对应的界面片段与交互数据。
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
pub fn settings_value_badge(value: impl ToString) -> Element<'static, Message> {
    container(text(value.to_string()).size(13))
        .padding([7, 12])
        .style(settings_value_badge_style)
        .into()
}

/// 将字段提示紧贴在对应控件下方，避免提示脱离字段落到面板底部。
pub fn settings_control_with_hint<'a>(
    control: impl Into<Element<'a, Message>>,
    hint: &'a str,
) -> Element<'a, Message> {
    column![control.into(), text(hint).size(12).style(settings_muted_text_style)]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

/// 构建或处理 `settings_error_banner` 对应的界面片段与交互数据。
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
pub fn settings_error_banner<'a>(message: &'a str) -> Element<'a, Message> {
    container(text(message).size(13))
        .padding([10, 12])
        .width(Length::Fill)
        .style(settings_error_banner_style)
        .into()
}

/// 构建或处理 `settings_success_banner` 对应的界面片段与交互数据。
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
pub fn settings_success_banner<'a>(message: &'a str) -> Element<'a, Message> {
    container(text(message).size(13))
        .padding([10, 12])
        .width(Length::Fill)
        .style(settings_success_banner_style)
        .into()
}

pub(super) fn settings_section_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.20)
        } else {
            Color::from_rgba8(246, 248, 252, 0.96)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.8)
            } else {
                Color::from_rgba8(15, 23, 42, 0.06)
            },
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_divider_style(theme: &Theme) -> iced::widget::container::Style {
    let color = if is_dark_theme(theme) {
        theme.extended_palette().background.strong.color.scale_alpha(0.72)
    } else {
        Color::from_rgba8(15, 23, 42, 0.06)
    };
    iced::widget::container::Style {
        background: Some(Background::Color(color)),
        ..Default::default()
    }
}

pub(super) fn settings_value_badge_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(if is_dark {
            palette.background.weak.color.scale_alpha(0.36)
        } else {
            Color::from_rgba8(246, 248, 252, 0.96)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                palette.background.strong.color.scale_alpha(0.84)
            } else {
                Color::from_rgba8(15, 23, 42, 0.08)
            },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_error_banner_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(palette.danger.base.color),
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(0.18))),
        border: Border {
            width: 1.0,
            color: palette.danger.base.color.scale_alpha(0.45),
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn settings_success_banner_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(palette.success.base.color),
        background: Some(Background::Color(palette.success.weak.color.scale_alpha(0.18))),
        border: Border {
            width: 1.0,
            color: palette.success.base.color.scale_alpha(0.45),
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}
