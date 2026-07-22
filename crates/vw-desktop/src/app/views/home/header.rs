//! 首页视图模块，负责项目入口、最近项目和常用工具入口的界面组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

use crate::app::assets::Icon;
use crate::app::components::system_settings_common::settings_panel_style;
use crate::app::{App, Message, message};

use super::common::{icon_svg, is_dark_theme, primary_button, secondary_button};

/// 渲染对应界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn render(app: &App) -> Element<'_, Message> {
    let choose =
        primary_button("选择文件夹", Message::Project(message::ProjectMessage::OpenFolderPressed));
    let open = secondary_button(
        "打开最近的项目",
        Message::Project(message::ProjectMessage::OpenProjectPressed),
    );

    let switches_row = row![app_chip(
        Icon::Grid1x2,
        "应用中心",
        Color::from_rgb8(0x35, 0x85, 0xD6),
        Message::View(message::ViewMessage::OpenApps),
    ),]
    .spacing(6)
    .align_y(Alignment::Center);

    let switches = scrollable(switches_row)
        .id(app.home_apps_bar_scroll_id.clone())
        .on_scroll(|viewport| home_apps_bar_scroll_changed(viewport.relative_offset().x))
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Shrink)
        .width(Length::Fill);

    container(
        row![
            row![text("应用").size(18)].spacing(4).align_y(Alignment::Center),
            container(switches).width(Length::Fill).center_x(Length::Fill),
            row![open, choose].spacing(10).align_y(Alignment::Center),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .padding([14, 18])
    .width(Length::Fill)
    .style(header_container_style)
    .into()
}

fn home_apps_bar_scroll_changed(relative_x: f32) -> Message {
    Message::View(message::ViewMessage::HomeAppsBarScrollChanged(relative_x))
}

fn header_container_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.width = 0.0;
    style.border.color = Color::TRANSPARENT;
    style.border.radius = 20.0.into();
    style
}

fn app_chip_icon_style(accent: Color) -> svg::Style {
    svg::Style { color: Some(accent) }
}

fn app_chip_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    accent: Color,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let pressed = matches!(status, iced::widget::button::Status::Pressed);
    let background = if pressed {
        if is_dark_theme(theme) {
            palette.background.strong.color.scale_alpha(0.92)
        } else {
            palette.background.weak.color.scale_alpha(0.92)
        }
    } else if hovered {
        if is_dark_theme(theme) {
            palette.background.weak.color.scale_alpha(0.86)
        } else {
            Color::WHITE.scale_alpha(0.94)
        }
    } else if is_dark_theme(theme) {
        palette.background.base.color.scale_alpha(0.60)
    } else {
        Color::WHITE.scale_alpha(0.82)
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        border: Border {
            width: 1.0,
            color: if hovered {
                accent.scale_alpha(0.34)
            } else {
                palette.background.strong.color.scale_alpha(0.72)
            },
            radius: 999.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.14 } else { 0.05 }),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 16.0,
        },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn app_chip_tooltip_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Color::from_rgba8(24, 24, 24, 0.96).into()),
        text_color: Some(Color::WHITE),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.40),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        snap: false,
    }
}

fn app_chip<'a>(icon: Icon, label: &'a str, accent: Color, on: Message) -> Element<'a, Message> {
    let icon =
        icon_svg(icon, 14.0).style(move |_theme: &Theme, _status| app_chip_icon_style(accent));

    let button = button(
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(on)
    .width(Length::Fixed(34.0))
    .height(Length::Fixed(34.0))
    .padding(0)
    .style(move |theme: &Theme, status| app_chip_button_style(theme, status, accent));

    let tip_content = container(text(label.to_string()).size(12))
        .padding([6, 8])
        .style(|_theme: &Theme| app_chip_tooltip_style());

    Tooltip::new(button, tip_content, TooltipPosition::Bottom).gap(8.0).into()
}
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
