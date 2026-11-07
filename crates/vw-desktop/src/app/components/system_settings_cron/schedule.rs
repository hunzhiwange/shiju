//! 新增和编辑任务共用的执行频率控件。
use super::{
    cron_segment_style, field_row, settings_muted_text_style, settings_select, text_field,
};
use crate::app::Message;
use crate::app::message::SettingsMessage;
use crate::app::state::{CronFrequency, CronJobDraft};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Theme};

fn replace_field(expression: &str, index: usize, value: String) -> String {
    let mut fields: Vec<_> = expression.split_whitespace().map(str::to_string).collect();
    fields[index] = value;
    fields.join(" ")
}

fn number_picker<'a>(
    expression: &'a str,
    index: usize,
    range: std::ops::RangeInclusive<u8>,
    selected: String,
    changed: fn(String) -> SettingsMessage,
) -> Element<'a, Message> {
    settings_select(
        range.map(|n| n.to_string()).collect::<Vec<_>>(),
        Some(selected),
        move |value| Message::Settings(changed(replace_field(expression, index, value))),
    )
    .width(80)
    .into()
}

pub(super) fn frequency_form(
    locale: vw_config_types::ui::DesktopLocale,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'_, Message> {
    let frequency_changed = if editing {
        SettingsMessage::CronJobEditFrequencyChanged
    } else {
        SettingsMessage::CronAddFrequencyChanged
    };
    let changed = if editing {
        SettingsMessage::CronJobEditScheduleChanged
    } else {
        SettingsMessage::CronAddScheduleChanged
    };
    let frequency = if CronFrequency::from_expression(&draft.schedule) == draft.frequency {
        draft.frequency
    } else {
        CronFrequency::Advanced
    };
    let mut choices = column![].spacing(8);
    for group in CronFrequency::ALL.chunks(3) {
        let mut buttons = row![].spacing(8);
        for &option in group {
            buttons = buttons.push(
                button(
                    text(crate::app::i18n::I18n::new(locale).__(&option.to_string()).to_string())
                        .size(13)
                        .center(),
                )
                .width(Length::Fill)
                .padding([10, 8])
                .style(move |theme: &Theme, status| {
                    cron_segment_style(theme, status, frequency == option)
                })
                .on_press(Message::Settings(frequency_changed(option))),
            );
        }
        choices = choices.push(buttons);
    }
    let mut form = column![field_row(
        crate::app::i18n::I18n::new(locale).__("执行频率"),
        crate::app::i18n::I18n::new(locale).__("选择常用频率，或使用高级表达式。"),
        choices
    )];
    if frequency == CronFrequency::Advanced {
        return form
            .push(field_row(
                crate::app::i18n::I18n::new(locale).__("Cron 表达式"),
                crate::app::i18n::I18n::new(locale)
                    .__("按 UTC 执行。5 字段依次为分、时、日、月、星期；6 字段在最前面增加秒。"),
                column![
                    text_field(
                        crate::app::i18n::I18n::new(locale).__("例如 0 9 * * MON-FRI"),
                        &draft.schedule,
                        move |v| { Message::Settings(changed(v)) }
                    ),
                    text(
                        crate::app::i18n::I18n::new(locale)
                            .__("示例：0 9 * * MON-FRI 表示工作日 UTC 09:00 执行。")
                    )
                    .size(11)
                    .style(settings_muted_text_style),
                ]
                .spacing(8),
            ))
            .into();
    }
    let fields: Vec<_> = draft.schedule.split_whitespace().collect();
    let expression = draft.schedule.as_str();
    let mut controls = row![].spacing(10).align_y(Alignment::Center);
    if frequency == CronFrequency::Minutes {
        let selected = fields[0].trim_start_matches("*/").to_string();
        controls = controls
            .push(text(crate::app::i18n::I18n::new(locale).__("每")))
            .push(
                settings_select(
                    (1..=59).map(|n| n.to_string()).collect::<Vec<_>>(),
                    Some(selected),
                    move |v| Message::Settings(changed(format!("*/{v} * * * *"))),
                )
                .width(80),
            )
            .push(text(crate::app::i18n::I18n::new(locale).__("分钟")));
    } else {
        if frequency == CronFrequency::Weekly {
            let days = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
            let labels = [
                crate::app::i18n::I18n::new(locale).__("周一"),
                crate::app::i18n::I18n::new(locale).__("周二"),
                crate::app::i18n::I18n::new(locale).__("周三"),
                crate::app::i18n::I18n::new(locale).__("周四"),
                crate::app::i18n::I18n::new(locale).__("周五"),
                crate::app::i18n::I18n::new(locale).__("周六"),
                crate::app::i18n::I18n::new(locale).__("周日"),
            ];
            let selected = days.iter().position(|d| *d == fields[4]).map(|i| labels[i]);
            controls = controls.push(
                settings_select(labels, selected, move |label| {
                    let index =
                        labels.iter().position(|value| *value == label).expect("weekday option");
                    Message::Settings(changed(replace_field(
                        expression,
                        4,
                        days[index].to_string(),
                    )))
                })
                .width(126),
            );
        }
        if frequency == CronFrequency::Monthly {
            controls = controls.push(
                row![
                    text(crate::app::i18n::I18n::new(locale).__("每月")),
                    number_picker(expression, 2, 1..=31, fields[2].to_string(), changed),
                    text(crate::app::i18n::I18n::new(locale).__("日")),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if frequency != CronFrequency::Hourly {
            controls = controls.push(
                row![
                    number_picker(expression, 1, 0..=23, fields[1].to_string(), changed),
                    text(crate::app::i18n::I18n::new(locale).__("时")),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        } else {
            controls = controls.push(text(crate::app::i18n::I18n::new(locale).__("每小时第")));
        }
        controls = controls.push(
            row![
                number_picker(expression, 0, 0..=59, fields[0].to_string(), changed),
                text(crate::app::i18n::I18n::new(locale).__("分")),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    let note = match frequency {
        CronFrequency::Minutes => crate::app::i18n::I18n::new(locale)
            .__("每小时从第 0 分起按所选分钟数触发，到下一小时重新计数。"),
        CronFrequency::Monthly => {
            crate::app::i18n::I18n::new(locale).__("时间使用 UTC；当月没有所选日期时跳过该月。")
        }
        _ => crate::app::i18n::I18n::new(locale).__("时间使用 UTC（北京时间为 UTC + 8 小时）。"),
    };
    form = form.push(field_row(
        crate::app::i18n::I18n::new(locale).__("执行时间"),
        note,
        controls.wrap(),
    ));
    form.into()
}
