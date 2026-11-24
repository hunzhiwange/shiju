//! Cron 弹窗局部的日期时间临时选择；确认前不写入表单。
use desktop_ui_controls::system_settings_common::{settings_close_button, settings_modal_header, settings_modal_layout, settings_muted_text_style, settings_select};
use desktop_i18n::I18n;
use super::{cron_segment_style, cron_modal_style, small_button, primary_button, secondary_button};
use crate::message::{Message, SettingsMessage};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveTime, Timelike};
use iced::widget::{button, container, text};
use iced::widget::{column, row};
use iced::{Alignment, Element, Length};

#[derive(Debug, Clone)]
pub struct DateTimePicker {
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

/// 选择器与周期表单共用同一套时分控件及设置样式。
pub fn time_controls<'a, M: Clone + 'a>(
    i18n: I18n,
    time: NaiveTime,
    changed: impl Fn(NaiveTime) -> M + Clone + 'a,
) -> Element<'a, M> {
    let hour = time.hour();
    let minute = time.minute();
    let hour_changed = changed.clone();
    row![
        text(i18n.__("时间")),
        settings_select(0..=23, Some(hour), move |hour| hour_changed(
            NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
        ))
        .width(80),
        text(i18n.__("时")),
        settings_select(0..=59, Some(minute), move |minute| changed(
            NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
        ))
        .width(80),
        text(i18n.__("分")),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

pub fn overlay<'a, M: Clone + 'a>(
    i18n: I18n,
    picker: &DateTimePicker,
    timezone: &str,
    base: Element<'a, M>,
    on_message: fn(Message) -> M,
) -> Element<'a, M> {
    picker_overlay_with_i18n(
        i18n,
        picker,
        desktop_i18n::__!(i18n, "时区：%s；确认后请预览校验执行时间。", (timezone).to_string()).expect("valid settings message"),
        base,
        move |event| {
            let message = match event {
                PickerEvent::Navigate(months) => SettingsMessage::CronPickerNavigate(months),
                PickerEvent::Date(date) => SettingsMessage::CronPickerDate(date),
                PickerEvent::Time(time) => {
                    SettingsMessage::CronPickerTime { hour: time.hour(), minute: time.minute() }
                }
                PickerEvent::Cancel => SettingsMessage::CronPickerCancel,
                PickerEvent::Confirm => SettingsMessage::CronPickerConfirm,
            };
            on_message(Message::Settings(message))
        },
    )
}

#[derive(Debug, Clone, Copy)]
pub enum PickerEvent {
    Navigate(i32),
    Date(NaiveDate),
    Time(NaiveTime),
    Cancel,
    Confirm,
}

pub fn picker_overlay_with_i18n<'a, M: Clone + 'a>(
    i18n: I18n,
    picker: &DateTimePicker,
    note: impl Into<String>,
    base: Element<'a, M>,
    on_message: impl Fn(PickerEvent) -> M + 'a,
) -> Element<'a, M> {
    use desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape_and_backdrop_dismiss;
    let cancel = PickerEvent::Cancel;
    let header =
        settings_modal_header(text(i18n.__("选择日期和时间")).size(20), settings_close_button(cancel));
    let navigation = row![
        small_button("«", PickerEvent::Navigate(-12), false),
        small_button("‹", PickerEvent::Navigate(-1), false),
        text(desktop_i18n::__!(i18n, "%d年%u月", picker.month.year(), picker.month.month()).expect("valid settings message"))
            .width(Length::Fill)
            .center(),
        small_button("›", PickerEvent::Navigate(1), false),
        small_button("»", PickerEvent::Navigate(12), false),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    let mut weekdays = row![].spacing(2);
    for label in [i18n.__("一"), i18n.__("二"), i18n.__("三"), i18n.__("四"), i18n.__("五"), i18n.__("六"), i18n.__("日")] {
        weekdays = weekdays.push(container(text(i18n.__(label)).size(13)).center_x(Length::Fill));
    }
    let mut calendar = column![navigation, weekdays].spacing(8);
    for week in picker.days().chunks(7) {
        let mut cells = row![].spacing(2);
        for date in week {
            let cell: Element<'_, PickerEvent> = if let Some(date) = date {
                let selected = *date == picker.date;
                let current = date.month() == picker.month.month();
                let label = text(date.day().to_string()).size(13).center();
                let label = if current { label } else { label.style(settings_muted_text_style) };
                button(label)
                    .width(Length::Fill)
                    .padding([8, 2])
                    .style(move |theme, status| cron_segment_style(theme, status, selected))
                    .on_press(PickerEvent::Date(*date))
                    .into()
            } else {
                container(text("")).width(Length::Fill).into()
            };
            cells = cells.push(cell);
        }
        calendar = calendar.push(cells);
    }
    let time = time_controls(i18n, picker.time, PickerEvent::Time);
    let body = column![
        calendar,
        time,
        text(desktop_i18n::__!(i18n, "已选 %s %s", (picker.date).to_string(), (picker.time.format("%H:%M")).to_string()).expect("valid settings message")).size(13),
        text(note.into()).size(12).style(settings_muted_text_style),
    ]
    .spacing(12);
    let footer =
        row![secondary_button(i18n.__("取消"), cancel), primary_button(i18n.__("确认"), PickerEvent::Confirm),]
            .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(cron_modal_style)
        .width(420)
        .height(540);
    let card: Element<'a, PickerEvent> = card.into();
    settings_modal_overlay_with_escape_and_backdrop_dismiss(
        base,
        on_message(cancel),
        card.map(on_message),
    )
}

#[cfg(test)]
#[path = "date_time_tests.rs"]
mod tests;

/// Legacy consumers without settings locale retain their existing default language.
pub fn picker_overlay<'a, M: Clone + 'a>(picker: &DateTimePicker, note: impl Into<String>, base: Element<'a, M>, on_message: impl Fn(PickerEvent) -> M + 'a) -> Element<'a, M> {
    picker_overlay_with_i18n(I18n::new(config_ui::DesktopLocale::SimplifiedChinese), picker, note, base, on_message)
}
