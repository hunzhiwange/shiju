//! 新建与编辑共用的七类时间规则、有效期和预览。
use super::{at_field, cron_segment_style, primary_button, secondary_button};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, settings_muted_text_style, settings_select, settings_text_input,
};
use crate::app::message::SettingsMessage as S;
use crate::app::state::{CronJobDraft, CronScheduleDraft, CronScheduleRule as R, IntervalUnit};
use crate::app::{App, Message};
use chrono::{Datelike, NaiveDate, NaiveTime};
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

fn changed(editing: bool, draft: CronScheduleDraft) -> Message {
    Message::Settings(S::CronScheduleDraftChanged { editing, draft })
}

fn schedule(draft: &CronJobDraft) -> CronScheduleDraft {
    draft.schedule_draft.clone().unwrap_or_else(|| {
        let request = vw_gateway_client::CronPreviewRequest {
            schedule_kind: draft.schedule_kind.as_api_value().into(),
            schedule: Some(draft.schedule.clone()),
            at: Some(draft.at.clone()),
            every_ms: draft.every_ms.parse().ok(),
            ..Default::default()
        };
        CronScheduleDraft::from_request(&request).unwrap_or_default()
    })
}

fn time_of(rule: &R) -> NaiveTime {
    match rule {
        R::At { time, .. }
        | R::Daily { time }
        | R::Weekly { time, .. }
        | R::Biweekly { time, .. }
        | R::Monthly { time, .. }
        | R::Yearly { time, .. } => *time,
        _ => NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
    }
}

fn set_time(rule: &mut R, value: NaiveTime) {
    match rule {
        R::At { time, .. }
        | R::Daily { time }
        | R::Weekly { time, .. }
        | R::Biweekly { time, .. }
        | R::Monthly { time, .. }
        | R::Yearly { time, .. } => *time = value,
        _ => {}
    }
}

fn selected(rule: &R) -> usize {
    match rule {
        R::At { .. } | R::AbsoluteAt(_) => 0,
        R::Every { .. } => 1,
        R::Daily { .. } => 2,
        R::Weekly { .. } => 3,
        R::Biweekly { .. } => 4,
        R::Monthly { .. } => 5,
        R::Yearly { .. } => 6,
        R::Advanced(_) => 7,
    }
}

fn date_select<'a>(
    date: NaiveDate,
    on_change: impl Fn(NaiveDate) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let year = date.year();
    let month = date.month();
    let day = date.day();
    let valid_date = move |year, month, day| {
        (1..=day).rev().find_map(|d| NaiveDate::from_ymd_opt(year, month, d)).unwrap()
    };
    let year_change = on_change.clone();
    let month_change = on_change.clone();
    row![
        settings_select((year - 10)..=(year + 10), Some(year), move |y| year_change(valid_date(
            y, month, day
        )))
        .width(96),
        text("年"),
        settings_select(1..=12u32, Some(month), move |m| month_change(valid_date(year, m, day)))
            .width(70),
        text("月"),
        settings_select(
            (1..=31u32)
                .filter(|d| NaiveDate::from_ymd_opt(year, month, *d).is_some())
                .collect::<Vec<_>>(),
            Some(day),
            move |d| on_change(valid_date(year, month, d))
        )
        .width(70),
        text("日"),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .wrap()
    .into()
}

fn boundary_date(draft: &CronScheduleDraft, end: bool) -> Option<NaiveDate> {
    let patch = if end { draft.end_date } else { draft.start_date };
    if let Some(value) = patch {
        return value;
    }
    let absolute = if end { draft.ends_at } else { draft.starts_at }?;
    let timezone = draft.timezone.parse::<chrono_tz::Tz>().ok()?;
    let date = absolute.with_timezone(&timezone).date_naive();
    if end { date.pred_opt() } else { Some(date) }
}

fn toggle(values: &mut Vec<u32>, value: u32) {
    if values.contains(&value) {
        values.retain(|v| *v != value);
    } else {
        values.push(value);
        values.sort_unstable();
    }
}

pub(super) fn frequency_form<'a>(
    _app: &App,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let current = schedule(draft);
    let today = chrono::Utc::now().date_naive();
    let time = time_of(&current.rule);
    let rules = [
        R::At { date: today, time },
        R::Every { amount: 1, unit: IntervalUnit::Hours },
        R::Daily { time },
        R::Weekly { weekdays: vec![1], time },
        R::Biweekly { anchor: today, weekdays: vec![1], time },
        R::Monthly { days: vec![1], time },
        R::Yearly { month: 1, day: 1, time },
        R::Advanced("0 9 * * *".into()),
    ];
    let mut choices = row![].spacing(6);
    for (index, (label, rule)) in
        ["单次", "固定间隔", "每天", "每周", "每双周", "每月", "每年", "高级 Cron"]
            .into_iter()
            .zip(rules)
            .enumerate()
    {
        let mut next = current.clone();
        next.rule = rule;
        let active = index == selected(&current.rule);
        choices = choices.push(
            button(text(label).size(13))
                .padding([8, 10])
                .style(move |theme, status| cron_segment_style(theme, status, active))
                .on_press(changed(editing, next)),
        );
    }
    let mut form =
        column![text("时间规则").size(15), choices.wrap()].spacing(12).width(Length::Fill);
    match &current.rule {
        R::At { date, time } => {
            form = form.push(super::secondary_button(
                format!("{date} {} · 选择日期和时间", time.format("%H:%M")),
                Message::Settings(S::CronPickerOpen { editing }),
            ));
        }
        R::AbsoluteAt(value) => {
            form = form.push(at_field(_app, &draft.at, editing)).push(text(value.clone()));
        }
        R::Every { amount, unit } => {
            let mut next = current.clone();
            let input = settings_text_input("正整数", &amount.to_string())
                .on_input(move |value| {
                    let mut next = next.clone();
                    if let R::Every { amount, .. } = &mut next.rule {
                        *amount = value.parse().unwrap_or(0);
                    }
                    changed(editing, next)
                })
                .width(130);
            next = current.clone();
            let options = ["毫秒", "秒", "分钟", "小时", "天（24小时）"];
            let units = [
                IntervalUnit::Milliseconds,
                IntervalUnit::Seconds,
                IntervalUnit::Minutes,
                IntervalUnit::Hours,
                IntervalUnit::Days,
            ];
            let index = units.iter().position(|u| u == unit).unwrap();
            let unit = settings_select(options, Some(options[index]), move |label| {
                let mut next = next.clone();
                if let R::Every { unit, .. } = &mut next.rule {
                    *unit = units[options.iter().position(|v| *v == label).unwrap()];
                }
                changed(editing, next)
            });
            form = form.push(row![text("每"), input, unit].spacing(8).wrap());
        }
        R::Advanced(expression) => {
            let next = current.clone();
            form = form
                .push(settings_text_input("5 或 6 字段 Cron", expression).on_input(move |value| {
                    let mut next = next.clone();
                    next.rule = R::Advanced(value);
                    changed(editing, next)
                }))
                .push(
                    text("5 字段：分 时 日 月 星期；6 字段在最前增加秒。使用当前时区。").size(12),
                );
        }
        _ => {
            if let R::Weekly { weekdays, .. } | R::Biweekly { weekdays, .. } = &current.rule {
                let mut days = row![].spacing(4);
                for (index, label) in
                    ["周一", "周二", "周三", "周四", "周五", "周六", "周日"].iter().enumerate()
                {
                    let value = index as u32 + 1;
                    let active = weekdays.contains(&value);
                    let mut next = current.clone();
                    if let R::Weekly { weekdays, .. } | R::Biweekly { weekdays, .. } =
                        &mut next.rule
                    {
                        toggle(weekdays, value);
                    }
                    days = days.push(
                        button(text(*label).size(13))
                            .padding([8, 10])
                            .style(move |t, s| cron_segment_style(t, s, active))
                            .on_press(changed(editing, next)),
                    );
                }
                form = form.push(text("星期（至少选择一天）")).push(days.wrap());
            }
            if let R::Biweekly { anchor, .. } = &current.rule {
                let next = current.clone();
                let monday = anchor
                    .checked_sub_days(chrono::Days::new(
                        anchor.weekday().num_days_from_monday().into(),
                    ))
                    .unwrap_or(*anchor);
                form = form
                    .push(text(format!("双周锚点 · 活动周从 {monday} 开始，隔周跳过")).size(12))
                    .push(date_select(*anchor, move |date| {
                        let mut next = next.clone();
                        if let R::Biweekly { anchor, .. } = &mut next.rule {
                            *anchor = date;
                        }
                        changed(editing, next)
                    }));
            }
            if let R::Monthly { days, .. } = &current.rule {
                form = form.push(text("日期（至少选择一天）"));
                for week in (1..=31u32).collect::<Vec<_>>().chunks(7) {
                    let mut buttons = row![].spacing(4);
                    for value in week {
                        let active = days.contains(value);
                        let mut next = current.clone();
                        if let R::Monthly { days, .. } = &mut next.rule {
                            toggle(days, *value);
                        }
                        buttons = buttons.push(
                            button(text(value.to_string()).size(13))
                                .padding([8, 10])
                                .style(move |t, s| cron_segment_style(t, s, active))
                                .on_press(changed(editing, next)),
                        );
                    }
                    form = form.push(buttons.wrap());
                }
            }
            if let R::Yearly { month, day, .. } = &current.rule {
                let next = current.clone();
                let day_value = *day;
                let month_input = settings_select(1..=12u32, Some(*month), move |month| {
                    let mut next = next.clone();
                    if let R::Yearly { month: m, day, .. } = &mut next.rule {
                        *m = month;
                        *day = day_value;
                    }
                    changed(editing, next)
                });
                let next = current.clone();
                let day_input = settings_select(1..=31u32, Some(*day), move |day| {
                    let mut next = next.clone();
                    if let R::Yearly { day: d, .. } = &mut next.rule {
                        *d = day;
                    }
                    changed(editing, next)
                });
                form = form
                    .push(row![month_input, text("月"), day_input, text("日")].spacing(8).wrap());
            }
            let next = current.clone();
            form = form.push(super::date_time::time_controls(time, move |time| {
                let mut next = next.clone();
                set_time(&mut next.rule, time);
                changed(editing, next)
            }));
        }
    }
    let next = current.clone();
    form = form.push(text("时区（IANA）")).push(
        settings_text_input("UTC / Asia/Shanghai", &current.timezone).on_input(move |value| {
            let mut next = next.clone();
            next.timezone = value;
            changed(editing, next)
        }),
    );
    if current.timezone.parse::<chrono_tz::Tz>().is_err() {
        form = form.push(super::settings_error_banner("请选择有效的 IANA 时区"));
    }
    if !matches!(current.rule, R::At { .. } | R::AbsoluteAt(_)) {
        let start = boundary_date(&current, false);
        let end = boundary_date(&current, true);
        let ranged = start.is_some() || end.is_some();
        let mut long = current.clone();
        long.start_date = Some(None);
        long.end_date = Some(None);
        let mut range = current.clone();
        range.start_date = Some(Some(start.unwrap_or(today)));
        range.end_date = Some(Some(end.unwrap_or(today)));
        form = form.push(text("有效期")).push(
            row![
                secondary_button("长期", changed(editing, long)),
                secondary_button("日期范围", changed(editing, range))
            ]
            .spacing(8),
        );
        if ranged {
            for (label, is_end, date) in
                [("开始日期", false, start), ("结束日期（含当日）", true, end)]
            {
                let mut next = current.clone();
                next.start_date = Some(start);
                next.end_date = Some(end);
                form =
                    form.push(text(label)).push(date_select(date.unwrap_or(today), move |date| {
                        let mut next = next.clone();
                        if is_end {
                            next.end_date = Some(Some(date));
                        } else {
                            next.start_date = Some(Some(date));
                        }
                        changed(editing, next)
                    }));
            }
        }
        form = form.push(
            text(if ranged {
                "包含起止日期；结束后不启动新运行。"
            } else {
                "长期有效"
            })
            .size(12),
        );
    }
    if let Err(error) = current.request() {
        form = form.push(super::settings_error_banner(error));
    }
    form.into()
}

pub(super) fn save_button<'a>(draft: &CronJobDraft, editing: bool) -> Element<'a, Message> {
    let enabled = !draft.submitting
        && !draft.preview_loading
        && matches!(&draft.preview,Some(Ok(p)) if !p.candidates.is_empty());
    button(text(if draft.submitting { "正在保存…" } else { "保存" }).size(13))
        .padding([10, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::Settings(if editing {
            S::CronJobEditSave
        } else {
            S::CronAddSubmit
        })))
        .into()
}

fn rule_summary(rule: &R) -> String {
    match rule {
        R::At { date, time } => format!("单次 {date} {}", time.format("%H:%M")),
        R::AbsoluteAt(value) => format!("单次 {value}"),
        R::Every { amount, unit } => format!(
            "间隔 {amount} {}",
            match unit {
                IntervalUnit::Milliseconds => "毫秒",
                IntervalUnit::Seconds => "秒",
                IntervalUnit::Minutes => "分钟",
                IntervalUnit::Hours => "小时",
                IntervalUnit::Days => "天（24小时）",
            }
        ),
        R::Daily { time } => format!("每天 {}", time.format("%H:%M")),
        R::Weekly { weekdays, time } => format!("每周 {weekdays:?} {}", time.format("%H:%M")),
        R::Biweekly { anchor, weekdays, time } => {
            format!("每双周 {weekdays:?} {} · 锚点 {anchor}", time.format("%H:%M"))
        }
        R::Monthly { days, time } => format!("每月 {days:?} 日 {}", time.format("%H:%M")),
        R::Yearly { month, day, time } => {
            format!("每年 {month} 月 {day} 日 {}", time.format("%H:%M"))
        }
        R::Advanced(expression) => format!("高级 Cron {expression}"),
    }
}

pub(super) fn preview<'a>(draft: &CronJobDraft, editing: bool) -> Element<'a, Message> {
    let current = schedule(draft);
    let mut output = column![
        text(format!("规则：{} · 时区：{}", rule_summary(&current.rule), current.timezone))
            .size(12),
        text("短月缺日与非闰年 2 月 29 日跳过；DST 以网关预览为准。")
            .size(11)
            .style(settings_muted_text_style)
    ]
    .spacing(3);
    let status = if draft.preview_loading { "正在请求预览…" } else { "预览未来 5 次" };
    output = output.push(primary_button(
        status,
        if draft.preview_loading || draft.submitting {
            Message::None
        } else {
            Message::Settings(S::CronSchedulePreview { editing })
        },
    ));
    match &draft.preview {
        Some(Ok(preview)) if preview.candidates.is_empty() => {
            output = output.push(text("无未来触发，请修改规则或有效期。").size(12))
        }
        Some(Ok(preview)) => {
            for value in preview.candidates.iter().take(5) {
                let local = chrono::DateTime::parse_from_rfc3339(value)
                    .ok()
                    .and_then(|date| {
                        current.timezone.parse::<chrono_tz::Tz>().ok().map(|tz| {
                            date.with_timezone(&tz).format("%Y-%m-%d %H:%M %Z").to_string()
                        })
                    })
                    .unwrap_or(value.clone());
                output = output.push(text(local).size(12));
            }
        }
        Some(Err(error)) => output = output.push(text(format!("预览失败：{error}")).size(12)),
        None if !draft.preview_loading => {
            output = output.push(text("规则修改后需重新预览，再保存。").size(12))
        }
        _ => {}
    }
    output.into()
}
