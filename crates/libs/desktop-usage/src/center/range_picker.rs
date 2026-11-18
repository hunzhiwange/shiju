//! Local calendar drafts. Only a returned range commits a query; callers refresh on Some.
use chrono::{DateTime, Datelike, Duration, Local, Months, NaiveDate, NaiveDateTime, TimeZone};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length, Theme,
    widget::{Space, button, checkbox, column, container, row, rule, scrollable, text, text_input},
};
use vw_api_usage::{UsageRange, UsageUnixSeconds};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Today,
    Day,
    Week,
    Fortnight,
    Month,
    PreviousMonth,
    ThisMonth,
    All,
}
impl Preset {
    pub const ALL: [Self; 8] = [
        Self::Today,
        Self::Day,
        Self::Week,
        Self::Fortnight,
        Self::Month,
        Self::PreviousMonth,
        Self::ThisMonth,
        Self::All,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "当天",
            Self::Day => "近24小时",
            Self::Week => "近7天",
            Self::Fortnight => "近14天",
            Self::Month => "近30天",
            Self::PreviousMonth => "上月",
            Self::ThisMonth => "本月",
            Self::All => "全部",
        }
    }
    /// Calendar subtraction precedes timezone resolution, preserving DST boundaries.
    pub fn resolve<T: TimeZone>(self, now: DateTime<T>) -> Result<UsageRange, String> {
        if self == Self::All {
            return Ok(UsageRange::All);
        }
        if self == Self::Day {
            return Ok(UsageRange::Rolling { seconds: 86400 });
        }
        let today = now.date_naive();
        let month_start = today.with_day(1).unwrap();
        let zone = now.timezone();
        let date = match self {
            Self::PreviousMonth => month_start.checked_sub_months(Months::new(1)),
            Self::ThisMonth => Some(month_start),
            Self::Week => today.checked_sub_signed(Duration::days(6)),
            Self::Fortnight => today.checked_sub_signed(Duration::days(13)),
            Self::Month => today.checked_sub_signed(Duration::days(29)),
            _ => Some(today),
        }
        .ok_or("日期超出范围")?;
        let end = if self == Self::PreviousMonth {
            local_day_start(&zone, month_start)?
        } else {
            now.timestamp()
        };
        Ok(UsageRange::Absolute {
            start_unix_seconds: UsageUnixSeconds(local_day_start(&zone, date)?),
            end_unix_seconds: UsageUnixSeconds(end),
        })
    }
}

fn local_day_start(zone: &impl TimeZone, date: NaiveDate) -> Result<i64, String> {
    let midnight = date.and_hms_opt(0, 0, 0).unwrap();
    zone.from_local_datetime(&midnight)
        .earliest()
        // Some zones skip midnight. Find that date's first valid local minute.
        .or_else(|| {
            (1..1440).find_map(|minute| {
                zone.from_local_datetime(&(midnight + Duration::minutes(minute))).earliest()
            })
        })
        .map(|time| time.timestamp())
        .ok_or_else(|| "当地日期不存在".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Start,
    End,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Cancel,
    Preset(Preset),
    Input(Field, String),
    Active(Field),
    Live(bool),
    Month(i32),
    Today,
    Pick(NaiveDate),
    Apply,
    Tick,
}
#[derive(Clone, Debug)]
pub struct State {
    pub open: bool,
    pub preset: Option<Preset>,
    pub start: String,
    pub end: String,
    pub live: bool,
    pub active: Field,
    pub month: NaiveDate,
    pub error: Option<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            preset: Some(Preset::All),
            start: String::new(),
            end: String::new(),
            live: false,
            active: Field::Start,
            month: Local::now().date_naive().with_day(1).unwrap(),
            error: None,
        }
    }
}

pub fn parse_local<T: TimeZone>(zone: &T, input: &str) -> Result<i64, String> {
    let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M")
        .map_err(|_| "请输入 YYYY-MM-DD HH:MM".to_owned())?;
    zone.from_local_datetime(&naive)
        .single()
        .map(|time| time.timestamp())
        .ok_or_else(|| "该当地时间不存在或因夏令时重复，请选择其他时间".into())
}
pub fn custom<T: TimeZone>(
    zone: &T,
    start: &str,
    end: &str,
    live: bool,
    now: i64,
) -> Result<UsageRange, String> {
    let start = parse_local(zone, start)?;
    let end = if live { now } else { parse_local(zone, end)? };
    if start >= end {
        return Err("开始时间必须早于结束时间".into());
    }
    if end.saturating_sub(start) > 30 * 86400 {
        return Err("自定义范围最长30天".into());
    }
    Ok(UsageRange::Custom {
        start_unix_seconds: Some(UsageUnixSeconds(start)),
        end_unix_seconds: if live { None } else { Some(UsageUnixSeconds(end)) },
        live_end_time: live,
    })
}
impl State {
    /// Refresh callers resolve the remembered preset again so midnight never freezes bounds.
    pub fn refresh_range(&self, current: &UsageRange) -> Result<UsageRange, String> {
        self.preset.map_or_else(|| Ok(current.clone()), |preset| preset.resolve(Local::now()))
    }
    pub fn update(&mut self, event: Event, current: &UsageRange) -> Option<UsageRange> {
        let now = Local::now();
        self.error = None;
        match event {
            Event::Open => {
                let (start, end, live) = match current {
                    UsageRange::Absolute { start_unix_seconds: s, end_unix_seconds: e } => {
                        (s.0, e.0, false)
                    }
                    UsageRange::Custom {
                        start_unix_seconds: s,
                        end_unix_seconds: e,
                        live_end_time,
                    } => (
                        s.map_or(now.timestamp() - 86400, |s| s.0),
                        if *live_end_time {
                            now.timestamp()
                        } else {
                            e.map_or(now.timestamp(), |e| e.0)
                        },
                        *live_end_time,
                    ),
                    UsageRange::Rolling { seconds } => (
                        now.timestamp().saturating_sub((*seconds).min(i64::MAX as u64) as i64),
                        now.timestamp(),
                        false,
                    ),
                    _ => (now.timestamp() - 30 * 86400, now.timestamp(), false),
                };
                let format = |ts| {
                    Local
                        .timestamp_opt(ts, 0)
                        .single()
                        .unwrap_or(now)
                        .format("%Y-%m-%d %H:%M")
                        .to_string()
                };
                self.start = format(start);
                self.end = format(end);
                self.live = live;
                self.month = Local
                    .timestamp_opt(start, 0)
                    .single()
                    .unwrap_or(now)
                    .date_naive()
                    .with_day(1)
                    .unwrap();
                self.active = Field::Start;
                self.open = true;
            }
            Event::Cancel => self.open = false,
            Event::Preset(preset) => match preset.resolve(now) {
                Ok(range) => {
                    self.preset = Some(preset);
                    self.open = false;
                    return Some(range);
                }
                Err(error) => self.error = Some(error),
            },
            Event::Input(field, value) => {
                if field == Field::Start {
                    self.start = value;
                } else if !self.live {
                    self.end = value;
                }
                self.active = field;
            }
            Event::Active(field) => {
                if !self.live || field == Field::Start {
                    self.active = field;
                    let value = if field == Field::Start { &self.start } else { &self.end };
                    if let Ok(date) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M") {
                        self.month = date.date().with_day(1).unwrap();
                    }
                }
            }
            Event::Live(live) => {
                self.live = live;
                if live {
                    self.active = Field::Start;
                    self.end = now.format("%Y-%m-%d %H:%M").to_string();
                }
            }
            Event::Tick => {
                if self.open && self.live {
                    self.end = now.format("%Y-%m-%d %H:%M").to_string();
                }
            }
            Event::Month(delta) => {
                let index = i64::from(self.month.year()) * 12
                    + i64::from(self.month.month0())
                    + i64::from(delta);
                if let Ok(year) = i32::try_from(index.div_euclid(12)) {
                    if let Some(date) =
                        NaiveDate::from_ymd_opt(year, index.rem_euclid(12) as u32 + 1, 1)
                    {
                        self.month = date;
                    }
                }
            }
            Event::Today => self.month = now.date_naive().with_day(1).unwrap(),
            Event::Pick(day) => {
                let value = if self.active == Field::Start || self.live {
                    &mut self.start
                } else {
                    &mut self.end
                };
                let time = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
                    .map(|dt| dt.time())
                    .unwrap_or(now.time());
                *value = day.and_time(time).format("%Y-%m-%d %H:%M").to_string();
                self.month = day.with_day(1).unwrap();
                if !self.live {
                    if self.start > self.end {
                        if self.active == Field::Start {
                            self.end = self.start.clone();
                        } else {
                            self.start = self.end.clone();
                        }
                    }
                    self.active = Field::End;
                }
            }
            Event::Apply => {
                match custom(&Local, &self.start, &self.end, self.live, now.timestamp()) {
                    Ok(range) => {
                        self.preset = None;
                        self.open = false;
                        return Some(range);
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
        None
    }
    pub fn view(&self) -> Element<'_, Event> {
        let action = |label: String, event| {
            button(text(label).size(13))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(event)
        };
        let trigger = button(
            text(format!("{}  ▾", self.preset.map_or("自定义时间", Preset::label)))
                .size(13)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Center),
        )
        .height(SETTINGS_CONTROL_HEIGHT)
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press(if self.open { Event::Cancel } else { Event::Open });
        if !self.open {
            return trigger.into();
        }
        let mut presets = row![].spacing(5);
        for preset in Preset::ALL {
            let selected = self.preset == Some(preset);
            presets = presets.push(action(preset.label().into(), Event::Preset(preset)).style(
                move |theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    if selected {
                        style.background = Some(theme.extended_palette().primary.weak.color.into());
                        style.text_color = theme.extended_palette().primary.weak.text;
                    }
                    style
                },
            ));
        }
        let input = |label: &str, value: &str, field| {
            let active = self.active == field;
            let enabled = field != Field::End || !self.live;
            container(
                column![
                    action(label.into(), Event::Active(field))
                        .padding([2, 5])
                        .style(move |theme, status| {
                            let mut style = rounded_action_btn_style(theme, status);
                            style.border.width = 0.0;
                            style.background = None;
                            if active {
                                style.text_color = theme.palette().primary;
                            }
                            style
                        })
                        .on_press_maybe(enabled.then_some(Event::Active(field))),
                    text_input("YYYY-MM-DD HH:MM", value)
                        .style(settings_text_input_style)
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .on_input_maybe(enabled.then_some(move |value| Event::Input(field, value)))
                ]
                .spacing(4),
            )
            .padding(7)
            .width(Length::Fill)
            .style(move |theme: &Theme| {
                let palette = theme.palette();
                container::Style::default().border(iced::Border {
                    radius: 9.0.into(),
                    width: 1.0,
                    color: if active { palette.primary } else { palette.text.scale_alpha(0.16) },
                })
            })
        };
        let fields = column![
            text("自定义时间 · 最长 30 天").size(12).style(text::secondary),
            input("开始时间", &self.start, Field::Start),
            input("结束时间", &self.end, Field::End),
            checkbox(self.live)
                .label("结束时间跟随当前时刻")
                .size(16)
                .text_size(13)
                .style(settings_checkbox_style)
                .on_toggle(Event::Live)
        ]
        .spacing(10)
        .width(244);

        let mut calendar = column![
            row![
                action("‹".into(), Event::Month(-1)),
                Space::new().width(Length::Fill),
                action(self.month.format("%Y年%m月").to_string(), Event::Today),
                Space::new().width(Length::Fill),
                action("›".into(), Event::Month(1))
            ]
            .align_y(iced::Alignment::Center),
            ["日", "一", "二", "三", "四", "五", "六"].into_iter().fold(
                row![].spacing(2),
                |days, label| days.push(
                    container(text(label).size(12).style(text::secondary)).center_x(Length::Fill)
                )
            )
        ]
        .spacing(3);
        let first =
            self.month - Duration::days(i64::from(self.month.weekday().num_days_from_sunday()));
        let parse_date = |value: &str| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok().map(|time| time.date())
        };
        let (start, end) = (parse_date(&self.start), parse_date(&self.end));
        for week in 0..6 {
            let mut days = row![].spacing(2);
            for day in 0..7 {
                let date = first + Duration::days(week * 7 + day);
                let endpoint = start == Some(date) || end == Some(date);
                let in_range = start.zip(end).is_some_and(|(s, e)| s <= date && date <= e);
                let other_month = date.month() != self.month.month();
                days = days.push(
                    button(
                        text(date.day().to_string())
                            .size(13)
                            .width(Length::Fill)
                            .align_x(iced::alignment::Horizontal::Center),
                    )
                    .padding([4, 0])
                    .width(Length::Fill)
                    .height(29)
                    .style(move |theme, status| {
                        if endpoint {
                            return primary_action_btn_style(theme, status);
                        }
                        let mut style = rounded_action_btn_style(theme, status);
                        style.border = iced::border::rounded(5);
                        if in_range {
                            style.background =
                                Some(theme.extended_palette().background.strong.color.into());
                        } else if status == button::Status::Active {
                            style.background = None;
                        }
                        if other_month {
                            style.text_color = style.text_color.scale_alpha(0.55);
                        }
                        style
                    })
                    .on_press(Event::Pick(date)),
                );
            }
            calendar = calendar.push(days);
        }

        let form = row![
            fields,
            container(calendar).padding(10).width(296).style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.extended_palette().background.weak.color)
                    .border(iced::border::rounded(10))
            })
        ]
        .spacing(16)
        .wrap();
        let body = scrollable(container(form).padding([0, 7]).center_x(Length::Fill))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Fill)
            .width(Length::Fill);
        let footer = row![
            Space::new().width(Length::Fill),
            action("取消".into(), Event::Cancel),
            action("确定".into(), Event::Apply).style(primary_action_btn_style)
        ]
        .spacing(8);
        let mut panel = column![presets.wrap(), rule::horizontal(1), body].spacing(8);
        if let Some(error) = &self.error {
            panel = panel.push(text(error).size(12).style(text::danger));
        }
        let panel = settings_modal_card(panel.push(footer).height(Length::Fill))
            .padding([12, 14])
            .width(610)
            .max_height(372)
            .style(|theme| settings_modal_card_style(theme).background(theme.palette().background));
        desktop_ui_controls::overlays::BelowOverlay::new(trigger, panel)
            .show(true)
            .gap(8.0)
            .on_close(Event::Cancel)
            .capture_outside_click(true)
            .snap_within_viewport(true)
            .into()
    }
}

#[cfg(test)]
#[path = "../usage_replica_02_tests.rs"]
mod tests;
