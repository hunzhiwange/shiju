//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::Message;
use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::theme::is_dark_theme;
use iced::widget::{
    button, container,
    svg::{self, Svg},
    text,
    tooltip::{Position as TooltipPosition, Tooltip},
};
use iced::{Background, Border, Color, Element, Length, Shadow};

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
pub fn icon_svg(icon: Icon, size: f32) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size))
}

/// 构建或处理 `provider_logo_svg` 对应的界面片段与交互数据。
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
pub fn provider_logo_svg(provider_id: &str, size: f32) -> Svg<'static> {
    Svg::new(assets::get_provider_icon(provider_id))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(provider_logo_svg_style)
}

pub(super) fn provider_logo_svg_style(theme: &iced::Theme, _status: svg::Status) -> svg::Style {
    let color = if is_dark_theme(theme) { Some(theme.palette().text) } else { None };
    svg::Style { color }
}

pub(super) fn icon_btn_svg_style(theme: &iced::Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text) }
}

pub(super) fn icon_btn_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(palette.background.weak.color.into()),
        iced::widget::button::Status::Pressed => Some(palette.background.strong.color.into()),
        _ => None,
    };

    iced::widget::button::Style {
        background: bg,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub(super) fn icon_btn_tooltip_style(theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(theme.palette().background)),
        border: Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// 构建或处理 `icon_btn` 对应的界面片段与交互数据。
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
pub fn icon_btn(icon: Icon, tip: &'static str, on: Option<Message>) -> Element<'static, Message> {
    let btn = {
        let base = button(icon_svg(icon, 14.0).style(icon_btn_svg_style))
            .padding([4, 6])
            .style(icon_btn_style);

        if let Some(msg) = on { base.on_press(msg) } else { base }
    };

    let tip_content = container(text(tip).size(12)).padding([6, 10]).style(icon_btn_tooltip_style);

    Tooltip::new(btn, tip_content, TooltipPosition::FollowCursor).gap(6).into()
}
