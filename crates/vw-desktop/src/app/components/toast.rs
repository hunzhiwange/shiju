//! 应用内短消息提示的状态模型与渲染控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    danger_action_btn_style, rounded_action_btn_style, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_panel_style,
};
use crate::app::message::SkillHubMessage;
use crate::app::state::ToastKind;
use crate::app::{App, Message};
use iced::widget::svg::{self, Svg};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

pub(super) fn icon_svg(icon: Icon, size: f32, color: Color) -> Svg<'static> {
    Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme: &Theme, _status| svg::Style { color: Some(color) })
}

pub(super) struct ToastPalette {
    pub(super) icon: Icon,
    pub(super) accent: Color,
    pub(super) background: Color,
    pub(super) title: &'static str,
}

pub(super) fn toast_palette(kind: ToastKind) -> ToastPalette {
    match kind {
        ToastKind::Success => ToastPalette {
            icon: Icon::Check,
            accent: Color::from_rgb8(0x28, 0x8F, 0x61),
            background: Color::from_rgb8(0xE8, 0xF7, 0xEF),
            title: "操作已完成",
        },
        ToastKind::Info => ToastPalette {
            icon: Icon::QuestionCircle,
            accent: Color::from_rgb8(0x2A, 0x6F, 0xC2),
            background: Color::from_rgb8(0xE8, 0xF1, 0xFE),
            title: "提示",
        },
        ToastKind::Warning => ToastPalette {
            icon: Icon::QuestionCircle,
            accent: Color::from_rgb8(0xB5, 0x7A, 0x00),
            background: Color::from_rgb8(0xFF, 0xF4, 0xD6),
            title: "请注意",
        },
        ToastKind::Error => ToastPalette {
            icon: Icon::X,
            accent: Color::from_rgb8(0xC5, 0x3E, 0x3E),
            background: Color::from_rgb8(0xFE, 0xEA, 0xEA),
            title: "操作失败",
        },
    }
}

pub(super) fn toast_icon_badge_style(
    theme: &Theme,
    accent: Color,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            accent.scale_alpha(0.18)
        } else {
            accent.scale_alpha(0.12)
        })),
        border: Border {
            width: 1.0,
            color: accent.scale_alpha(if is_dark_theme(theme) { 0.36 } else { 0.18 }),
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn toast_message_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.extended_palette().background.base.text.scale_alpha(0.94)
    } else {
        theme.palette().text.scale_alpha(0.88)
    }
}

pub(super) fn toast_card_style(
    theme: &Theme,
    accent: Color,
    background: Color,
) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    let is_dark = is_dark_theme(theme);
    style.background =
        Some(Background::Color(if is_dark { background.scale_alpha(0.22) } else { background }));
    style.border = Border {
        width: 1.0,
        color: accent.scale_alpha(if is_dark { 0.54 } else { 0.22 }),
        radius: 12.0.into(),
    };
    style.shadow = iced::Shadow {
        color: Color::BLACK.scale_alpha(if is_dark { 0.18 } else { 0.08 }),
        offset: Vector::new(0.0, 14.0),
        blur_radius: 26.0,
    };
    style
}

pub(super) fn cache_sync_retry_message(app: &App) -> Option<Message> {
    app.expert_hub
        .cache_sync
        .pending_retry_skill_id()
        .map(|_| Message::SkillHub(SkillHubMessage::RetrySkillCacheSync))
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
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
pub fn view(app: &App) -> Element<'_, Message> {
    let Some(toast) = &app.active_toast else {
        return container(text("")).width(Length::Fixed(0.0)).height(Length::Fixed(0.0)).into();
    };

    let palette = toast_palette(toast.kind);
    let accent = palette.accent;
    let background = palette.background;

    let icon_badge = container(icon_svg(palette.icon, 14.0, accent))
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(34.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |theme: &Theme| toast_icon_badge_style(theme, accent));

    let mut content = row![
        icon_badge,
        column![
            text(palette.title)
                .size(11)
                .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(accent) }),
            text(&toast.message).size(13).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(toast_message_text_color(theme)),
            })
        ]
        .spacing(3)
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if let Some(message) = cache_sync_retry_message(app) {
        content = content.push(
            button(text("重试").size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(message),
        );
    }

    container(content)
        .padding([12, 14])
        .style(move |theme: &Theme| toast_card_style(theme, accent, background))
        .width(Length::Shrink)
        .into()
}

/// 构建通用确认弹层。
///
/// # 参数
///
/// 调用方提供标题、正文、按钮文案和确认/取消消息；本组件只负责渲染通用确认 UI。
///
/// # 返回值
///
/// 返回一个覆盖全屏的模态确认弹层，可作为任意页面的顶层 layer 复用。
///
/// # 错误处理
///
/// 本函数不执行业务操作；确认或取消后的错误由调用方消息处理。
pub fn confirm_dialog<'a>(
    title: impl Into<String>,
    body: impl Into<String>,
    confirm_label: &'a str,
    cancel_label: &'a str,
    confirm_message: Message,
    cancel_message: Message,
) -> Element<'a, Message> {
    let card = settings_modal_card(
        column![
            text(title.into()).size(18),
            text(body.into()).size(13).style(settings_muted_text_style),
            row![
                Space::new().width(Length::Fill),
                button(text(cancel_label).size(13))
                    .style(rounded_action_btn_style)
                    .padding([9, 14])
                    .on_press(cancel_message.clone()),
                button(text(confirm_label).size(13))
                    .style(danger_action_btn_style)
                    .padding([9, 14])
                    .on_press(confirm_message),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16),
    )
    .width(Length::Fixed(420.0));

    settings_modal_overlay(None, cancel_message, card)
}
