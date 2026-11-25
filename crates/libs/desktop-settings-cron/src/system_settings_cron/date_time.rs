//! Cron 弹窗局部的日期时间临时选择；确认前不写入表单。
use super::{cron_modal_style, cron_segment_style, primary_button, secondary_button, small_button};
use crate::message::{Message, SettingsMessage};
use chrono::{Datelike, Days, Months, NaiveDate, NaiveTime, Timelike};
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::{
    settings_close_button, settings_modal_header, settings_modal_layout, settings_muted_text_style,
    settings_select,
};
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
    pub time_only: bool,
}

impl DateTimePicker {
    pub fn new(editing: bool, date: NaiveDate, time: NaiveTime) -> Self {
        Self {
            editing,
            date,
            time,
            month: date.with_day(1).unwrap(),
            revision: 0,
            time_only: false,
        }
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

fn time_controls<'a, M: Clone + 'a>(
    i18n: I18n,
    time: NaiveTime,
    roomy: bool,
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
        .format_labels(|hour| format!("{hour:02}"))
        .roomy_menu(roomy)
        .width(80),
        text(i18n.__("时")),
        settings_select(0..=59, Some(minute), move |minute| changed(
            NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
        ))
        .format_labels(|minute| format!("{minute:02}"))
        .roomy_menu(roomy)
        .width(80),
        text(i18n.__("分")),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

pub(super) fn popover<'a>(
    i18n: I18n,
    picker: Option<&DateTimePicker>,
    timezone: &str,
    trigger: Element<'a, Message>,
) -> Element<'a, Message> {
    let card = if let Some(picker) = picker {
        let note =
            desktop_i18n::__!(i18n, "时区：%s；确认后请预览校验执行时间。", timezone.to_owned())
                .expect("valid settings message");
        picker_card(i18n, picker, note, true).map(|event| {
            Message::Settings(match event {
                PickerEvent::Navigate(months) => SettingsMessage::CronPickerNavigate(months),
                PickerEvent::Date(date) => SettingsMessage::CronPickerDate(date),
                PickerEvent::Time(time) => {
                    SettingsMessage::CronPickerTime { hour: time.hour(), minute: time.minute() }
                }
                PickerEvent::Cancel => SettingsMessage::CronPickerCancel,
                PickerEvent::Confirm => SettingsMessage::CronPickerConfirm,
            })
        })
    } else {
        iced::widget::Space::new().into()
    };
    BelowOverlay::new(trigger, container(card).padding(8))
        .show(picker.is_some())
        .within_window()
        .align_right(true)
        .gap(2.0)
        .on_close(Message::Settings(SettingsMessage::CronPickerCancel))
        .dismiss_on_escape()
        .into()
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
    settings_modal_overlay_with_escape_and_backdrop_dismiss(
        base,
        on_message(PickerEvent::Cancel),
        picker_card(i18n, picker, note.into(), false).map(on_message),
    )
}

fn picker_card<'a>(
    i18n: I18n,
    picker: &DateTimePicker,
    note: String,
    floating: bool,
) -> Element<'a, PickerEvent> {
    let cancel = PickerEvent::Cancel;
    let title =
        if picker.time_only { i18n.__("选择时间") } else { i18n.__("选择日期和时间") };
    let header = settings_modal_header(text(title).size(20), settings_close_button(cancel));
    let mut body = column![].spacing(12);
    if !picker.time_only {
        let navigation = row![
            small_button("«", PickerEvent::Navigate(-12), false),
            small_button("‹", PickerEvent::Navigate(-1), false),
            text(
                desktop_i18n::__!(i18n, "%d年%u月", picker.month.year(), picker.month.month())
                    .expect("valid settings message")
            )
            .width(Length::Fill)
            .center(),
            small_button("›", PickerEvent::Navigate(1), false),
            small_button("»", PickerEvent::Navigate(12), false),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        let mut weekdays = row![].spacing(2);
        for label in [
            i18n.__("一"),
            i18n.__("二"),
            i18n.__("三"),
            i18n.__("四"),
            i18n.__("五"),
            i18n.__("六"),
            i18n.__("日"),
        ] {
            weekdays =
                weekdays.push(container(text(i18n.__(label)).size(13)).center_x(Length::Fill));
        }
        let mut calendar = column![navigation, weekdays].spacing(8);
        for week in picker.days().chunks(7) {
            let mut cells = row![].spacing(2);
            for date in week {
                let cell: Element<'_, PickerEvent> = if let Some(date) = date {
                    let selected = *date == picker.date;
                    let current = date.month() == picker.month.month();
                    let label = text(date.day().to_string()).size(13).center();
                    let label =
                        if current { label } else { label.style(settings_muted_text_style) };
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
        body = body.push(calendar);
    }
    let time = time_controls(i18n, picker.time, floating, PickerEvent::Time);
    let body = body.push(time);
    let body = if picker.time_only {
        body
    } else {
        body.push(
            text(
                desktop_i18n::__!(
                    i18n,
                    "已选 %s %s",
                    picker.date.to_string(),
                    picker.time.format("%H:%M").to_string()
                )
                .expect("valid settings message"),
            )
            .size(13),
        )
    };
    let body = body.push(text(note).size(12).style(settings_muted_text_style));
    let footer = row![
        secondary_button(i18n.__("取消"), cancel),
        primary_button(i18n.__("确认"), PickerEvent::Confirm),
    ]
    .spacing(10);
    let height = match (floating, picker.time_only) {
        (true, true) => 226,
        (true, false) => 470,
        (false, true) => 235,
        (false, false) => 540,
    };
    settings_modal_layout(header, body, Some(footer.into()))
        .style(cron_modal_style)
        .width(420)
        .height(height)
        .into()
}

#[cfg(test)]
#[path = "date_time_tests.rs"]
mod tests;

/// Legacy consumers without settings locale retain their existing default language.
pub fn picker_overlay<'a, M: Clone + 'a>(
    picker: &DateTimePicker,
    note: impl Into<String>,
    base: Element<'a, M>,
    on_message: impl Fn(PickerEvent) -> M + 'a,
) -> Element<'a, M> {
    picker_overlay_with_i18n(
        I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        picker,
        note,
        base,
        on_message,
    )
}
