//! 网关和工具共用的时间输入与预览。
use crate::Schedule;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use runtime_time::zones::NamedTimeZone as Tz;

type Instant = DateTime<Utc>;
pub type WindowPatch = (Option<Option<Instant>>, Option<Option<Instant>>);

pub fn timezone(value: Option<&str>) -> Result<Tz, String> {
    value.unwrap_or("UTC").parse().map_err(|_| "timezone: invalid IANA timezone".into())
}

pub fn instant(value: &str, field: &str) -> Result<Instant, String> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|err| format!("{field}: Invalid RFC3339 time: {err}"))
}

pub fn date(value: &str, field: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|err| format!("{field}: invalid date: {err}"))
}

pub fn local_instant(local: NaiveDateTime, tz: Tz, field: &str) -> Result<Instant, String> {
    tz.from_local_datetime(&local)
        .single()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| format!("{field}: local time is nonexistent or ambiguous in {tz}"))
}

pub fn window_patch(
    starts_at: &Option<Option<String>>,
    ends_at: &Option<Option<String>>,
    start_date: &Option<Option<String>>,
    end_date: &Option<Option<String>>,
    tz_name: Option<&str>,
) -> Result<WindowPatch, String> {
    let tz = timezone(tz_name)?;
    if (starts_at.is_some() || ends_at.is_some()) && (start_date.is_some() || end_date.is_some()) {
        return Err("start_date/end_date: cannot be combined with starts_at/ends_at".into());
    }
    if (start_date.as_ref().is_some_and(Option::is_some)
        || end_date.as_ref().is_some_and(Option::is_some))
        && tz_name.is_none()
    {
        return Err("timezone: required with date window".into());
    }
    let boundary = |absolute: &Option<Option<String>>,
                    local: &Option<Option<String>>,
                    field: &str,
                    end: bool| {
        if let Some(value) = absolute {
            return value.as_ref().map(|s| instant(s, field)).transpose().map(Some);
        }
        let Some(value) = local else {
            return Ok(None);
        };
        let Some(value) = value else {
            return Ok(Some(None));
        };
        let mut day = date(value, field)?;
        if end {
            day = day.succ_opt().ok_or_else(|| format!("{field}: date overflow"))?;
        }
        let midnight =
            day.and_hms_opt(0, 0, 0).ok_or_else(|| format!("{field}: invalid midnight"))?;
        local_instant(midnight, tz, field).map(|at| Some(Some(at)))
    };
    Ok((
        boundary(starts_at, start_date, "starts_at/start_date", false)?,
        boundary(ends_at, end_date, "ends_at/end_date", true)?,
    ))
}

pub fn validate_candidates(
    schedule: &Schedule,
    start: Option<Instant>,
    end: Option<Instant>,
    now: Instant,
) -> Result<Vec<Instant>, String> {
    super::validate_schedule(schedule, now).map_err(|e| format!("schedule: {e}"))?;
    let mut candidates = Vec::new();
    let mut from = now;
    for _ in 0..5 {
        let Some(next) = crate::next_run_in_window(schedule, start, end, from)
            .map_err(|e| format!("schedule/window: {e}"))?
        else {
            break;
        };
        if next <= from {
            return Err("schedule: candidate must be in the future".into());
        }
        candidates.push(next);
        if matches!(schedule, Schedule::At { .. }) {
            break;
        }
        from = next;
    }
    if candidates.is_empty() {
        return Err("starts_at/ends_at: no occurrence within execution window".into());
    }
    Ok(candidates)
}

pub fn parse_local_at(value: &str, timezone_name: &str) -> Result<Instant, String> {
    let tz = timezone(Some(timezone_name))?;
    let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|e| format!("local_at: expected YYYY-MM-DDTHH:MM: {e}"))?;
    local_instant(local, tz, "local_at")
}

pub fn deserialize_window_field<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    serde::Deserialize::deserialize(deserializer).map(Some)
}
