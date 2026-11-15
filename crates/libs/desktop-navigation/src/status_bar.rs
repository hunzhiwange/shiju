//! Status bar composition and notification entry.
use iced::{Element, Theme, Background, Color};
use iced::widget::button;
pub const STATUS_BAR_HEIGHT: f32 = 30.0;
fn is_dark_theme(theme: &Theme) -> bool {
    let palette = theme.palette();
    palette.background.r + palette.background.g + palette.background.b < 1.5
}

pub fn notification_button<'a>(count: usize) -> Element<'a, super::components::notification::Message> {

    let icon: Element<'_, super::components::notification::Message> = if count > 0 {
        iced::widget::text("⚠️")
            .size(14)
            .style(move |theme: &iced::Theme| {
                let p = theme.extended_palette();
                iced::widget::text::Style { color: Some(p.warning.base.color) }
            })
            .into()
    } else {
        iced::widget::text("🔔")
            .size(14)
            .style(move |theme: &iced::Theme| {
                let p = theme.extended_palette();
                iced::widget::text::Style {
                    color: Some(if is_dark_theme(theme) {
                        p.background.base.text.scale_alpha(0.72)
                    } else {
                        theme.palette().text.scale_alpha(0.64)
                    }),
                }
            })
            .into()
    };

    let count_el: Element<'_, super::components::notification::Message> = if count > 0 {
        iced::widget::text(count.to_string())
            .size(12)
            .style(move |theme: &iced::Theme| {
                let p = theme.extended_palette();
                iced::widget::text::Style { color: Some(p.warning.base.color) }
            })
            .into()
    } else {
        iced::widget::Space::new()
            .width(iced::Length::Fixed(0.0))
            .height(iced::Length::Fixed(0.0))
            .into()
    };

    button(iced::widget::row![icon, count_el].spacing(4).align_y(iced::Alignment::Center))
        .on_press(super::components::notification::Message::Notification(super::components::notification::NotificationMessage::ToggleExpanded))
        .style(|theme: &iced::Theme, status| {
            let p = theme.extended_palette();
            iced::widget::button::Style {
                background: match status {
                    iced::widget::button::Status::Hovered => {
                        Some(Background::Color(if is_dark_theme(theme) {
                            p.background.weak.color.scale_alpha(0.42)
                        } else {
                            Color::WHITE.scale_alpha(0.52)
                        }))
                    }
                    iced::widget::button::Status::Pressed => {
                        Some(Background::Color(if is_dark_theme(theme) {
                            p.background.strong.color.scale_alpha(0.54)
                        } else {
                            p.background.weak.color.scale_alpha(0.76)
                        }))
                    }
                    _ => None,
                },
                text_color: if is_dark_theme(theme) {
                    p.background.base.text.scale_alpha(0.80)
                } else {
                    theme.palette().text.scale_alpha(0.72)
                },
                ..Default::default()
            }
        })
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn view<'a, M: 'a>(account: Element<'a, M>, quota: Option<Element<'a, M>>, lsp_content: Element<'a, M>, notification: Element<'a, M>) -> Element<'a, M> {
    let content = iced::widget::row![account]
        .extend(quota)
        .push(lsp_content)
        .push(iced::widget::Space::new().width(iced::Length::Fill))
        .push(notification)
        .align_y(iced::Alignment::Center)
        .spacing(10);

    iced::widget::container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(STATUS_BAR_HEIGHT))
        .padding([2, 8])
        .style(|theme: &iced::Theme| {
            let p = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(if is_dark_theme(theme) {
                    p.background.weak.color.scale_alpha(0.78)
                } else {
                    Color::from_rgba8(248, 250, 253, 0.96)
                })),
                text_color: Some(if is_dark_theme(theme) {
                    p.background.base.text.scale_alpha(0.82)
                } else {
                    theme.palette().text.scale_alpha(0.72)
                }),
                ..Default::default()
            }
        })
        .into()
}
#[cfg(target_arch = "wasm32")]
pub fn view<'a, M: 'a>(account: Element<'a, M>, quota: Option<Element<'a, M>>, lsp_content: Element<'a, M>, notification: Element<'a, M>) -> Element<'a, M> {
    let _ = lsp_content;
    let content = iced::widget::row![account]
        .extend(quota)
        .push(iced::widget::Space::new().width(iced::Length::Fill))
        .push(notification)
        .spacing(10)
        .align_y(iced::Alignment::Center);
    iced::widget::container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(STATUS_BAR_HEIGHT))
        .padding([0, 12])
        .align_y(iced::alignment::Vertical::Center)
        .style(|theme: &iced::Theme| iced::widget::container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            text_color: Some(theme.palette().text),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
#[path = "status_bar_tests.rs"]
mod tests;
