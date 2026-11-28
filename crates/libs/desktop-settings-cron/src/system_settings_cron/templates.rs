//! 常用任务卡片只发送选择消息，确认保存前不会创建任务。
use super::*;
use crate::state::{CRON_TEMPLATES, CronTemplateId};
use desktop_assets::assets::Icon;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::icon_svg;
use iced::widget::{Space, svg};
use iced::widget::{column, responsive};

pub fn view(i18n: I18n) -> Element<'static, Message> {
    let grid = responsive(move |size| {
        let columns = super::grid_columns(size.width, 244.0);
        let mut cards = column![].spacing(11).width(Length::Fill);
        for group in CRON_TEMPLATES.chunks(columns) {
            let mut line = row![].spacing(11).width(Length::Fill);
            for template in group {
                line = line.push(
                    button(
                        row![
                            container(icon_svg(template_icon(template.id), 18.0).style(
                                |theme, _| svg::Style { color: Some(theme.palette().primary) }
                            ))
                            .center(38)
                            .style(|theme| container::Style {
                                background: Some(theme.palette().primary.scale_alpha(0.1).into()),
                                border: iced::Border { radius: 8.0.into(), ..Default::default() },
                                ..Default::default()
                            }),
                            column![
                                SingleLineLabel::new(i18n.__(template.title).to_owned(), 14.0)
                                    .font(list::bold()),
                                container(SingleLineLabel::new(
                                    i18n.__(template.summary).to_owned(),
                                    12.0
                                ))
                                .style(|theme| container::Style {
                                    text_color: settings_muted_text_style(theme).color,
                                    ..Default::default()
                                })
                            ]
                            .spacing(4)
                            .width(Length::Fill),
                        ]
                        .spacing(11)
                        .align_y(Alignment::Center),
                    )
                    .padding([11, 13])
                    .width(Length::FillPortion(1))
                    .style(|theme, status| {
                        let mut style = rounded_action_btn_style(theme, status);
                        let surface = list::surface(theme, false);
                        style.border = surface.border;
                        if status == button::Status::Active {
                            style.background = surface.background;
                        } else {
                            style.border.color = theme.palette().primary.scale_alpha(0.6);
                        }
                        style
                    })
                    .on_press(Message::Settings(
                        message::SettingsMessage::CronTemplateSelected(template.id),
                    )),
                );
            }
            for _ in group.len()..columns {
                line = line.push(Space::new().width(Length::FillPortion(1)));
            }
            cards = cards.push(line);
        }
        cards.into()
    })
    .height(Length::Shrink);
    let title = text(i18n.__("试试这些常用任务")).size(16).font(list::bold());
    column![settings_group_header(title), grid]
        .spacing(13)
        .width(Length::Fill)
        .into()
}

fn template_icon(id: CronTemplateId) -> Icon {
    match id {
        CronTemplateId::AiNews => Icon::FileText,
        CronTemplateId::English => Icon::Type,
        CronTemplateId::Story => Icon::Journals,
        CronTemplateId::WeeklyReport => Icon::Clipboard,
        CronTemplateId::Movie => Icon::Image,
        CronTemplateId::History => Icon::Calendar,
        CronTemplateId::Why => Icon::QuestionCircle,
        CronTemplateId::Parents => Icon::Person,
        CronTemplateId::Health => Icon::Plus,
        CronTemplateId::Interview => Icon::ChatTextFill,
        CronTemplateId::Meeting => Icon::CheckSquare,
        CronTemplateId::Wallpaper => Icon::Palette,
    }
}

pub(super) fn overlay<'a, M: Clone + 'static>(
    i18n: I18n,
    dialog: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let close = Message::Settings(message::SettingsMessage::CronTemplatesToggled(false));
    let header = settings_modal_header(
        text(i18n.__("从模板添加")).size(20),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_layout(header, view(i18n), None)
        .style(cron_modal_style)
        .width(1050)
        .height(630);
    desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape(
        dialog,
        map(close),
        Element::<Message>::from(container(card).padding([10, 12])).map(map),
    )
}

#[cfg(test)]
#[path = "templates_tests.rs"]
mod tests;
