//! 常用任务卡片只发送选择消息，确认保存前不会创建任务。
use super::*;
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::icon_svg;
use crate::app::state::CRON_TEMPLATES;
use iced::widget::{column, responsive, text::Wrapping};

pub(super) fn view() -> Element<'static, Message> {
    responsive(|size| {
        let columns = if size.width >= 900.0 {
            3
        } else if size.width >= 560.0 {
            2
        } else {
            1
        };
        let mut cards = column![text("试试这些常用任务").size(18)].spacing(12);
        for group in CRON_TEMPLATES.chunks(columns) {
            let mut line = row![].spacing(12);
            for template in group {
                line = line.push(
                    button(
                        row![
                            icon_svg(Icon::Clock, 22.0),
                            column![
                                text(template.title).size(15).wrapping(Wrapping::None),
                                text(template.summary)
                                    .size(12)
                                    .wrapping(Wrapping::None)
                                    .style(settings_muted_text_style)
                            ]
                            .spacing(6)
                            .width(Length::Fill)
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                    )
                    .padding(16)
                    .width(Length::FillPortion(1))
                    .style(rounded_action_btn_style)
                    .on_press(Message::Settings(
                        message::SettingsMessage::CronTemplateSelected(template.id),
                    )),
                );
            }
            cards = cards.push(line);
        }
        cards.into()
    })
    .height(Length::Shrink)
    .into()
}

pub(super) fn overlay<'a>(_app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let close = Message::Settings(message::SettingsMessage::CronTemplatesToggled(false));
    let header =
        settings_modal_header(text("从模板添加").size(20), settings_close_button(close.clone()));
    let card =
        settings_modal_layout(header, view(), None).style(cron_modal_style).width(1000).height(700);
    settings_modal_overlay(Some(dialog), close, container(card).padding([10, 12]))
}
