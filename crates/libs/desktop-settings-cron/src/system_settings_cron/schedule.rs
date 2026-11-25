//! 新建与编辑共用的七类时间规则、有效期和预览。
use super::{ViewData, cron_segment_style, date_time, editor_select, primary_button};
use crate::message::Message;
use crate::message::SettingsMessage as S;
use crate::state::{CronJobDraft, CronScheduleDraft, CronScheduleRule as R, IntervalUnit};
use chrono::{Datelike, NaiveDate, NaiveTime};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, icon_svg, primary_action_btn_style, rounded_action_btn_style,
    settings_muted_text_style, settings_text_input,
};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Element, Length};

fn changed(editing: bool, draft: CronScheduleDraft) -> Message {
    Message::Settings(S::CronScheduleDraftChanged { editing, draft })
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
        editor_select((year - 10)..=(year + 10), Some(year), move |y| year_change(valid_date(
            y, month, day
        )))
        .width(96),
        text(i18n.__("年")),
        editor_select(1..=12u32, Some(month), move |m| month_change(valid_date(year, m, day)))
            .width(70),
        text(i18n.__("月")),
        editor_select(
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
    let timezone = draft.timezone.parse::<runtime_time::zones::NamedTimeZone>().ok()?;
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
    app: &ViewData<'_>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let current = draft.effective_schedule();
    let now = chrono::Utc::now();
    let today = current
        .timezone
        .parse::<runtime_time::zones::NamedTimeZone>()
        .map_or_else(|_| now.date_naive(), |timezone| now.with_timezone(&timezone).date_naive());
    let weekday = today.weekday().number_from_monday();
    let time =
        current.rule.wall_time().unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let rules = [
        R::At { date: today, time },
        R::Every { amount: 1, unit: IntervalUnit::Hours },
        R::Daily { time },
        R::Weekly { weekdays: vec![weekday], time },
        R::Biweekly { anchor: today, weekdays: vec![weekday], time },
        R::Monthly { days: vec![today.day()], time },
        R::Yearly { month: today.month(), day: today.day(), time },
        R::Advanced("0 9 * * *".into()),
    ];
    let labels = [
        i18n.__("单次"),
        i18n.__("固定间隔"),
        i18n.__("每天"),
        i18n.__("每周"),
        i18n.__("每双周"),
        i18n.__("每月"),
        i18n.__("每年"),
        i18n.__("高级 Cron"),
    ];
    let repeat = selected(&current.rule);
    let repeat_caption = repeat_caption(i18n, labels[repeat], &current.rule);
    let ranged =
        boundary_date(&current, false).is_some() || boundary_date(&current, true).is_some();
    let picker = app
        .cron_settings
        .date_time_picker
        .as_ref()
        .filter(|picker| picker.editing == editing && picker.revision == draft.schedule_revision)
        .cloned();
    let toolbar_draft = current.clone();
    let toolbar = responsive(move |size| {
        let next = toolbar_draft.clone();
        let rules = rules.clone();
        let rule_field = editor_select([1, 0, 2, 3, 4, 5, 6, 7], Some(repeat), move |index| {
            let mut next = next.clone();
            if index == repeat {
                return Message::None;
            }
            next.rule = rules[index].clone();
            changed(editing, next)
        })
        .format_labels(move |index| labels[*index].to_owned())
        .trigger_label(format!("{} · {}", i18n.__("重复"), repeat_caption))
        .width(Length::Fill);
        let time_label = match &toolbar_draft.rule {
            R::At { date, time } => Some(format!("{date} {}", time.format("%H:%M"))),
            R::AbsoluteAt(value) => Some(
                local_schedule_time(value, &toolbar_draft.timezone, "%Y-%m-%d %H:%M")
                    .unwrap_or_else(|| value.clone()),
            ),
            R::Every { .. } | R::Advanced(_) => None,
            _ => Some(time.format("%H:%M").to_string()),
        };
        let mut fields: Vec<Element<'_, Message>> = vec![rule_field.into()];
        if let Some(label) = time_label {
            let trigger = button(
                row![
                    text(format!("{} · {label}", i18n.__("时间"))).size(13).width(Length::Fill),
                    icon_svg(if picker.is_some() { Icon::ChevronUp } else { Icon::ChevronDown }, 12.0),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding([8, 12])
            .height(SETTINGS_CONTROL_HEIGHT)
            .width(Length::Fill)
            .style(rounded_action_btn_style)
            .on_press(Message::Settings(if picker.is_some() {
                S::CronPickerCancel
            } else {
                S::CronPickerOpen { editing }
            }));
            fields.push(date_time::popover(
                i18n,
                picker.as_ref(),
                &toolbar_draft.timezone,
                trigger.into(),
            ));
        }
        if !matches!(toolbar_draft.rule, R::At { .. } | R::AbsoluteAt(_)) {
            let next = toolbar_draft.clone();
            let labels = [i18n.__("长期有效"), i18n.__("日期范围")];
            fields.push(
                editor_select([false, true], Some(ranged), move |range| {
                    if range == ranged {
                        return Message::None;
                    }
                    let mut next = next.clone();
                    next.start_date = Some(range.then_some(today));
                    next.end_date = Some(range.then_some(today));
                    changed(editing, next)
                })
                .format_labels(move |range| labels[usize::from(*range)].to_owned())
                .trigger_label(format!("{} · {}", i18n.__("有效期"), labels[usize::from(ranged)]))
                .width(Length::Fill)
                .into(),
            );
        }
        if size.width >= 560.0 {
            row(fields
                .into_iter()
                .map(|field| container(field).width(Length::FillPortion(1)).into()))
            .spacing(10)
            .into()
        } else {
            iced::widget::column(fields).spacing(8).into()
        }
    })
    .height(Length::Shrink);
    let mut form = column![toolbar].spacing(10).width(Length::Fill);
    match &current.rule {
        R::At { .. } | R::AbsoluteAt(_) => {}
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
            let unit = editor_select(options, Some(options[index]), move |label| {
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
                for (index, label) in weekday_labels(i18n).into_iter().enumerate() {
                    let value = index as u32 + 1;
                    let active = weekdays.contains(&value);
                    let mut next = current.clone();
                    if let R::Weekly { weekdays, .. } | R::Biweekly { weekdays, .. } =
                        &mut next.rule
                    {
                        toggle(weekdays, value);
                    }
                    days = days.push(
                        button(text(label).size(13))
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
                let month_input = editor_select(1..=12u32, Some(*month), move |month| {
                    let mut next = next.clone();
                    if let R::Yearly { month: m, day, .. } = &mut next.rule {
                        *m = month;
                        *day = day_value;
                    }
                    changed(editing, next)
                });
                let next = current.clone();
                let days =
                    (1..=31u32).filter(|day| NaiveDate::from_ymd_opt(2000, *month, *day).is_some());
                let day_input = editor_select(days, Some(*day), move |day| {
                    let mut next = next.clone();
                    if let R::Yearly { day: d, .. } = &mut next.rule {
                        *d = day;
                    }
                    changed(editing, next)
                });
                form = form
                    .push(row![month_input, text(i18n.__("月")), day_input, text(i18n.__("日"))].spacing(8).wrap());
            }
        }
    }
    if matches!(current.rule, R::Monthly { .. } | R::Yearly { .. }) {
        form = form.push(
            text(i18n.__("短月缺日与非闰年 2 月 29 日跳过；DST 以网关预览为准。"))
                .size(11)
                .style(settings_muted_text_style),
        );
    }
    if ranged && !matches!(current.rule, R::At { .. } | R::AbsoluteAt(_)) {
        let start = boundary_date(&current, false);
        let end = boundary_date(&current, true);
        for (label, is_end, date) in
            [(i18n.__("开始日期"), false, start), (i18n.__("结束日期（含当日）"), true, end)]
        {
            let mut next = current.clone();
            next.start_date = Some(start);
            next.end_date = Some(end);
            form = form.push(text(label)).push(date_select(
                i18n,
                date.unwrap_or(today),
                move |date| {
                    let mut next = next.clone();
                    if is_end {
                        next.end_date = Some(Some(date));
                    } else {
                        next.start_date = Some(Some(date));
                    }
                    changed(editing, next)
                },
            ));
        }
    }
    if current.timezone.parse::<runtime_time::zones::NamedTimeZone>().is_err() {
        form = form.push(super::settings_error_banner(i18n.__("请选择有效的 IANA 时区")));
    }
    if let Err(error) = current.request() {
        form = form.push(super::settings_error_banner(i18n.__(&error).to_owned()));
    }
    form.into()
}

fn weekday_labels(i18n: I18n) -> [&'static str; 7] {
    [
        i18n.__("周一"),
        i18n.__("周二"),
        i18n.__("周三"),
        i18n.__("周四"),
        i18n.__("周五"),
        i18n.__("周六"),
        i18n.__("周日"),
    ]
}

fn repeat_caption(i18n: I18n, label: &str, rule: &R) -> String {
    let details = match rule {
        R::Weekly { weekdays, .. } | R::Biweekly { weekdays, .. } => {
            let labels = weekday_labels(i18n);
            weekdays
                .iter()
                .filter_map(|day| labels.get(day.saturating_sub(1) as usize).copied())
                .collect::<Vec<_>>()
                .join(", ")
        }
        R::Monthly { days, .. } => days.iter().map(u32::to_string).collect::<Vec<_>>().join(", "),
        R::Yearly { month, day, .. } => format!("{month:02}-{day:02}"),
        _ => String::new(),
    };
    if details.is_empty() { label.to_owned() } else { format!("{label} {details}") }
}

pub(super) fn timezone_field(
    i18n: I18n,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'static, Message> {
    let current = draft.effective_schedule();
    let input =
        settings_text_input("UTC / Asia/Shanghai", &current.timezone).on_input(move |value| {
            let mut next = current.clone();
            next.timezone = value;
            changed(editing, next)
        });
    container(column![text(i18n.__("时区（IANA）")).size(15), input].spacing(6))
        .padding([14, 0])
        .width(Length::Fill)
        .into()
}

pub(super) fn save_button<'a>(
    locale: desktop_i18n::I18n,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let enabled = !draft.submitting && preview_ready(draft);
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

pub(super) fn preview_ready(draft: &CronJobDraft) -> bool {
    !draft.preview_loading && matches!(&draft.preview, Some(Ok(p)) if !p.candidates.is_empty())
}

fn local_schedule_time(value: &str, timezone: &str, format: &str) -> Option<String> {
    let date = chrono::DateTime::parse_from_rfc3339(value).ok()?;
    let timezone = timezone.parse::<runtime_time::zones::NamedTimeZone>().ok()?;
    Some(date.with_timezone(&timezone).format(format).to_string())
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
            desktop_i18n::__!(i18n, "每年 %u 月 %u 日 %s", *month, *day, (time.format("%H:%M")).to_string()).expect("valid settings message")
        }
        R::Advanced(expression) => desktop_i18n::__!(i18n, "高级 Cron %s", (expression).to_string()).expect("valid settings message"),
    }
}

pub(super) fn preview<'a>(i18n: I18n, draft: &CronJobDraft, editing: bool) -> Element<'a, Message> {
    let current = draft.effective_schedule();
    let mut output = column![
        text(
            desktop_i18n::__!(
                i18n,
                "规则：%s · 时区：%s",
                (rule_summary(i18n, &current.rule)).to_string(),
                (current.timezone).to_string()
            )
            .expect("valid settings message")
        )
        .size(13),
    ]
    .spacing(12)
    .width(Length::Fill);
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
            let times = column(preview.candidates.iter().take(5).map(|value| {
                let local = local_schedule_time(value, &current.timezone, "%Y-%m-%d %H:%M %Z")
                    .unwrap_or(value.clone());
                text(local).size(13).into()
            }))
            .spacing(8);
            output = output.push(super::settings_group_card(times));
        }
        Some(Err(error)) => output = output.push(text(desktop_i18n::__!(i18n, "预览失败：%s", (error).to_string()).expect("valid settings message")).size(12)),
        None if !draft.preview_loading => {
            output = output.push(text(i18n.__("规则修改后需重新预览，再保存。")).size(12))
        }
        _ => {}
    }
    output.into()
}
