//! Cron 弹窗局部的日期时间临时选择；确认前不写入表单。
use super::{cron_segment_style, primary_button, secondary_button, small_button};
use crate::app::components::system_settings_common::{
    settings_close_button, settings_modal_header, settings_modal_layout, settings_muted_text_style,
    settings_select,
};
use crate::app::message::SettingsMessage;
use crate::app::{App, Message};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveTime};
use iced::widget::{button, container, text};
use iced::widget::{column, row};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone)]
pub(crate) struct DateTimePicker {
    pub editing: bool,
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub month: NaiveDate,
    pub revision: u64,
}

impl DateTimePicker {
    pub fn new(editing: bool, date: NaiveDate, time: NaiveTime) -> Self {
        Self { editing, date, time, month: date.with_day(1).unwrap(), revision: 0 }
    }

    pub fn navigate(&mut self, months: i32) {
        let next = if months >= 0 {
            self.month.checked_add_months(Months::new(months.unsigned_abs()))
        } else {
            self.month.checked_sub_months(Months::new(months.unsigned_abs()))
        };
        if let Some(next) = next {
            self.month = next;
        }
    }

    pub fn select(&mut self, date: NaiveDate) {
        self.date = date;
        self.month = date.with_day(1).unwrap();
    }

    pub fn set_time(&mut self, hour: u32, minute: u32) {
        if let Some(time) = NaiveTime::from_hms_opt(hour, minute, 0) {
            self.time = time;
        }
    }

    /// 周一开头，补齐首尾周；chrono 极值外的单元格为空。
    pub fn days(&self) -> Vec<Option<NaiveDate>> {
        let leading = self.month.weekday().num_days_from_monday();
        let last = self
            .month
            .checked_add_months(Months::new(1))
            .and_then(|next| next.pred_opt())
            .unwrap_or(NaiveDate::MAX);
        let count = (leading + last.day()).div_ceil(7) * 7;
        (0..count)
            .map(|index| {
                if index < leading {
                    self.month.checked_sub_days(Days::new((leading - index).into()))
                } else {
                    self.month.checked_add_days(Days::new((index - leading).into()))
                }
            })
            .collect()
    }
}

fn msg(message: SettingsMessage) -> Message {
    Message::Settings(message)
}

pub(super) fn overlay<'a>(
    picker: &DateTimePicker,
    timezone: &str,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::settings_modal_overlay_with_escape_and_backdrop_dismiss;
    use chrono::Timelike;
    let cancel = msg(SettingsMessage::CronPickerCancel);
    let header = settings_modal_header(
        text("选择日期和时间").size(20),
        settings_close_button(cancel.clone()),
    );
    let navigation = row![
        small_button("«", msg(SettingsMessage::CronPickerNavigate(-12)), false),
        small_button("‹", msg(SettingsMessage::CronPickerNavigate(-1)), false),
        text(format!("{}年{}月", picker.month.year(), picker.month.month()))
            .width(Length::Fill)
            .center(),
        small_button("›", msg(SettingsMessage::CronPickerNavigate(1)), false),
        small_button("»", msg(SettingsMessage::CronPickerNavigate(12)), false),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    let mut weekdays = row![].spacing(2);
    for label in ["一", "二", "三", "四", "五", "六", "日"] {
        weekdays = weekdays.push(container(text(label).size(13)).center_x(Length::Fill));
    }
    let mut calendar = column![navigation, weekdays].spacing(8);
    for week in picker.days().chunks(7) {
        let mut cells = row![].spacing(2);
        for date in week {
            let cell: Element<'_, Message> = if let Some(date) = date {
                let selected = *date == picker.date;
                let current = date.month() == picker.month.month();
                let label = text(date.day().to_string()).size(13).center();
                let label = if current { label } else { label.style(settings_muted_text_style) };
                button(label)
                    .width(Length::Fill)
                    .padding([8, 2])
                    .style(move |theme, status| cron_segment_style(theme, status, selected))
                    .on_press(msg(SettingsMessage::CronPickerDate(*date)))
                    .into()
            } else {
                container(text("")).width(Length::Fill).into()
            };
            cells = cells.push(cell);
        }
        calendar = calendar.push(cells);
    }
    let hour = picker.time.hour();
    let minute = picker.time.minute();
    let time = row![
        text("时间"),
        settings_select(0..=23, Some(hour), move |hour| msg(SettingsMessage::CronPickerTime {
            hour,
            minute
        }))
        .width(80),
        text("时"),
        settings_select(0..=59, Some(minute), move |minute| msg(SettingsMessage::CronPickerTime {
            hour,
            minute
        }))
        .width(80),
        text("分"),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let body = column![
        calendar,
        time,
        text(format!("已选 {} {}", picker.date, picker.time.format("%H:%M"))).size(13),
        text(format!("时区：{timezone}；确认后请预览校验执行时间。"))
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(12);
    let footer = row![
        secondary_button("取消", cancel.clone()),
        primary_button("确认", msg(SettingsMessage::CronPickerConfirm)),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::cron_modal_style)
        .width(420)
        .height(540);
    settings_modal_overlay_with_escape_and_backdrop_dismiss(base, cancel, card)
}

pub(super) fn view_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(picker) = &app.cron_settings.date_time_picker else {
        return base;
    };
    let draft =
        if picker.editing { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
    let timezone = draft.schedule_draft.as_ref().map(|d| d.timezone.as_str()).unwrap_or("UTC");
    overlay(picker, timezone, base)
}

#[cfg(test)]
#[path = "date_time_tests.rs"]
mod tests;
