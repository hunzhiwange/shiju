use std::fmt;

use chrono::Timelike;
use desktop_ui_controls::system_settings_common::settings_select;
use iced::Length;
use iced::widget::{column, row};

use super::task_schedule::{INTERVALS, RepeatMode, RepeatPattern, RepeatSchedule, WEEKDAYS};
use super::tasks::{CenterEvent, message};
use super::tasks_view::field;
use super::view::{Item, Tone, label};

pub(super) fn repeat_view(schedule: &RepeatSchedule, editable: bool) -> Item<'static> {
    let current = match schedule {
        RepeatSchedule::Pattern(pattern) => Some(*pattern),
        RepeatSchedule::Custom(_) => None,
    };
    let pattern = current.unwrap_or_default();
    let mode = field(
        "重复周期",
        settings_select(RepeatMode::ALL, current.map(|pattern| pattern.mode), move |mode| {
            message(CenterEvent::DraftRepeat(RepeatPattern { mode, ..pattern }))
        })
        .placeholder("保留原有时间安排")
        .width(Length::Fill)
        .disabled(!editable)
        .into(),
    );
    if current.is_none() {
        return column![
            mode,
            label("将按原有时间规则执行。选择新的周期会替换原规则。", 12, Tone::Muted),
        ]
        .spacing(8)
        .into();
    }

    let detail = match pattern.mode {
        RepeatMode::Weekly => Some(number_field(
            "星期",
            0..7,
            pattern.weekday,
            |value| WEEKDAYS[value as usize].to_owned(),
            move |weekday| RepeatPattern { weekday, ..pattern },
            editable,
        )),
        RepeatMode::Monthly => Some(number_field(
            "每月几号",
            1..=31,
            pattern.day,
            |value| format!("{value} 日"),
            move |day| RepeatPattern { day, ..pattern },
            editable,
        )),
        RepeatMode::Hourly => Some(number_field(
            "每小时第几分钟",
            0..60,
            pattern.time.minute(),
            |value| format!("{value} 分钟"),
            move |minute| RepeatPattern {
                time: pattern.time.with_minute(minute).unwrap(),
                ..pattern
            },
            editable,
        )),
        RepeatMode::Minutes => Some(number_field(
            "间隔",
            INTERVALS
                .into_iter()
                .chain((!INTERVALS.contains(&pattern.interval)).then_some(pattern.interval)),
            pattern.interval,
            |value| format!("{value} 分钟"),
            move |interval| RepeatPattern { interval, ..pattern },
            editable,
        )),
        RepeatMode::Daily | RepeatMode::Weekdays => None,
    };
    let mut form = column![row![mode, detail].spacing(8)].spacing(10);
    if !matches!(pattern.mode, RepeatMode::Hourly | RepeatMode::Minutes) {
        let hours = number_field(
            "执行小时",
            0..24,
            pattern.time.hour(),
            |value| format!("{value:02} 时"),
            move |hour| RepeatPattern { time: pattern.time.with_hour(hour).unwrap(), ..pattern },
            editable,
        );
        let minutes = number_field(
            "执行分钟",
            0..60,
            pattern.time.minute(),
            |value| format!("{value:02} 分"),
            move |minute| RepeatPattern {
                time: pattern.time.with_minute(minute).unwrap(),
                ..pattern
            },
            editable,
        );
        form = form.push(row![hours, minutes].spacing(8));
    }
    let hint = match pattern.mode {
        RepeatMode::Monthly if pattern.day > 28 => Some("没有这一天的月份会跳过。"),
        RepeatMode::Minutes => Some("每个整点重新计时。"),
        _ => None,
    };
    form.push(label(format!("{}（北京时间）", pattern.description()), 12, Tone::Muted))
        .push(hint.map(|hint| label(hint, 12, Tone::Muted)))
        .into()
}

#[derive(Clone, PartialEq, Eq)]
struct NumberOption {
    value: u32,
    label: String,
}

impl fmt::Display for NumberOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn number_field(
    name: &'static str,
    values: impl IntoIterator<Item = u32>,
    selected: u32,
    format: impl Fn(u32) -> String,
    update: impl Fn(u32) -> RepeatPattern + 'static,
    editable: bool,
) -> Item<'static> {
    let option = |value| NumberOption { value, label: format(value) };
    let selected = option(selected);
    field(
        name,
        settings_select(values.into_iter().map(option), Some(selected), move |choice| {
            message(CenterEvent::DraftRepeat(update(choice.value)))
        })
        .width(Length::Fill)
        .disabled(!editable)
        .into(),
    )
}
