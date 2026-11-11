//! 应用内短消息提示的状态模型与渲染控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::{
    danger_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_header, settings_modal_overlay, settings_muted_text_style, settings_panel_style,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::message::SkillHubMessage;
use crate::app::state::ToastKind;
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg::{self, Svg};
use iced::widget::text::Wrapping;
use iced::widget::{Container, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

const TOAST_MAX_WIDTH: f32 = 360.0;

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

pub(super) fn icon_svg(icon: Icon, size: f32, color: Color) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size)).style(
        move |theme: &Theme, _status| {
            let color = if is_dark_theme(theme) {
                iced::theme::palette::mix(color, theme.palette().text, 0.30)
            } else {
                color
            };
            svg::Style { color: Some(color) }
        },
    )
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
    let background = if is_dark {
        iced::theme::palette::mix(theme.extended_palette().background.base.color, background, 0.08)
    } else {
        background
    };
    style.background = Some(Background::Color(background));
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

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cache_sync_retry_message(app: &App) -> Option<Message> {
    app.expert_hub
        .cache_sync
        .pending_retry_skill_id()
        .map(|_| Message::SkillHub(SkillHubMessage::RetrySkillCacheSync))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cache_sync_retry_message(_app: &App) -> Option<Message> {
    None
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
pub fn view(app: &App) -> Element<'static, Message> {
    let Some(toast) = &app.active_toast else {
        return container(text("")).width(Length::Fixed(0.0)).height(Length::Fixed(0.0)).into();
    };

    let palette = toast_palette(toast.kind);
    let accent = palette.accent;
    let background = palette.background;
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let title = i18n.__(palette.title);
    let message = toast
        .localized_message
        .as_ref()
        .map(|message| message.render(i18n).expect("valid toast template"))
        .unwrap_or_else(|| toast.message.clone());
    let label = if message.is_empty() { title.to_owned() } else { format!("{title} · {message}") };

    let icon_badge = container(icon_svg(palette.icon, 14.0, accent))
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(34.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |theme: &Theme| toast_icon_badge_style(theme, accent));

    let mut content = row![
        icon_badge,
        text(label).size(13).width(Length::Fill).wrapping(Wrapping::WordOrGlyph).style(
            |theme: &Theme| iced::widget::text::Style {
                color: Some(toast_message_text_color(theme)),
            }
        )
    ]
    .width(Length::Fill)
    .spacing(10)
    .align_y(Alignment::Center);
    let width = if let Some(message) = cache_sync_retry_message(app) {
        content = content.push(
            button(text(i18n.__("重试")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(message),
        );
        // Fix the card width so the row reserves space for the button before laying out text.
        Length::Fixed(TOAST_MAX_WIDTH)
    } else {
        Length::Shrink
    };

    container(content)
        .padding([8, 14])
        .style(move |theme: &Theme| toast_card_style(theme, accent, background))
        .width(width)
        .max_width(TOAST_MAX_WIDTH)
        .into()
}

/// 将询问弹窗和提示上下排布，使长提示不会盖住弹窗按钮。
pub(crate) fn modal_with_feedback<'a>(
    app: &App,
    card: Container<'a, Message>,
) -> Container<'a, Message> {
    let mut content = column![card].spacing(8).align_x(Alignment::Center);
    if app.active_toast.is_some() {
        let feedback = scrollable(container(view(app)).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)));
        content = content.push(container(feedback).max_height(116));
    }
    container(content).padding(12)
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
    let header = settings_modal_header(
        text(title.into()).size(18),
        settings_close_button(cancel_message.clone()),
    );
    let body = scrollable(
        container(text(body.into()).size(13).style(settings_muted_text_style))
            .padding([0, 7])
            .width(Length::Fill),
    )
    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
    .width(Length::Fill);
    let actions = container(
        row![
            button(text(cancel_label).size(13))
                .style(rounded_action_btn_style)
                .padding([8, 14])
                .on_press(cancel_message.clone()),
            button(text(confirm_label).size(13))
                .style(danger_action_btn_style)
                .padding([8, 14])
                .on_press(confirm_message),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    )
    .align_right(Length::Fill);
    let card =
        settings_modal_card(column![header, container(body).max_height(150), actions].spacing(16))
            .width(420);

    settings_modal_overlay(None, cancel_message, container(card).padding(12))
}

#[cfg(test)]
#[path = "toast_ui_refresh_33_tests.rs"]
mod toast_ui_refresh_33_tests;
