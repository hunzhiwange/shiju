//! Git 文件回滚确认弹窗。
use iced::{Element, Color, Background, Border, Length};
use iced::widget::{button, text, container, column, row, Space};
#[derive(Debug, Clone)]
pub enum PanelMessage { CancelDiscardFile, DiscardFile(String) }
pub fn discard_file_modal(locale: config_ui::DesktopLocale, file: Option<&str>) -> Option<Element<'static, PanelMessage>> {
    let file = file?.to_string();

    let cancel_button = button(text(desktop_i18n::I18n::new(locale).__("取消")))
        .on_press(PanelMessage::CancelDiscardFile)
        .padding([6, 12])
        .style(|theme: &iced::Theme, status| {
            let ext = theme.extended_palette();
            let bg = match status {
                iced::widget::button::Status::Hovered => {
                    ext.background.strong.color.scale_alpha(0.30)
                }
                iced::widget::button::Status::Pressed => {
                    ext.background.strong.color.scale_alpha(0.40)
                }
                _ => ext.background.strong.color.scale_alpha(0.22),
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    width: 1.0,
                    color: ext.background.strong.color.scale_alpha(0.55),
                    radius: 10.0.into(),
                },
                text_color: theme.palette().text,
                ..Default::default()
            }
        });

    let confirm_button =
        button(text(desktop_i18n::I18n::new(locale).__("确认回滚")))
            .on_press(PanelMessage::DiscardFile(file.clone()))
            .padding([6, 12])
            .style(|theme: &iced::Theme, status| {
                let ext = theme.extended_palette();
                let bg = match status {
                    iced::widget::button::Status::Hovered => {
                        ext.danger.base.color.scale_alpha(0.92)
                    }
                    iced::widget::button::Status::Pressed => {
                        ext.danger.base.color.scale_alpha(0.82)
                    }
                    _ => ext.danger.base.color,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(bg)),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
                    text_color: Color::WHITE,
                    ..Default::default()
                }
            });

    let modal_actions =
        row![cancel_button, confirm_button].spacing(10).align_y(iced::Alignment::Center);

    let modal = container(
        column![
            text(
                desktop_i18n::I18n::new(locale)
                    .format("确认回滚文件：%s？", &[file.into()])
                    .expect("valid revert confirmation")
            )
            .size(16),
            modal_actions
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fixed(420.0))
    .style(|theme: &iced::Theme| iced::widget::container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.25),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    });

    let overlay = container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
        iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
            ..Default::default()
        }
    });

    let modal_layer: Element<'_, PanelMessage> = iced::widget::opaque(
        container(modal)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    );

    Some(iced::widget::stack![overlay, modal_layer].into())
}
