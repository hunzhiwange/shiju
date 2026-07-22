//! 用量视图的会话菜单，负责会话选择、筛选和交互消息组装。

use iced::widget::svg;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Point, Theme};

use crate::app::assets::Icon;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::{App, Message};

use super::components::RightClickArea;
use super::utils::icon_svg;

fn session_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        iced::widget::button::Status::Hovered => p.background.weak.color,
        iced::widget::button::Status::Pressed => p.background.strong.color,
        _ => Color::TRANSPARENT,
    };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: theme.palette().text,
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        ..Default::default()
    }
}

fn session_menu_separator_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.strong.color)),
        ..Default::default()
    }
}

fn session_menu_container_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.base.color)),
        border: Border { width: 1.0, color: p.background.strong.color, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.15),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

fn session_menu_icon_style() -> svg::Style {
    svg::Style { color: Some(iced::Color::from_rgb8(0x80, 0x80, 0x80)) }
}

fn session_menu_icon_button_style(theme: &Theme) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border { radius: 4.0.into(), ..Border::default() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn session_label_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().background.weak.text.scale_alpha(0.9)),
    }
}

fn session_value_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text) }
}

fn session_right_click_message(id: &str, pos: Point) -> Message {
    Message::Project(crate::app::message::ProjectMessage::SessionRightClicked(
        id.to_string(),
        pos.x,
        pos.y,
    ))
}

fn session_menu_button<'a>(label: &str, msg: Message) -> Element<'a, Message> {
    let label = label.to_string();
    button(container(text(label).size(13)).width(Length::Fill).padding([6, 10]))
        .on_press(msg)
        .style(session_menu_button_style)
        .width(Length::Fill)
        .into()
}

fn build_session_menu<'a>(id: String) -> Element<'a, Message> {
    let separator = || -> Element<'a, Message> {
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(session_menu_separator_style)
            .into()
    };

    let content = column![
        session_menu_button(
            "重命名",
            Message::Project(crate::app::message::ProjectMessage::SessionRenamePressed(id.clone()))
        ),
        session_menu_button(
            "复制对话",
            Message::Project(crate::app::message::ProjectMessage::SessionCopyPressed(id.clone()))
        ),
        separator(),
        session_menu_button(
            "归档",
            Message::Project(crate::app::message::ProjectMessage::SessionArchivePressed(
                id.clone()
            ))
        ),
        session_menu_button(
            "删除",
            Message::Project(crate::app::message::ProjectMessage::SessionDeletePressed(id.clone()))
        ),
    ]
    .spacing(4);

    container(content)
        .padding(6)
        .width(Length::Fixed(140.0))
        .style(session_menu_container_style)
        .into()
}

/// 构建或更新 kv with menu 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn kv_with_menu<'a>(
    app: &App,
    label: &'a str,
    value: String,
    session_id: Option<String>,
) -> Element<'a, Message> {
    let menu_btn = if let Some(id) = session_id {
        let id_for_click = id.clone();
        let btn = button(
            container(
                icon_svg(Icon::ChevronDown)
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|_theme: &Theme, _| session_menu_icon_style()),
            )
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .padding(0)
        .style(|theme: &Theme, _status| session_menu_icon_button_style(theme));

        let right_click = Element::new(RightClickArea::new(
            btn.into(),
            Box::new(move |pos: iced::Point| session_right_click_message(&id_for_click, pos)),
        ));

        if app.session_menu_id.as_ref() == Some(&id) {
            Element::from(
                PointBelowOverlay::new(right_click, build_session_menu(id))
                    .show(true)
                    .anchor(app.session_menu_anchor.unwrap_or(Point::ORIGIN))
                    .on_close(Message::Project(
                        crate::app::message::ProjectMessage::SessionMenuClose,
                    )),
            )
        } else {
            right_click
        }
    } else {
        Space::new().width(Length::Fixed(24.0)).into()
    };

    row![
        text(label).size(12).style(session_label_style),
        Space::new().width(Length::Fill),
        text(value).size(12).style(session_value_style),
        menu_btn
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

#[cfg(test)]
#[path = "session_menu_tests.rs"]
mod session_menu_tests;
