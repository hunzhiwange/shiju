//! Tool schedule inputs are converted to the same domain representation used by the gateway.
use agent_cron::{CronJobPatch, Schedule, schedule::input};
use serde::Deserialize;
use serde_json::{Value, json};

fn schedule_schema() -> Value {
    json!({
        "type": "object",
        "description": "One rule per task. For daily, weekly, monthly, or yearly wall-clock schedules, use a Cron expression with an IANA time zone. Use a single rule with lists for multiple weekdays or month dates. Missing month dates and non-leap February 29 are skipped.",
        "oneOf": [
            {"properties": {
                "kind": {"const": "cron"},
                "expr": {"type": "string", "description": "5 fields (minute, hour, day, month, weekday), 6 with leading seconds, or 7 with trailing year. Examples: 0 9 * * MON,WED,FRI; 0 9 1,15 * *; 0 9 29 2 *"},
                "tz": {"type": ["string", "null"], "description": "IANA time zone; default UTC"}
            }, "required": ["kind", "expr"], "additionalProperties": false},
            {"properties": {
                "kind": {"const": "biweekly"},
                "anchor_date": {"type": "string", "format": "date", "description": "The local Monday of this date's week begins week zero"},
                "weekdays": {"type": "array", "minItems": 1, "uniqueItems": true, "items": {"type": "integer", "minimum": 1, "maximum": 7}, "description": "Monday=1, Sunday=7"},
                "hour": {"type": "integer", "minimum": 0, "maximum": 23},
                "minute": {"type": "integer", "minimum": 0, "maximum": 59},
                "timezone": {"type": "string", "description": "IANA time zone"}
            }, "required": ["kind", "anchor_date", "weekdays", "hour", "minute", "timezone"], "additionalProperties": false},
            {"properties": {
                "kind": {"const": "at"},
                "at": {"type": "string", "format": "date-time", "description": "Future RFC3339 instant with explicit offset"}
            }, "required": ["kind", "at"], "additionalProperties": false},
            {"properties": {
                "kind": {"const": "at"},
                "local_at": {"type": "string", "description": "Future local time in YYYY-MM-DDTHH:MM format"},
                "timezone": {"type": "string", "description": "IANA time zone for local_at"}
            }, "required": ["kind", "local_at", "timezone"], "additionalProperties": false},
            {"properties": {
                "kind": {"const": "every"},
                "every_ms": {"type": "integer", "minimum": 1, "description": "Elapsed milliseconds between runs; one day is 86400000 ms"}
            }, "required": ["kind", "every_ms"], "additionalProperties": false}
        ]
    })
}

pub(super) fn schedule_properties() -> serde_json::Map<String, Value> {
    serde_json::Map::from_iter([
        ("schedule".into(), schedule_schema()),
        (
            "starts_at".into(),
            json!({"type": ["string", "null"], "format": "date-time", "description": "Inclusive RFC3339 start. Null clears this boundary."}),
        ),
        (
            "ends_at".into(),
            json!({"type": ["string", "null"], "format": "date-time", "description": "Exclusive RFC3339 end. Null clears this boundary."}),
        ),
        (
            "start_date".into(),
            json!({"type": ["string", "null"], "format": "date", "description": "Inclusive local starting date. Use local dates or RFC3339 boundaries. Null clears this boundary."}),
        ),
        (
            "end_date".into(),
            json!({"type": ["string", "null"], "format": "date", "description": "Inclusive local ending date. Use local dates or RFC3339 boundaries. Null clears this boundary."}),
        ),
        (
            "timezone".into(),
            json!({"type": "string", "description": "IANA time zone for local window dates. Defaults to the explicit time zone in schedule, if supplied."}),
        ),
    ])
}

#[derive(Deserialize)]
struct WindowInput {
    #[serde(default, deserialize_with = "input::deserialize_window_field")]
    starts_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "input::deserialize_window_field")]
    ends_at: Option<Option<String>>,
    #[serde(default, deserialize_with = "input::deserialize_window_field")]
    start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "input::deserialize_window_field")]
    end_date: Option<Option<String>>,
    timezone: Option<String>,
}

pub(super) fn apply_schedule(args: &Value, patch: &mut CronJobPatch) -> Result<(), String> {
    let raw = args.get("schedule").filter(|value| !value.is_null());
    if let Some(raw) = raw {
        patch.schedule = Some(parse_schedule(raw)?);
    }
    let window = WindowInput::deserialize(args)
        .map_err(|error| format!("Invalid execution window: {error}"))?;
    let timezone = window.timezone.as_deref().or_else(|| {
        let raw = raw?;
        raw.get("timezone").or_else(|| raw.get("tz")).and_then(Value::as_str)
    });
    (patch.starts_at, patch.ends_at) = input::window_patch(
        &window.starts_at,
        &window.ends_at,
        &window.start_date,
        &window.end_date,
        timezone,
    )?;
    Ok(())
}

fn parse_schedule(raw: &Value) -> Result<Schedule, String> {
    if let Some(local_at) = raw.get("local_at") {
        if raw.get("kind").and_then(Value::as_str) != Some("at") || raw.get("at").is_some() {
            return Err("local_at requires kind=at and cannot be combined with at".into());
        }
        let local_at = local_at.as_str().ok_or("local_at: expected YYYY-MM-DDTHH:MM")?;
        let timezone = raw
            .get("timezone")
            .and_then(Value::as_str)
            .ok_or("timezone: required with local_at")?;
        return input::parse_local_at(local_at, timezone).map(|at| Schedule::At { at });
    }
    Schedule::deserialize(raw).map_err(|error| format!("Invalid schedule: {error}"))
}
