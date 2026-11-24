use std::fmt;

use chrono::{DateTime, NaiveDateTime, NaiveTime, Timelike, Utc};

pub(super) const TIMEZONE: chrono_tz::Tz = chrono_tz::Asia::Shanghai;
pub(super) const WEEKDAYS: [&str; 7] = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
pub(super) const INTERVALS: [u32; 6] = [1, 5, 10, 15, 20, 30];

pub(super) fn future_once(value: NaiveDateTime) -> Result<DateTime<chrono_tz::Tz>, &'static str> {
    let time = value.and_local_timezone(TIMEZONE).single().ok_or("执行时间无效")?;
    if time <= Utc::now() {
        return Err("执行时间必须晚于当前时间");
    }
    Ok(time)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RepeatMode {
    Daily,
    Weekdays,
    Weekly,
    Monthly,
    Hourly,
    Minutes,
}

impl RepeatMode {
    pub(super) const ALL: [Self; 6] =
        [Self::Daily, Self::Weekdays, Self::Weekly, Self::Monthly, Self::Hourly, Self::Minutes];
}

impl fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Daily => "每天",
            Self::Weekdays => "每周一至周五",
            Self::Weekly => "每周",
            Self::Monthly => "每月",
            Self::Hourly => "每小时",
            Self::Minutes => "每隔几分钟",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepeatPattern {
    pub(super) mode: RepeatMode,
    pub(super) time: NaiveTime,
    pub(super) weekday: u32,
    pub(super) day: u32,
    pub(super) interval: u32,
}

impl Default for RepeatPattern {
    fn default() -> Self {
        Self {
            mode: RepeatMode::Daily,
            time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            weekday: 1,
            day: 1,
            interval: 15,
        }
    }
}

impl RepeatPattern {
    fn from_cron(cron: &str) -> Option<Self> {
        let mut fields = cron.split_whitespace();
        let (minute, hour, day, month, weekday) =
            (fields.next()?, fields.next()?, fields.next()?, fields.next()?, fields.next()?);
        if month != "*" || fields.next().is_some() {
            return None;
        }
        let mut pattern = Self::default();
        if hour == "*" && day == "*" && weekday == "*" {
            if minute == "*" {
                pattern.mode = RepeatMode::Minutes;
                pattern.interval = 1;
            } else if let Some(step) = minute.strip_prefix("*/") {
                pattern.mode = RepeatMode::Minutes;
                pattern.interval = step.parse().ok().filter(|n| (1..60).contains(n))?;
            } else {
                pattern.mode = RepeatMode::Hourly;
                pattern.time = pattern.time.with_minute(minute.parse().ok()?)?;
            }
            return Some(pattern);
        }
        pattern.time = NaiveTime::from_hms_opt(hour.parse().ok()?, minute.parse().ok()?, 0)?;
        pattern.mode = match (day, weekday) {
            ("*", "*") => RepeatMode::Daily,
            ("*", "1-5") => RepeatMode::Weekdays,
            ("*", weekday) => {
                pattern.weekday = weekday.parse::<u32>().ok().filter(|day| *day <= 7)? % 7;
                RepeatMode::Weekly
            }
            (day, "*") => {
                pattern.day = day.parse().ok().filter(|day| (1..=31).contains(day))?;
                RepeatMode::Monthly
            }
            _ => return None,
        };
        Some(pattern)
    }

    fn cron(self) -> String {
        let (hour, minute) = (self.time.hour(), self.time.minute());
        match self.mode {
            RepeatMode::Daily => format!("{minute} {hour} * * *"),
            RepeatMode::Weekdays => format!("{minute} {hour} * * 1-5"),
            RepeatMode::Weekly => format!("{minute} {hour} * * {}", self.weekday),
            RepeatMode::Monthly => format!("{minute} {hour} {} * *", self.day),
            RepeatMode::Hourly => format!("{minute} * * * *"),
            RepeatMode::Minutes if self.interval == 1 => "* * * * *".into(),
            RepeatMode::Minutes => format!("*/{} * * * *", self.interval),
        }
    }

    pub(super) fn description(self, i18n: desktop_i18n::I18n) -> String {
        let time = self.time.format("%H:%M");
        match self.mode {
            RepeatMode::Daily | RepeatMode::Weekdays => i18n.format("%s %s 执行", &[i18n.__(&self.mode.to_string()).to_owned().into(), format!("{}", time).into()]).expect("valid workbench template"),
            RepeatMode::Weekly => i18n.format("每%s %s 执行", &[i18n.__(WEEKDAYS[self.weekday as usize]).to_owned().into(), format!("{}", time).into()]).expect("valid workbench template"),
            RepeatMode::Monthly => i18n.format("每月 %s 日 %s 执行", &[format!("{}", self.day).into(), format!("{}", time).into()]).expect("valid workbench template"),
            RepeatMode::Hourly => i18n.format("每小时第 %s 分钟执行", &[format!("{}", self.time.minute()).into()]).expect("valid workbench template"),
            RepeatMode::Minutes => i18n.format("每 %s 分钟执行", &[format!("{}", self.interval).into()]).expect("valid workbench template"),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum RepeatSchedule {
    Pattern(RepeatPattern),
    Custom(String),
}

impl RepeatSchedule {
    pub(super) fn from_cron(cron: &str) -> Self {
        match RepeatPattern::from_cron(cron) {
            Some(pattern) => Self::Pattern(pattern),
            None => Self::Custom(cron.to_owned()),
        }
    }

    pub(super) fn cron(&self) -> String {
        match self {
            Self::Pattern(pattern) => pattern.cron(),
            Self::Custom(cron) => cron.trim().to_owned(),
        }
    }
}
