//! 双周使用当地日期差保持锚点相位，不按 ISO 周号或固定毫秒间隔计算。

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

pub(super) fn next_run(
    anchor_date: NaiveDate,
    weekdays: &[u32],
    hour: u32,
    minute: u32,
    timezone: &str,
    from: DateTime<Utc>,
) -> Result<DateTime<Utc>> {
    if weekdays.is_empty() || weekdays.iter().any(|day| !(1..=7).contains(day)) {
        anyhow::bail!("Invalid biweekly schedule: weekdays must contain values from 1 to 7");
    }
    if hour > 23 || minute > 59 {
        anyhow::bail!("Invalid biweekly schedule: hour must be 0–23 and minute 0–59");
    }
    let timezone: Tz = timezone.parse().context("Invalid IANA timezone for biweekly schedule")?;
    let anchor_monday = anchor_date
        .checked_sub_signed(Duration::days(i64::from(anchor_date.weekday().num_days_from_monday())))
        .context("Biweekly anchor week overflowed date range")?;
    let mut date = from.with_timezone(&timezone).date_naive().max(anchor_monday);
    loop {
        let days_since_anchor = date.signed_duration_since(anchor_monday).num_days();
        if days_since_anchor % 14 < 7 && weekdays.contains(&date.weekday().number_from_monday()) {
            let local = date.and_hms_opt(hour, minute, 0).context("Invalid biweekly local time")?;
            // 缺失的当地时间跳过；歧义时间的两个绝对时刻都参与严格后继选择。
            let candidate = match timezone.from_local_datetime(&local) {
                LocalResult::None => None,
                LocalResult::Single(value) => {
                    let value = value.with_timezone(&Utc);
                    (value > from).then_some(value)
                }
                LocalResult::Ambiguous(first, second) => [first, second]
                    .into_iter()
                    .map(|value| value.with_timezone(&Utc))
                    .filter(|value| *value > from)
                    .min(),
            };
            if let Some(candidate) = candidate {
                return Ok(candidate);
            }
        }
        date = date.succ_opt().context("No future occurrence for biweekly schedule")?;
    }
}
