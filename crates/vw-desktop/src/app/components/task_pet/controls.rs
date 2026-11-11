//! 独立胶囊的点击区域不包含头像拖动处理器。
use super::*;

pub(super) fn toolbar(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let write = button(text(i18n.__("写消息")).size(13))
        .width(86)
        .height(34)
        .padding([6, 10])
        .style(action_style)
        .on_press(Message::View(message::ViewMessage::TaskPetWriteMessage));
    let expand = button(text(if app.task_pet_collapsed { "⌄" } else { "⌃" }).size(16))
        .width(34)
        .height(34)
        .padding(6)
        .style(action_style)
        .on_press(Message::View(message::ViewMessage::TaskPetToggleCollapsed));
    container(
        row![write, container(iced::widget::rule::vertical(1)).height(18), expand]
            .align_y(Alignment::Center),
    )
    .padding(2)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(palette.background.strong.color.into()),
            text_color: Some(palette.background.strong.text),
            border: Border {
                color: palette.background.weak.text.scale_alpha(0.25),
                width: 1.0,
                radius: 22.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.2),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        }
    })
    .into()
}

fn action_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(palette.background.weak.color.into()),
        text_color: palette.background.strong.text,
        border: Border { radius: 18.0.into(), ..Default::default() },
        ..Default::default()
    }
}
