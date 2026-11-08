//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::chat_panel::message_view::selectable_titled_text;
use crate::app::components::overlays::side::SideOverlay;
use crate::app::components::system_settings_common::icons::{
    icon_btn, icon_btn_tooltip_style, icon_svg,
};
use crate::app::components::system_settings_common::panels::{
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
};
use crate::app::components::system_settings_common::styles::settings_muted_text_style;
use crate::app::components::system_settings_common::theme::is_dark_theme;
use crate::app::i18n::I18n;
use crate::app::{App, Message};
use iced::widget::{Row, Text, button, container, mouse_area, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector, mouse};
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

/// 构建或处理 `settings_close_button` 对应的界面片段与交互数据。
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
pub fn settings_close_button(on_press: Message) -> Element<'static, Message> {
    button(
        container(icon_svg(Icon::X, 12.0).style(settings_close_icon_style))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .padding(0)
    .style(settings_close_button_style)
    .on_press(on_press)
    .into()
}

pub(super) fn settings_close_icon_style(theme: &iced::Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.72)
        } else {
            theme.palette().text.scale_alpha(0.82)
        }),
    }
}

pub(super) fn settings_close_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => {
            Some(palette.background.weak.color.scale_alpha(0.72).into())
        }
        iced::widget::button::Status::Pressed => {
            Some(palette.background.strong.color.scale_alpha(0.82).into())
        }
        _ => None,
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        ..Default::default()
    }
}

/// 构建或处理 `settings_help_button` 对应的界面片段与交互数据。
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
pub fn settings_help_button(on_press: Message) -> Element<'static, Message> {
    settings_help_button_with_locale(
        vw_config_types::ui::DesktopLocale::SimplifiedChinese,
        on_press,
    )
}

pub fn settings_help_button_with_locale(
    locale: vw_config_types::ui::DesktopLocale,
    on_press: Message,
) -> Element<'static, Message> {
    icon_btn(Icon::QuestionCircle, crate::app::i18n::I18n::new(locale).__("帮助"), Some(on_press))
}

/// 将已本地化的静态说明放进可悬停、可固定的问号卡片。
pub(crate) fn settings_hover_help_label<'a>(
    i18n: I18n,
    theme: &iced::Theme,
    title: &'a str,
    label: Text<'a>,
    description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    let icon = mouse_area(
        container(icon_svg(Icon::QuestionCircle, 14.0).style(settings_close_icon_style))
            .center_x(22)
            .center_y(22),
    )
    .interaction(mouse::Interaction::Pointer);
    let card = container(selectable_titled_text(title, description, theme.palette().text, i18n))
        .width(300)
        .padding([10, 12])
        .style(|theme| container::Style {
            shadow: Shadow {
                color: Color::BLACK.scale_alpha(0.22),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..icon_btn_tooltip_style(theme)
        });
    row![label, SideOverlay::new(icon, card).hover_and_click().gap(4.0).min_y(8.0)]
        .spacing(4)
        .align_y(Alignment::Center)
}

/// 构建或处理 `with_settings_help_modal` 对应的界面片段与交互数据。
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
pub fn with_settings_help_modal<'a>(
    app: &App,
    base: Element<'a, Message>,
    title: &'a str,
    help_text: impl Into<String>,
    close_message: Message,
) -> Element<'a, Message> {
    let help_text = help_text.into();
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let help_text = i18n.__(&help_text).to_string();
    let close_btn = settings_close_button(close_message.clone());
    let (copy_icon, copy_tip) = settings_help_copy_state(app.last_copied_code_hash, &help_text);
    let copy_tip = crate::app::i18n::settings_runtime::text(
        app.desktop_locale,
        if copy_tip == "已复制" { "已复制" } else { "复制帮助" },
    );
    let copy_btn = icon_btn(copy_icon, copy_tip, Some(Message::CopyCode(help_text.clone())));
    let header = settings_modal_header(
        text(i18n.__(title).to_string()).size(16),
        row![copy_btn, close_btn].spacing(10).align_y(Alignment::Center),
    );
    let body = text(help_text).size(13).style(settings_muted_text_style);
    let card = settings_modal_layout(header, body, None).width(720).height(560);

    settings_modal_overlay(Some(base), close_message, card)
}

pub fn settings_help_copy_state(
    last_copied_code_hash: Option<u64>,
    help_text: &str,
) -> (Icon, &'static str) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    help_text.hash(&mut hasher);
    let help_text_hash = hasher.finish();

    if last_copied_code_hash == Some(help_text_hash) {
        (Icon::Check, "已复制")
    } else {
        (Icon::Copy, "复制帮助")
    }
}

#[cfg(test)]
#[path = "help_ui_refresh_03_tests.rs"]
mod help_ui_refresh_03_tests;
