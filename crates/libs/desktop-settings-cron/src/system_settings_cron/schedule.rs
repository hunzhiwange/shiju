//! 新建与编辑共用的七类时间规则、有效期和预览。
use super::{at_field, cron_segment_style, primary_button, secondary_button};
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, settings_muted_text_style, settings_select, settings_text_input,
};
use crate::message::SettingsMessage as S;
use crate::state::{CronJobDraft, CronScheduleDraft, CronScheduleRule as R, IntervalUnit};
use crate::message::Message;
use super::ViewData;
use desktop_i18n::I18n;
use chrono::{Datelike, NaiveDate, NaiveTime};
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

fn changed(editing: bool, draft: CronScheduleDraft) -> Message {
    Message::Settings(S::CronScheduleDraftChanged { editing, draft })
}

fn schedule(draft: &CronJobDraft) -> CronScheduleDraft {
    draft.schedule_draft.clone().unwrap_or_else(|| {
        let request = gateway_cron::CronPreviewRequest {
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

fn date_select<'a>(i18n: I18n,
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
        text(i18n.__("年")),
        settings_select(1..=12u32, Some(month), move |m| month_change(valid_date(year, m, day)))
            .width(70),
        text(i18n.__("月")),
        settings_select(
            (1..=31u32)
                .filter(|d| NaiveDate::from_ymd_opt(year, month, *d).is_some())
                .collect::<Vec<_>>(),
            Some(day),
            move |d| on_change(valid_date(year, month, d))
        )
        .width(70),
        text(i18n.__("日")),
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
    _app: &ViewData<'_>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(_app.desktop_locale);
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
        [i18n.__("单次"), i18n.__("固定间隔"), i18n.__("每天"), i18n.__("每周"), i18n.__("每双周"), i18n.__("每月"), i18n.__("每年"), i18n.__("高级 Cron")]
            .into_iter()
            .zip(rules)
            .enumerate()
    {
        let mut next = current.clone();
        next.rule = rule;
        let active = index == selected(&current.rule);
        choices = choices.push(
            button(text(i18n.__(label)).size(13))
                .padding([8, 10])
                .style(move |theme, status| cron_segment_style(theme, status, active))
                .on_press(changed(editing, next)),
        );
    }
    let mut form =
        column![text(i18n.__("时间规则")).size(15), choices.wrap()].spacing(12).width(Length::Fill);
    match &current.rule {
        R::At { date, time } => {
            form = form.push(super::secondary_button(
                desktop_i18n::__!(i18n, "%s %s · 选择日期和时间", (date).to_string(), (time.format("%H:%M")).to_string()).expect("valid settings message"),
                Message::Settings(S::CronPickerOpen { editing }),
            ));
        }
        R::AbsoluteAt(value) => {
            form = form.push(at_field(_app, &draft.at, editing)).push(text(value.clone()));
        }
        R::Every { amount, unit } => {
            let mut next = current.clone();
            let input = settings_text_input(i18n.__("正整数"), &amount.to_string())
                .on_input(move |value| {
                    let mut next = next.clone();
                    if let R::Every { amount, .. } = &mut next.rule {
                        *amount = value.parse().unwrap_or(0);
                    }
                    changed(editing, next)
                })
                .width(130);
            next = current.clone();
            let options = [i18n.__("毫秒"), i18n.__("秒"), i18n.__("分钟"), i18n.__("小时"), i18n.__("天（24小时）")];
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
            form = form.push(row![text(i18n.__("每")), input, unit].spacing(8).wrap());
        }
        R::Advanced(expression) => {
            let next = current.clone();
            form = form
                .push(settings_text_input(i18n.__("5 或 6 字段 Cron"), expression).on_input(move |value| {
                    let mut next = next.clone();
                    next.rule = R::Advanced(value);
                    changed(editing, next)
                }))
                .push(
                    text(i18n.__("5 字段：分 时 日 月 星期；6 字段在最前增加秒。使用当前时区。")).size(12),
                );
        }
        _ => {
            if let R::Weekly { weekdays, .. } | R::Biweekly { weekdays, .. } = &current.rule {
                let mut days = row![].spacing(4);
                for (index, label) in
                    [i18n.__("周一"), i18n.__("周二"), i18n.__("周三"), i18n.__("周四"), i18n.__("周五"), i18n.__("周六"), i18n.__("周日")].iter().enumerate()
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
                        button(text(i18n.__(*label)).size(13))
                            .padding([8, 10])
                            .style(move |t, s| cron_segment_style(t, s, active))
                            .on_press(changed(editing, next)),
                    );
                }
                form = form.push(text(i18n.__("星期（至少选择一天）"))).push(days.wrap());
            }
            if let R::Biweekly { anchor, .. } = &current.rule {
                let next = current.clone();
                let monday = anchor
                    .checked_sub_days(chrono::Days::new(
                        anchor.weekday().num_days_from_monday().into(),
                    ))
                    .unwrap_or(*anchor);
                form = form
                    .push(text(desktop_i18n::__!(i18n, "双周锚点 · 活动周从 %s 开始，隔周跳过", (monday).to_string()).expect("valid settings message")).size(12))
                    .push(date_select(i18n, *anchor, move |date| {
                        let mut next = next.clone();
                        if let R::Biweekly { anchor, .. } = &mut next.rule {
                            *anchor = date;
                        }
                        changed(editing, next)
                    }));
            }
            if let R::Monthly { days, .. } = &current.rule {
                form = form.push(text(i18n.__("日期（至少选择一天）")));
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
                    .push(row![month_input, text(i18n.__("月")), day_input, text(i18n.__("日"))].spacing(8).wrap());
            }
            let next = current.clone();
            form = form.push(super::date_time::time_controls(i18n, time, move |time| {
                let mut next = next.clone();
                set_time(&mut next.rule, time);
                changed(editing, next)
            }));
        }
    }
    let next = current.clone();
    form = form.push(text(i18n.__("时区（IANA）"))).push(
        settings_text_input("UTC / Asia/Shanghai", &current.timezone).on_input(move |value| {
            let mut next = next.clone();
            next.timezone = value;
            changed(editing, next)
        }),
    );
    if current.timezone.parse::<chrono_tz::Tz>().is_err() {
        form = form.push(super::settings_error_banner(i18n.__("请选择有效的 IANA 时区")));
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
        form = form.push(text(i18n.__("有效期"))).push(
            row![
                secondary_button(i18n.__("长期"), changed(editing, long)),
                secondary_button(i18n.__("日期范围"), changed(editing, range))
            ]
            .spacing(8),
        );
        if ranged {
            for (label, is_end, date) in
                [(i18n.__("开始日期"), false, start), (i18n.__("结束日期（含当日）"), true, end)]
            {
                let mut next = current.clone();
                next.start_date = Some(start);
                next.end_date = Some(end);
                form =
                    form.push(text(i18n.__(label))).push(date_select(i18n, date.unwrap_or(today), move |date| {
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
                i18n.__("包含起止日期；结束后不启动新运行。")
            } else {
                i18n.__("长期有效")
            })
            .size(12),
        );
    }
    if let Err(error) = current.request() {
        form = form.push(super::settings_error_banner(i18n.__(error)));
    }
    form.into()
}

pub(super) fn save_button<'a>(
    locale: desktop_i18n::I18n,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let enabled = !draft.submitting
        && !draft.preview_loading
        && matches!(&draft.preview,Some(Ok(p)) if !p.candidates.is_empty());
    button(text(locale.__(if draft.submitting { "正在保存…" } else { "保存" })).size(13))
        .padding([10, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::Settings(if editing {
            S::CronJobEditSave
        } else {
            S::CronAddSubmit
        })))
        .into()
}

pub(super) fn rule_summary(i18n: I18n, rule: &R) -> String {
    match rule {
        R::At { date, time } => desktop_i18n::__!(i18n, "单次 %s %s", (date).to_string(), (time.format("%H:%M")).to_string()).expect("valid settings message"),
        R::AbsoluteAt(value) => desktop_i18n::__!(i18n, "单次 %s", (value).to_string()).expect("valid settings message"),
        R::Every { amount, unit } => desktop_i18n::__!(i18n, "间隔 %s %s", (amount).to_string(), (match unit {
                IntervalUnit::Milliseconds => i18n.__("毫秒"),
                IntervalUnit::Seconds => i18n.__("秒"),
                IntervalUnit::Minutes => i18n.__("分钟"),
                IntervalUnit::Hours => i18n.__("小时"),
                IntervalUnit::Days => i18n.__("天（24小时）"),
            }).to_string()).expect("valid settings message"),
        R::Daily { time } => desktop_i18n::__!(i18n, "每天 %s", (time.format("%H:%M")).to_string()).expect("valid settings message"),
        R::Weekly { weekdays, time } => desktop_i18n::__!(i18n, "每周 %s %s", format!("{:?}", weekdays), (time.format("%H:%M")).to_string()).expect("valid settings message"),
        R::Biweekly { anchor, weekdays, time } => {
            desktop_i18n::__!(i18n, "每双周 %s %s · 锚点 %s", format!("{:?}", weekdays), (time.format("%H:%M")).to_string(), (anchor).to_string()).expect("valid settings message")
        }
        R::Monthly { days, time } => desktop_i18n::__!(i18n, "每月 %s 日 %s", format!("{:?}", days), (time.format("%H:%M")).to_string()).expect("valid settings message"),
        R::Yearly { month, day, time } => {
            desktop_i18n::__!(i18n, "每年 %u 月 %u 日 %s", month, day, (time.format("%H:%M")).to_string()).expect("valid settings message")
        }
        R::Advanced(expression) => desktop_i18n::__!(i18n, "高级 Cron %s", (expression).to_string()).expect("valid settings message"),
    }
}

pub(super) fn preview<'a>(i18n: I18n, draft: &CronJobDraft, editing: bool) -> Element<'a, Message> {
    let current = schedule(draft);
    let mut output = column![
        text(desktop_i18n::__!(i18n, "规则：%s · 时区：%s", (rule_summary(i18n, &current.rule)).to_string(), (current.timezone).to_string()).expect("valid settings message"))
            .size(12),
        text(i18n.__("短月缺日与非闰年 2 月 29 日跳过；DST 以网关预览为准。"))
            .size(11)
            .style(settings_muted_text_style)
    ]
    .spacing(3);
    let status = if draft.preview_loading { i18n.__("正在请求预览…") } else { i18n.__("预览未来 5 次") };
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
            output = output.push(text(i18n.__("无未来触发，请修改规则或有效期。")).size(12))
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
        Some(Err(error)) => output = output.push(text(desktop_i18n::__!(i18n, "预览失败：%s", (error).to_string()).expect("valid settings message")).size(12)),
        None if !draft.preview_loading => {
            output = output.push(text(i18n.__("规则修改后需重新预览，再保存。")).size(12))
        }
        _ => {}
    }
    output.into()
}
