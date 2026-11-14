//! Calendar series share the summary's snapshot, canonical filtering and accounting.
use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use vw_api_types::{id::ProjectScope, usage::*};

use super::summary::{UsageSummary, UsageSummaryError, accumulate, load};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSeriesInterval {
    Hour,
    Day,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSeriesBucket {
    /// Hour labels include the UTC offset to distinguish repeated wall-clock hours.
    pub label: String,
    /// Bounds are clipped to the requested half-open range, including partial days.
    pub start_unix_seconds: UsageUnixSeconds,
    pub end_unix_seconds: UsageUnixSeconds,
    /// Unknown costs remain counted separately from the known USD subtotal.
    pub summary: UsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSeries {
    pub range: UsageResolvedRange,
    pub interval: UsageSeriesInterval,
    pub buckets: Vec<UsageSeriesBucket>,
}

type Result<T> = std::result::Result<T, UsageSummaryError>;

fn timestamp(at: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(at, 0).ok_or(UsageSummaryError::InvalidInput)
}

/// Explicit interval is useful for charts; None selects hours for at most 24h.
/// Unbounded ranges begin on the first matching request's local calendar date.
pub fn get_daily_trends(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
    interval: Option<UsageSeriesInterval>,
) -> Result<UsageSeries> {
    let zone: Tz = range.time_zone.parse().map_err(|_| UsageSummaryError::InvalidInput)?;
    let end = range.end_unix_seconds.0;
    timestamp(end)?;
    if range.start_unix_seconds.is_some_and(|s| s.0 > end) {
        return Err(UsageSummaryError::InvalidInput);
    }
    if range.start_unix_seconds.is_some_and(|s| s.0 == end) {
        // Validate scope even for an empty live calendar range.
        if host.trim().is_empty() || host.contains('\0') {
            return Err(UsageSummaryError::InvalidInput);
        }
        return Ok(UsageSeries {
            range: range.clone(),
            interval: interval.unwrap_or(UsageSeriesInterval::Hour),
            buckets: Vec::new(),
        });
    }
    let rows = load(connection, scope, host, range, filter)?;
    let start = match range.start_unix_seconds {
        Some(start) => start.0,
        None => {
            let first = rows
                .iter()
                .map(|v| {
                    v.dimensions.started_at_unix_seconds.unwrap_or(v.dimensions.day_unix_seconds)
                })
                .min();
            let Some(first) = first else {
                return Ok(UsageSeries {
                    range: range.clone(),
                    interval: interval.unwrap_or(UsageSeriesInterval::Day),
                    buckets: Vec::new(),
                });
            };
            let date = timestamp(first)?.with_timezone(&zone).date_naive();
            super::ranges::start_of_day(date, zone).map_err(|_| UsageSummaryError::InvalidInput)?
        }
    };
    let interval = interval.unwrap_or(if end.checked_sub(start).is_some_and(|d| d <= 86400) {
        UsageSeriesInterval::Hour
    } else {
        UsageSeriesInterval::Day
    });
    let mut buckets = build_buckets(start, end, zone, interval)?;
    for row in rows {
        let d = &row.dimensions;
        let at = d.started_at_unix_seconds.unwrap_or(d.day_unix_seconds);
        let index = buckets.partition_point(|b| b.end_unix_seconds.0 <= at);
        let bucket =
            buckets.get_mut(index).ok_or(UsageSummaryError::ArchivePrecisionUnavailable)?;
        if at < bucket.start_unix_seconds.0 {
            return Err(UsageSummaryError::ArchivePrecisionUnavailable);
        }
        if d.started_at_unix_seconds.is_none() {
            // A legacy UTC-day aggregate cannot be split into local days or hours.
            let next = at.checked_add(86400).ok_or(UsageSummaryError::StorageUnavailable)?;
            if next > bucket.end_unix_seconds.0 {
                return Err(UsageSummaryError::ArchivePrecisionUnavailable);
            }
        }
        accumulate(&mut bucket.summary, &row)?;
    }
    Ok(UsageSeries { range: range.clone(), interval, buckets })
}

/// Heatmap cells use precisely the same daily boundaries and filters as trends.
pub fn get_usage_heatmap(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<UsageSeries> {
    get_daily_trends(connection, scope, host, range, filter, Some(UsageSeriesInterval::Day))
}

fn build_buckets(
    start: i64,
    end: i64,
    zone: Tz,
    interval: UsageSeriesInterval,
) -> Result<Vec<UsageSeriesBucket>> {
    let mut buckets = Vec::new();
    let mut at = start;
    while at < end {
        let local = timestamp(at)?.with_timezone(&zone);
        let next = match interval {
            UsageSeriesInterval::Day => {
                let mut date = local.date_naive();
                loop {
                    date = date.succ_opt().ok_or(UsageSummaryError::InvalidInput)?;
                    // Some IANA transitions skip an entire calendar date.
                    if let Ok(next) = super::ranges::start_of_day(date, zone) {
                        if next > at {
                            break next;
                        }
                    }
                }
            }
            UsageSeriesInterval::Hour => {
                let wall_hour = local
                    .naive_local()
                    .with_minute(0)
                    .and_then(|v| v.with_second(0))
                    .ok_or(UsageSummaryError::InvalidInput)?;
                let mut next = None;
                // Inspect both occurrences, including the current hour's second
                // occurrence on a fall-back day. Missing local hours are skipped.
                for h in 0..=48 {
                    let wall = wall_hour
                        .checked_add_signed(Duration::hours(h))
                        .ok_or(UsageSummaryError::InvalidInput)?;
                    let candidates = zone.from_local_datetime(&wall);
                    for candidate in
                        [candidates.earliest(), candidates.latest()].into_iter().flatten()
                    {
                        let t = candidate.timestamp();
                        if t > at && next.is_none_or(|old| t < old) {
                            next = Some(t);
                        }
                    }
                    if h >= 2 && next.is_some() {
                        break;
                    }
                }
                next.ok_or(UsageSummaryError::InvalidInput)?
            }
        }
        .min(end);
        let label = match interval {
            UsageSeriesInterval::Hour => local.format("%Y-%m-%d %H:%M %:z").to_string(),
            UsageSeriesInterval::Day => local.format("%Y-%m-%d").to_string(),
        };
        buckets.push(UsageSeriesBucket {
            label,
            start_unix_seconds: UsageUnixSeconds(at),
            end_unix_seconds: UsageUnixSeconds(next),
            summary: UsageSummary::default(),
        });
        at = next;
    }
    Ok(buckets)
}
