//! Local calendar drafts. Only a returned range commits a query; callers refresh on Some.
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use iced::{Element, Length, widget::{button, checkbox, column, row, text, text_input}};
use vw_api_usage::{UsageRange, UsageUnixSeconds};
use desktop_ui_controls::system_settings_common::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset { Today, Day, Week, Fortnight, Month, All }
impl Preset {
    pub const ALL: [Self; 6] = [Self::Today, Self::Day, Self::Week, Self::Fortnight, Self::Month, Self::All];
    pub fn label(self) -> &'static str { match self {
        Self::Today => "当天", Self::Day => "近24小时", Self::Week => "近7天",
        Self::Fortnight => "近14天", Self::Month => "近30天", Self::All => "全部",
    }}
    /// Calendar subtraction precedes timezone resolution, preserving DST boundaries.
    pub fn resolve<T: TimeZone>(self, now: DateTime<T>) -> Result<UsageRange, String> {
        if self == Self::All { return Ok(UsageRange::All); }
        if self == Self::Day { return Ok(UsageRange::Rolling { seconds: 86400 }); }
        let days = match self { Self::Week => 6, Self::Fortnight => 13, Self::Month => 29, _ => 0 };
        let date = now.date_naive().checked_sub_signed(Duration::days(days)).ok_or("日期超出范围")?;
        let start = now.timezone().from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap()).earliest()
            // Some zones skip midnight. Find that date's first valid local minute.
            .or_else(|| (1..1440).find_map(|minute| now.timezone().from_local_datetime(
                &(date.and_hms_opt(0, 0, 0).unwrap() + Duration::minutes(minute))).earliest()))
            .ok_or("当地日期不存在")?;
        Ok(UsageRange::Absolute { start_unix_seconds: UsageUnixSeconds(start.timestamp()), end_unix_seconds: UsageUnixSeconds(now.timestamp()) })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field { Start, End }
#[derive(Clone, Debug)]
pub enum Event { Open, Cancel, Preset(Preset), Input(Field, String), Active(Field), Live(bool), Month(i32), Today, Pick(NaiveDate), Apply, Tick }
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
    fn default() -> Self { Self { open: false, preset: Some(Preset::All), start: String::new(), end: String::new(), live: false, active: Field::Start, month: Local::now().date_naive().with_day(1).unwrap(), error: None } }
}

pub fn parse_local<T: TimeZone>(zone: &T, input: &str) -> Result<i64, String> {
    let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M").map_err(|_| "请输入 YYYY-MM-DD HH:MM".to_owned())?;
    zone.from_local_datetime(&naive).single().map(|time| time.timestamp())
        .ok_or_else(|| "该当地时间不存在或因夏令时重复，请选择其他时间".into())
}
pub fn custom<T: TimeZone>(zone: &T, start: &str, end: &str, live: bool, now: i64) -> Result<UsageRange, String> {
    let start = parse_local(zone, start)?;
    let end = if live { now } else { parse_local(zone, end)? };
    if start >= end { return Err("开始时间必须早于结束时间".into()); }
    if end.saturating_sub(start) > 30 * 86400 { return Err("自定义范围最长30天".into()); }
    Ok(UsageRange::Custom { start_unix_seconds: Some(UsageUnixSeconds(start)), end_unix_seconds: if live { None } else { Some(UsageUnixSeconds(end)) }, live_end_time: live })
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
                    UsageRange::Absolute { start_unix_seconds: s, end_unix_seconds: e } => (s.0, e.0, false),
                    UsageRange::Custom { start_unix_seconds: s, end_unix_seconds: e, live_end_time } => (s.map_or(now.timestamp()-86400, |s| s.0), if *live_end_time { now.timestamp() } else { e.map_or(now.timestamp(), |e| e.0) }, *live_end_time),
                    UsageRange::Rolling { seconds } => (now.timestamp().saturating_sub((*seconds).min(i64::MAX as u64) as i64), now.timestamp(), false),
                    _ => (now.timestamp()-30*86400, now.timestamp(), false),
                };
                let format = |ts| Local.timestamp_opt(ts, 0).single().unwrap_or(now).format("%Y-%m-%d %H:%M").to_string();
                self.start = format(start); self.end = format(end); self.live = live;
                self.month = Local.timestamp_opt(start, 0).single().unwrap_or(now).date_naive().with_day(1).unwrap();
                self.active = Field::Start; self.open = true;
            }
            Event::Cancel => self.open = false,
            Event::Preset(preset) => match preset.resolve(now) {
                Ok(range) => { self.preset = Some(preset); self.open = false; return Some(range); }
                Err(error) => self.error = Some(error),
            },
            Event::Input(field, value) => {
                if field == Field::Start { self.start = value; } else if !self.live { self.end = value; }
                self.active = field;
            }
            Event::Active(field) => if !self.live || field == Field::Start { self.active = field; },
            Event::Live(live) => { self.live = live; if live { self.active = Field::Start; self.end = now.format("%Y-%m-%d %H:%M").to_string(); } }
            Event::Tick => if self.open && self.live { self.end = now.format("%Y-%m-%d %H:%M").to_string(); },
            Event::Month(delta) => {
                let index = i64::from(self.month.year()) * 12 + i64::from(self.month.month0()) + i64::from(delta);
                if let Ok(year) = i32::try_from(index.div_euclid(12)) {
                    if let Some(date) = NaiveDate::from_ymd_opt(year, index.rem_euclid(12) as u32 + 1, 1) { self.month = date; }
                }
            }
            Event::Today => self.month = now.date_naive().with_day(1).unwrap(),
            Event::Pick(day) => {
                let value = if self.active == Field::Start || self.live { &mut self.start } else { &mut self.end };
                let time = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").map(|dt| dt.time()).unwrap_or(now.time());
                *value = day.and_time(time).format("%Y-%m-%d %H:%M").to_string();
                self.month = day.with_day(1).unwrap();
                if !self.live {
                    if self.start > self.end {
                        if self.active == Field::Start { self.end = self.start.clone(); }
                        else { self.start = self.end.clone(); }
                    }
                    self.active = Field::End;
                }
            }
            Event::Apply => match custom(&Local, &self.start, &self.end, self.live, now.timestamp()) {
                Ok(range) => { self.preset = None; self.open = false; return Some(range); }
                Err(error) => self.error = Some(error),
            },
        }
        None
    }
    pub fn view(&self) -> Element<'_, Event> {
        let action = |label: String, event| button(text(label).size(13)).padding([6,10]).style(rounded_action_btn_style).on_press(event);
        let trigger = action(self.preset.map_or("日历筛选", Preset::label).to_owned(), Event::Open);
        if !self.open { return trigger.into(); }
        let presets = Preset::ALL.into_iter().fold(row![].spacing(4), |r, p| r.push(action(p.label().into(), Event::Preset(p))));
        let input = |label, value, field| column![action(label, Event::Active(field)), text_input("YYYY-MM-DD HH:MM", value).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).on_input_maybe(if field == Field::End && self.live { None } else { Some(move |v| Event::Input(field, v)) })].spacing(4);
        let fields = column![text("自定义日期与时间，最长30天").size(12), input("开始时间".into(), &self.start, Field::Start), input("结束时间".into(), &self.end, Field::End), checkbox(self.live).label("结束时间跟随当前时刻").on_toggle(Event::Live), text(self.error.as_deref().unwrap_or("")).size(12)].spacing(8).width(280);
        let mut calendar = column![row![action("‹".into(), Event::Month(-1)), action(self.month.format("%Y年%m月").to_string(), Event::Today), action("›".into(), Event::Month(1))].spacing(4), row![text("日"),text("一"),text("二"),text("三"),text("四"),text("五"),text("六")].spacing(21)].spacing(4);
        let first = self.month - Duration::days(i64::from(self.month.weekday().num_days_from_sunday()));
        for week in 0..6 {
            let mut days = row![].spacing(2);
            for day in 0..7 {
                let date = first + Duration::days(week*7+day);
                let start = NaiveDateTime::parse_from_str(&self.start, "%Y-%m-%d %H:%M").ok().map(|dt| dt.date());
                let end = NaiveDateTime::parse_from_str(&self.end, "%Y-%m-%d %H:%M").ok().map(|dt| dt.date());
                let endpoint = start == Some(date) || end == Some(date);
                let in_range = start.zip(end).is_some_and(|(s,e)| s <= date && date <= e);
                days = days.push(button(text(date.day().to_string()).size(13)).padding([6, 8]).width(34)
                    .style(move |theme, status| {
                        let mut style = if endpoint { primary_action_btn_style(theme, status) } else { rounded_action_btn_style(theme, status) };
                        if !endpoint && in_range { style.background = Some(theme.extended_palette().background.strong.color.into()); }
                        if !endpoint && date.month() != self.month.month() { style.text_color = theme.extended_palette().background.strong.text.scale_alpha(0.5); }
                        style
                    }).on_press(Event::Pick(date)));
            }
            calendar = calendar.push(days);
        }
        let footer = row![action("取消".into(), Event::Cancel), button(text("确定").size(13)).padding([6,10]).style(primary_action_btn_style).on_press(Event::Apply)].spacing(8);
        column![trigger, settings_modal_card(column![presets, row![fields, calendar].spacing(16), footer].spacing(12)).width(Length::Shrink)].spacing(8).into()
    }
}

#[cfg(test)]
#[path = "../usage_replica_02_tests.rs"]
mod tests;
