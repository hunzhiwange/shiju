use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use iced::widget::{Space, button, column, container, row, svg, text};
use desktop_assets::assets::Icon;
use crate::card_style::is_dark_theme;
pub use desktop_ui_controls::tooltip::tooltip_dark_style;

pub fn agent_option<'a, Message: Clone + 'a>(
    label: String,
    subtitle: String,
    identity_icon: Element<'a, Message>,
    selected: bool,
    on_press: Option<Message>,
) -> button::Button<'a, Message> {
    let check: Element<'_, Message> = if selected {
        crate::card_style::icon_svg(Icon::Check)
            .width(14)
            .height(14)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.92)),
            })
            .into()
    } else {
        Space::new().width(14).into()
    };
    button(
        row![
            identity_icon,
            column![
                text(label).size(13),
                text(subtitle).size(11).style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().text.scale_alpha(0.62)),
                }),
            ]
            .spacing(3)
            .width(Length::Fill),
            check,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(move |theme, status| selectable_list_button_style(theme, status, selected))
    .on_press_maybe(on_press)
}

pub fn selectable_list_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let neutral_hover = if is_dark {
        theme.palette().text.scale_alpha(0.08)
    } else {
        theme.palette().text.scale_alpha(0.06)
    };
    let selected_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.15 } else { 0.10 },
    );
    let selected_hover_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.19 } else { 0.13 },
    );
    let background = if pressed || hovered {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if selected {
        Some(Background::Color(selected_bg))
    } else {
        None
    };
    button::Style {
        background,
        border: Border { radius: 9.0.into(), ..Border::default() },
        text_color: if selected {
            selector_text_color(theme, true)
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.90 } else { 0.88 })
        },
        ..Default::default()
    }
}
pub fn popover_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgb8(30, 33, 40)
    } else {
        Color::from_rgb8(252, 252, 253)
    };
    let border = if is_dark {
        Color::from_rgba8(44, 47, 53, 0.96)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 18.0.into(), width: 1.0, color: border },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}
pub fn session_lock_tooltip<'a>(content: impl Into<iced::Element<'a, crate::selector_input::Message>>, i18n: desktop_i18n::I18n, locked: bool) -> iced::Element<'a, crate::selector_input::Message> {
 if locked { iced::widget::tooltip::Tooltip::new(content,
 container(text(i18n.__("会话开始后无法更改。如需更换，请新建会话。")).size(12).style(|_: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) })).padding([6,8]).style(tooltip_dark_style),
 iced::widget::tooltip::Position::Top).gap(6).into() } else { content.into() }
}

fn selector_text_color(theme: &Theme, highlighted: bool) -> Color {
    theme.palette().text.scale_alpha(match (is_dark_theme(theme), highlighted) {
        (true, true) => 0.97, (false, true) => 0.95,
        (true, false) => 0.92, (false, false) => 0.88,
    })
}
