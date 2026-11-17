use super::*;
use chrono::{FixedOffset, TimeZone};
use crate::center::overview::{self, Change};

#[test]
fn usage_replica_02_custom_validation_and_live_end() {
    let zone = FixedOffset::east_opt(8*3600).unwrap();
    let now = zone.with_ymd_and_hms(2026,11,17,12,0,0).unwrap().timestamp();
    assert!(custom(&zone, "2026-11-17 13:00", "2026-11-17 12:00", false, now).is_err());
    assert!(custom(&zone, "2026-10-01 12:00", "2026-11-17 12:00", false, now).is_err());
    let result = custom(&zone, "2026-11-17 11:00", "invalid", true, now).unwrap();
    assert_eq!(result, UsageRange::Custom { start_unix_seconds: Some(UsageUnixSeconds(now-3600)), end_unix_seconds: None, live_end_time: true });
}
#[test]
fn usage_replica_02_calendar_midnight_and_rolling() {
    let zone = FixedOffset::west_opt(5*3600).unwrap();
    let now = zone.with_ymd_and_hms(2026,11,17,0,15,0).unwrap();
    let range = Preset::Week.resolve(now).unwrap();
    assert_eq!(range, UsageRange::Absolute { start_unix_seconds: UsageUnixSeconds(zone.with_ymd_and_hms(2026,11,11,0,0,0).unwrap().timestamp()), end_unix_seconds: UsageUnixSeconds(now.timestamp()) });
    assert_eq!(Preset::Day.resolve(now).unwrap(), UsageRange::Rolling { seconds: 86400 });
}
#[test]
fn usage_replica_02_cancel_does_not_commit_and_month_wraps() {
    let mut picker = State::default();
    let range = UsageRange::All;
    assert!(picker.update(Event::Open, &range).is_none());
    picker.update(Event::Input(Field::Start, "invalid".into()), &range);
    assert!(picker.update(Event::Apply, &range).is_none());
    assert!(picker.error.is_some());
    assert!(picker.update(Event::Cancel, &range).is_none());
    assert_eq!(picker.preset, Some(Preset::All));
    picker.month = NaiveDate::from_ymd_opt(2026,1,1).unwrap();
    picker.update(Event::Month(-1), &range);
    assert_eq!(picker.month, NaiveDate::from_ymd_opt(2025,12,1).unwrap());
}
#[test]
fn usage_replica_02_cascade_and_clear_reset_cursor() {
    let mut query = overview::default_query();
    overview::apply(&mut query, Change::App("claude".into()));
    overview::apply(&mut query, Change::Provider("supplier".into()));
    overview::apply(&mut query, Change::Model("model".into()));
    query.cursor = Some("page2".into());
    overview::apply(&mut query, Change::App("codex".into()));
    assert!(query.cursor.is_none());
    assert!(query.filter.provider.is_none());
    assert!(query.filter.request_model.is_none());
    overview::apply(&mut query, Change::Clear);
    assert_eq!(query.filter, vw_api_usage::UsageFilter::default());
}
