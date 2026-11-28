//! 新建与编辑共用的七类时间规则、有效期和预览。
use super::{ViewData, date_time, editor_select, repeat_picker, validity_picker};
use crate::message::Message;
use crate::message::SettingsMessage as S;
use crate::state::{CronJobDraft, CronScheduleDraft, CronScheduleRule as R, IntervalUnit, local_today};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_svg, primary_action_btn_style,
    rounded_action_btn_style, settings_divider, settings_error_banner, settings_group_card,
    settings_muted_text_style, settings_text_input,
};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Element, Font, Length};
use runtime_time::zones::NamedTimeZone;

pub(super) fn changed(editing: bool, draft: CronScheduleDraft) -> Message {
    Message::Settings(S::CronScheduleDraftChanged { editing, draft })
}

fn selected(rule: &R) -> usize {
    match rule {
        R::Every { .. } => 0,
        R::At { .. } | R::AbsoluteAt(_) => 1,
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
            .width(70)
            .menu_min_width(104.0),
        text(i18n.__("月")),
        editor_select(
            (1..=31u32)
                .filter(|d| NaiveDate::from_ymd_opt(year, month, *d).is_some())
                .collect::<Vec<_>>(),
            Some(day),
            move |d| on_change(valid_date(year, month, d))
        )
        .width(70)
        .menu_min_width(104.0),
        text(i18n.__("日")),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center)
    .wrap()
    .into()
}

pub(super) fn frequency_form<'a>(
    app: &ViewData<'_>,
    draft: &'a CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let current = draft.effective_schedule();
    let today = local_today(&current.timezone);
    let weekday = today.weekday().number_from_monday();
    let time =
        current.rule.wall_time().unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let mut choices = [
        (R::Every { amount: 1, unit: IntervalUnit::Hours }, i18n.__("间隔")),
        (R::At { date: today, time }, i18n.__("单次")),
        (R::Daily { time }, i18n.__("每天")),
        (R::Weekly { weekdays: vec![weekday], time }, i18n.__("每周")),
        (R::Biweekly { anchor: today, weekdays: vec![weekday], time }, i18n.__("每双周")),
        (R::Monthly { days: vec![today.day()], time }, i18n.__("每月")),
        (R::Yearly { month: today.month(), day: today.day(), time }, i18n.__("每年")),
        (R::Advanced("0 9 * * *".into()), i18n.__("高级 Cron")),
    ];
    let repeat = selected(&current.rule);
    choices[repeat].0 = current.rule.clone();
    let repeat_caption = repeat_caption(i18n, choices[repeat].1, &current.rule);
    let open_picker = draft.editor_picker;
    let picker = app
        .cron_settings
        .date_time_picker
        .as_ref()
        .filter(|picker| picker.editing == editing && picker.revision == draft.schedule_revision)
        .cloned();
    let toolbar_draft = current.clone();
    let toolbar = responsive(move |size| {
        let rule_field = repeat_picker::view(
            i18n,
            &toolbar_draft,
            choices.clone(),
            repeat,
            desktop_i18n::__!(i18n, "重复：%s", repeat_caption.clone())
                .expect("valid settings message"),
            open_picker,
            editing,
        );
        let time_label = match &toolbar_draft.rule {
            R::At { date, time } => Some(format!("{date} {}", time.format("%H:%M"))),
            R::AbsoluteAt(value) => Some(
                local_schedule_time(value, &toolbar_draft.timezone)
                    .map(|local| local.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| value.clone()),
            ),
            R::Every { .. } | R::Advanced(_) => None,
            _ => Some(time.format("%H:%M").to_string()),
        };
        let mut fields = vec![rule_field];
        if let Some(label) = time_label {
            let trigger = repeat_picker::trigger(
                text(desktop_i18n::__!(i18n, "时间：%s", label).expect("valid settings message"))
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .width(Length::Fill),
                picker.is_some(),
                Message::Settings(if picker.is_some() {
                    S::CronPickerCancel
                } else {
                    S::CronPickerOpen { editing }
                }),
            );
            fields.push(date_time::popover(
                i18n,
                picker.as_ref(),
                &toolbar_draft.timezone,
                trigger.into(),
            ));
        }
        if toolbar_draft.rule.allows_validity() {
            fields.push(validity_picker::view(i18n, &toolbar_draft, today, open_picker, editing));
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
            form = form.push(
                row![text(i18n.__("每")), input, unit]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .wrap(),
            );
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
                    text(i18n.__("5 字段：分 时 日 月 星期；6 字段在最前增加秒。")).size(12),
                );
        }
        R::Biweekly { anchor, .. } => {
            let next = current.clone();
            let monday = anchor
                .checked_sub_days(chrono::Days::new(anchor.weekday().num_days_from_monday().into()))
                .unwrap_or(*anchor);
            form = form
                .push(
                    text(
                        desktop_i18n::__!(
                            i18n,
                            "双周锚点 · 活动周从 %s 开始，隔周跳过",
                            monday.to_string()
                        )
                        .expect("valid settings message"),
                    )
                    .size(12)
                    .style(settings_muted_text_style),
                )
                .push(date_select(i18n, *anchor, move |date| {
                    let mut next = next.clone();
                    if let R::Biweekly { anchor, .. } = &mut next.rule {
                        *anchor = date;
                    }
                    changed(editing, next)
                }));
        }
        _ => {}
    }
    if matches!(current.rule, R::Monthly { .. } | R::Yearly { .. }) {
        form = form.push(
            text(i18n.__("短月缺日与非闰年 2 月 29 日跳过；DST 以网关预览为准。"))
                .size(11)
                .style(settings_muted_text_style),
        );
    }
    if current.timezone.parse::<runtime_time::zones::NamedTimeZone>().is_err() {
        form = form.push(super::settings_error_banner(i18n.__("系统设置中的时区无效")));
    }
    if let Err(error) = current.request() {
        form = form.push(super::settings_error_banner(i18n.__(&error).to_owned()));
    }
    form.into()
}

pub(super) fn weekday_labels(i18n: I18n) -> [&'static str; 7] {
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
        R::Yearly { month, day, .. } => {
            desktop_i18n::__!(i18n, "%u月%u日", *month, *day).expect("valid settings message")
        }
        _ => String::new(),
    };
    if details.is_empty() { label.to_owned() } else { format!("{label} {details}") }
}

pub(super) fn save_button<'a>(
    locale: desktop_i18n::I18n,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    button(text(locale.__(if draft.submitting { "正在保存…" } else { "保存" })).size(13))
        .padding([10, 16])
        .style(primary_action_btn_style)
        .on_press_maybe((!draft.submitting).then_some(Message::Settings(if editing {
            S::CronJobEditSave
        } else {
            S::CronAddSubmit
        })))
        .into()
}

fn local_schedule_time(value: &str, timezone: &str) -> Option<DateTime<NamedTimeZone>> {
    let date = DateTime::parse_from_rfc3339(value).ok()?;
    let timezone = timezone.parse::<NamedTimeZone>().ok()?;
    Some(date.with_timezone(&timezone))
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
    let rule = rule_summary(i18n, &current.rule);
    let ready = draft.preview_ready();
    let can_preview = !draft.preview_loading && !draft.submitting;
    let action_label = i18n.__(if draft.preview_loading {
        "正在请求预览…"
    } else if ready {
        "重新预览"
    } else {
        "预览未来 5 次"
    });
    let summary = responsive(move |size| {
        let details = preview_property(i18n.__("规则"), rule.clone());
        let action = button(text(action_label).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(if ready { rounded_action_btn_style } else { primary_action_btn_style })
            .on_press_maybe(
                can_preview.then_some(Message::Settings(S::CronSchedulePreview { editing })),
            );
        if size.width >= 480.0 {
            row![details, action].spacing(18).align_y(Alignment::Center).into()
        } else {
            column![details, action].spacing(12).into()
        }
    })
    .height(Length::Shrink);
    let mut output =
        column![settings_group_card(summary).padding([16, 0])].spacing(14).width(Length::Fill);
    match &draft.preview {
        Some(Ok(preview)) if preview.candidates.is_empty() => {
            output = output.push(preview_hint(i18n.__("无未来触发，请修改规则或有效期。")));
        }
        Some(Ok(preview)) => {
            let title = container(text(i18n.__("执行时间")).size(13).font(preview_emphasis()))
                .padding([10, 0]);
            let mut times = column![title].width(Length::Fill);
            for value in preview.candidates.iter().take(5) {
                times = times.push(settings_divider()).push(preview_time(value, &current.timezone));
            }
            output = output.push(settings_group_card(times));
        }
        Some(Err(error)) => {
            let detail = desktop_i18n::__!(i18n, "预览失败：%s", i18n.__(error.as_str()))
                .expect("valid settings message");
            output = output.push(settings_error_banner(detail));
        }
        None => {}
    }
    output.into()
}

fn preview_property<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label).size(12).width(66).style(settings_muted_text_style),
        text(value)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .width(Length::Fill)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(8)
    .into()
}

fn preview_hint(message: &'static str) -> Element<'static, Message> {
    settings_group_card(text(message).size(13).style(settings_muted_text_style))
        .padding([14, 0])
        .into()
}

fn preview_time<'a>(value: &str, timezone: &str) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some(local) = local_schedule_time(value, timezone) {
        let time_format = if local.second() != 0 || local.nanosecond() != 0 {
            "%H:%M:%S%.f"
        } else {
            "%H:%M"
        };
        row![
            text(local.date_naive().to_string()).size(SETTINGS_CONTROL_TEXT_SIZE),
            text(local.format(time_format).to_string())
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .font(preview_emphasis())
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .wrapping(text::Wrapping::WordOrGlyph),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    } else {
        text(value.to_owned())
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .wrapping(text::Wrapping::WordOrGlyph)
            .into()
    };
    container(content).padding([11, 0]).width(Length::Fill).into()
}

fn preview_emphasis() -> Font {
    Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }
}
