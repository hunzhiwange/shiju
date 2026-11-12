//! 将网关日期字段转换为领域调度；预览与写入共享校验和窗口计算。
use super::super::super::types::{CronAddBody, CronUpdateBody};
use crate::app::agent::cron::{self, CronJob, Schedule};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};

type Instant = DateTime<Utc>;
pub(super) type WindowPatch = (Option<Option<Instant>>, Option<Option<Instant>>);

fn timezone(value: Option<&str>) -> Result<Tz, String> {
    value.unwrap_or("UTC").parse().map_err(|_| "timezone: invalid IANA timezone".into())
}

fn instant(value: &str, field: &str) -> Result<Instant, String> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|err| format!("{field}: Invalid RFC3339 time: {err}"))
}

fn date(value: &str, field: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|err| format!("{field}: invalid date: {err}"))
}

fn local_instant(local: NaiveDateTime, tz: Tz, field: &str) -> Result<Instant, String> {
    tz.from_local_datetime(&local)
        .single()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| format!("{field}: local time is nonexistent or ambiguous in {tz}"))
}

pub(super) fn cron_add_schedule(body: &CronAddBody) -> Result<Schedule, String> {
    let tz = timezone(body.timezone.as_deref())?;
    let kind = super::effective_schedule_kind(body);
    if !matches!(kind.as_str(), "cron" | "at" | "every" | "biweekly") {
        return Err(format!("Unsupported schedule_kind: {kind}"));
    }
    if body.at.is_some() && body.local_at.is_some() {
        return Err("local_at: cannot be combined with at".into());
    }
    if kind != "at" && (body.at.is_some() || body.local_at.is_some()) {
        return Err("at: only valid for at schedule".into());
    }
    if kind != "every" && body.every_ms.is_some() {
        return Err("every_ms: only valid for every schedule".into());
    }
    if kind != "biweekly"
        && (body.anchor_date.is_some()
            || body.weekdays.is_some()
            || body.hour.is_some()
            || body.minute.is_some())
    {
        return Err("anchor_date: biweekly fields require biweekly schedule".into());
    }
    match kind.as_str() {
        "cron" => {
            let expr = body
                .schedule
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or("Cron expression is required")?;
            Ok(Schedule::Cron {
                expr: cron::normalize_expression(expr).map_err(|e| e.to_string())?,
                tz: body.timezone.as_ref().map(|_| tz.name().to_string()),
            })
        }
        "at" => {
            let at = if let Some(local) = &body.local_at {
                if body.timezone.is_none() {
                    return Err("timezone: required with local_at".into());
                }
                let local = NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M")
                    .map_err(|e| format!("local_at: expected YYYY-MM-DDTHH:MM: {e}"))?;
                local_instant(local, tz, "local_at")?
            } else {
                instant(
                    body.at.as_deref().ok_or("RFC3339 time is required for at schedule")?,
                    "at",
                )?
            };
            Ok(Schedule::At { at })
        }
        "every" => {
            let every_ms = body
                .every_ms
                .filter(|value| *value > 0)
                .ok_or("every_ms must be greater than 0")?;
            Ok(Schedule::Every { every_ms })
        }
        "biweekly" => Ok(Schedule::Biweekly {
            anchor_date: date(
                body.anchor_date.as_deref().ok_or("anchor_date: required")?,
                "anchor_date",
            )?,
            weekdays: {
                let mut days = body.weekdays.clone().ok_or("weekdays: required")?;
                days.sort_unstable();
                days.dedup();
                days
            },
            hour: body.hour.ok_or("hour: required")?,
            minute: body.minute.ok_or("minute: required")?,
            timezone: tz.name().to_string(),
        }),
        other => Err(format!("Unsupported schedule_kind: {other}")),
    }
}

pub(super) fn window_patch(
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

pub(super) fn validate_candidates(
    schedule: &Schedule,
    start: Option<Instant>,
    end: Option<Instant>,
    now: Instant,
) -> Result<Vec<Instant>, String> {
    cron::validate_schedule(schedule, now).map_err(|e| format!("schedule: {e}"))?;
    let mut candidates = Vec::new();
    let mut from = now;
    for _ in 0..5 {
        let Some(next) = cron::next_run_in_window(schedule, start, end, from)
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

pub(super) fn schedule_json(schedule: &Schedule) -> Value {
    match schedule {
        Schedule::Cron { expr, tz } => {
            json!({"schedule_kind":"cron", "schedule":expr, "expression":expr, "timezone":tz.as_deref().unwrap_or("UTC")})
        }
        Schedule::Biweekly { anchor_date, weekdays, hour, minute, timezone } => {
            json!({"schedule_kind":"biweekly", "anchor_date":anchor_date, "weekdays":weekdays, "hour":hour, "minute":minute, "timezone":timezone})
        }
        Schedule::At { at } => {
            json!({"schedule_kind":"at", "at":at.to_rfc3339(), "timezone":"UTC"})
        }
        Schedule::Every { every_ms } => {
            json!({"schedule_kind":"every", "every_ms":every_ms, "timezone":"UTC"})
        }
    }
}

pub(super) fn preview(body: &CronAddBody, now: Instant) -> Result<Value, String> {
    let schedule = cron_add_schedule(body)?;
    let (start, end) = window_patch(
        &body.starts_at,
        &body.ends_at,
        &body.start_date,
        &body.end_date,
        body.timezone.as_deref(),
    )?;
    let start = start.flatten();
    let end = end.flatten();
    let candidates = validate_candidates(&schedule, start, end, now)?;
    let mut normalized = schedule_json(&schedule);
    normalized["starts_at"] = json!(start.map(|at| at.to_rfc3339()));
    normalized["ends_at"] = json!(end.map(|at| at.to_rfc3339()));
    Ok(
        json!({"timezone": normalized["timezone"], "normalized":normalized, "candidates":candidates.iter().map(DateTime::to_rfc3339).collect::<Vec<_>>()}),
    )
}

pub(super) fn cron_update_schedule(
    body: &CronUpdateBody,
    current: Option<&CronJob>,
) -> Result<Option<Schedule>, String> {
    let changed = body.schedule_kind.as_deref().is_some_and(|s| !s.trim().is_empty())
        || body.schedule.as_deref().is_some_and(|s| !s.trim().is_empty())
        || body.at.is_some()
        || body.local_at.is_some()
        || body.every_ms.is_some()
        || body.timezone.is_some()
        || body.anchor_date.is_some()
        || body.weekdays.is_some()
        || body.hour.is_some()
        || body.minute.is_some();
    if !changed {
        return Ok(None);
    }
    let mut input = if let Some(job) = current {
        let current_kind = match &job.schedule {
            Schedule::Cron { .. } => "cron",
            Schedule::Biweekly { .. } => "biweekly",
            Schedule::At { .. } => "at",
            Schedule::Every { .. } => "every",
        };
        let changing_kind = body.schedule_kind.as_deref().is_some_and(|kind| {
            let kind = match kind.trim() {
                "Cron" => "cron",
                "指定时间" => "at",
                "固定间隔" => "every",
                kind => kind,
            };
            !kind.is_empty() && kind != current_kind
        });
        if changing_kind {
            CronAddBody::default()
        } else {
            match &job.schedule {
                Schedule::Cron { expr, tz } => CronAddBody {
                    schedule_kind: Some("cron".into()),
                    schedule: Some(expr.clone()),
                    timezone: tz.clone(),
                    ..Default::default()
                },
                Schedule::Biweekly { anchor_date, weekdays, hour, minute, timezone } => {
                    CronAddBody {
                        schedule_kind: Some("biweekly".into()),
                        anchor_date: Some(anchor_date.to_string()),
                        weekdays: Some(weekdays.clone()),
                        hour: Some(*hour),
                        minute: Some(*minute),
                        timezone: Some(timezone.clone()),
                        ..Default::default()
                    }
                }
                // At stores an absolute instant. Its display UTC is not an input timezone for local_at.
                Schedule::At { at } => CronAddBody {
                    schedule_kind: Some("at".into()),
                    at: Some(at.to_rfc3339()),
                    ..Default::default()
                },
                Schedule::Every { every_ms } => CronAddBody {
                    schedule_kind: Some("every".into()),
                    every_ms: Some(*every_ms),
                    ..Default::default()
                },
            }
        }
    } else {
        CronAddBody::default()
    };
    input.schedule_kind = body.schedule_kind.clone().or(input.schedule_kind);
    input.schedule = body.schedule.clone().or(input.schedule);
    input.at = body.at.clone().or(input.at);
    input.local_at = body.local_at.clone().or(input.local_at);
    input.every_ms = body.every_ms.or(input.every_ms);
    input.timezone = body.timezone.clone().or(input.timezone);
    input.anchor_date = body.anchor_date.clone().or(input.anchor_date);
    input.weekdays = body.weekdays.clone().or(input.weekdays);
    input.hour = body.hour.or(input.hour);
    input.minute = body.minute.or(input.minute);
    if body.local_at.is_some() {
        input.at = body.at.clone();
    }
    cron_add_schedule(&input).map(Some)
}
