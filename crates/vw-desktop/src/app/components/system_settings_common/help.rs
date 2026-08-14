//! 系统设置页面复用的通用控件、样式与辅助能力。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::icons::{icon_btn, icon_svg};
use crate::app::components::system_settings_common::panels::{
    settings_modal_card, settings_modal_overlay,
};
use crate::app::components::system_settings_common::styles::settings_muted_text_style;
use crate::app::components::system_settings_common::theme::is_dark_theme;
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Border, Color, Element, Length};
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
    icon_btn(Icon::QuestionCircle, "帮助", Some(on_press))
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
    let close_btn = settings_close_button(close_message.clone());
    let (copy_icon, copy_tip) = settings_help_copy_state(app.last_copied_code_hash, &help_text);
    let copy_btn = icon_btn(copy_icon, copy_tip, Some(Message::CopyCode(help_text.clone())));
    let header = row![text(title).size(16), Space::new().width(Length::Fill), copy_btn, close_btn,]
        .spacing(10)
        .align_y(Alignment::Center);

    let body = scrollable(
        container(text(help_text).size(13).style(settings_muted_text_style))
            .width(Length::Fill)
            .padding([4, 0]),
    )
    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
    .width(Length::Fill)
    .height(Length::Fill);

    let card = settings_modal_card(
        column![header, body].spacing(12).width(Length::Fill).height(Length::Fill),
    )
    .width(Length::Fixed(720.0))
    .height(Length::Fixed(560.0));

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
