//! Shared UTC query boundaries for logs, summaries, trends and heatmaps.
//!
//! Call once per query with a captured clock value. Rolling/custom-live ranges
//! must be resolved again on each query; calendar days use an explicit IANA zone.

use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use vw_api_types::usage::{
    UsageCalendarPeriod, UsageRange, UsageResolvedRange, UsageUnixSeconds,
};

/// Resolve the wire range to a half-open UTC interval. `time_zone` controls
/// presentation for non-calendar ranges; calendar ranges carry their own zone.
/// Invalid zones, timestamps, empty/reversed intervals and overflow are errors.
pub fn resolve_usage_range(
    range: &UsageRange,
    time_zone: &str,
    now: UsageUnixSeconds,
) -> Result<UsageResolvedRange, String> {
    let zone_name = match range {
        UsageRange::Calendar { time_zone, .. } => time_zone.as_str(),
        _ => time_zone,
    };
    let zone: Tz = zone_name
        .parse()
        .map_err(|_| format!("invalid IANA time zone: {zone_name}"))?;
    let clock = timestamp(now.0)?;
    let (start, end) = match range {
        UsageRange::All => (None, now.0),
        UsageRange::Absolute {
            start_unix_seconds,
            end_unix_seconds,
        } => (Some(start_unix_seconds.0), end_unix_seconds.0),
        UsageRange::Custom {
            start_unix_seconds,
            end_unix_seconds,
            live_end_time,
        } => {
            let start = match start_unix_seconds {
                Some(start) => start.0,
                None => subtract_seconds(now.0, 86_400)?,
            };
            let end = if *live_end_time {
                now.0
            } else {
                end_unix_seconds.unwrap_or(now).0
            };
            (Some(start), end)
        }
        UsageRange::Rolling { seconds } => {
            if *seconds == 0 {
                return Err("rolling range duration must be positive".into());
            }
            (Some(subtract_seconds(now.0, *seconds)?), now.0)
        }
        UsageRange::Calendar { period, .. } => {
            let today = clock.with_timezone(&zone).date_naive();
            let (date, end) = match period {
                UsageCalendarPeriod::Today => (today, now.0),
                UsageCalendarPeriod::Last7Days => (previous_date(today, 6)?, now.0),
                UsageCalendarPeriod::Last14Days => (previous_date(today, 13)?, now.0),
                UsageCalendarPeriod::Last30Days => (previous_date(today, 29)?, now.0),
                UsageCalendarPeriod::Yesterday => {
                    (previous_date(today, 1)?, start_of_day(today, zone)?)
                }
                UsageCalendarPeriod::ThisWeek => (
                    previous_date(today, u64::from(today.weekday().num_days_from_monday()))?,
                    now.0,
                ),
                UsageCalendarPeriod::ThisMonth => (
                    today.with_day(1).ok_or("calendar date out of range")?,
                    now.0,
                ),
            };
            (Some(start_of_day(date, zone)?), end)
        }
    };
    timestamp(end)?;
    if let Some(start) = start {
        timestamp(start)?;
        // A calendar query exactly at midnight is a valid empty live interval.
        if start > end || (start == end && !matches!(range, UsageRange::Calendar { .. })) {
            return Err("usage range start must precede end".into());
        }
    }
    Ok(UsageResolvedRange {
        start_unix_seconds: start.map(UsageUnixSeconds),
        end_unix_seconds: UsageUnixSeconds(end),
        time_zone: zone.name().to_owned(),
    })
}

fn timestamp(seconds: i64) -> Result<DateTime<Utc>, String> {
    DateTime::from_timestamp(seconds, 0).ok_or_else(|| "usage timestamp out of range".into())
}

fn subtract_seconds(now: i64, seconds: u64) -> Result<i64, String> {
    let seconds = i64::try_from(seconds).map_err(|_| "usage duration out of range")?;
    now.checked_sub(seconds)
        .ok_or_else(|| "usage range out of range".into())
}

fn previous_date(date: NaiveDate, days: u64) -> Result<NaiveDate, String> {
    date.checked_sub_days(Days::new(days))
        .ok_or_else(|| "calendar date out of range".into())
}

/// Choose the earliest occurrence of an ambiguous midnight. When a DST change
/// skips midnight, the first valid wall-clock second is the day's actual start.
/// A wholly skipped date is rejected instead of silently adding a calendar day.
pub(super) fn start_of_day(date: NaiveDate, zone: Tz) -> Result<i64, String> {
    let midnight = date.and_hms_opt(0, 0, 0).ok_or("invalid calendar date")?;
    for seconds in 0..86_400 {
        let local = midnight + chrono::Duration::seconds(seconds);
        if let Some(start) = zone.from_local_datetime(&local).earliest() {
            return Ok(start.timestamp());
        }
    }
    Err(format!("calendar date {date} does not exist in {zone}"))
}
