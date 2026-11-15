use super::*;

fn draft(rule: CronScheduleRule) -> CronScheduleDraft {
    CronScheduleDraft { rule, ..Default::default() }
}
fn time() -> NaiveTime {
    NaiveTime::from_hms_opt(9, 30, 0).unwrap()
}
fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
}

#[test]
fn wb_schedule_draft_roundtrips_all_frequencies() {
    for rule in [
        CronScheduleRule::At { date: date(), time: time() },
        CronScheduleRule::Every { amount: 3, unit: IntervalUnit::Hours },
        CronScheduleRule::Daily { time: time() },
        CronScheduleRule::Weekly { weekdays: vec![1, 3], time: time() },
        CronScheduleRule::Biweekly { anchor: date(), weekdays: vec![2, 5], time: time() },
        CronScheduleRule::Monthly { days: vec![1, 31], time: time() },
        CronScheduleRule::Yearly { month: 2, day: 29, time: time() },
    ] {
        let request = draft(rule).request().unwrap();
        let restored = CronScheduleDraft::from_request(&request).unwrap();
        assert_eq!(
            serde_json::to_value(restored.request().unwrap()).unwrap(),
            serde_json::to_value(request).unwrap()
        );
    }
}
#[test]
fn wb_schedule_draft_weekdays_sorted_deduplicated() {
    let r = draft(CronScheduleRule::Weekly { weekdays: vec![5, 1, 5], time: time() })
        .request()
        .unwrap();
    assert_eq!(r.schedule.as_deref(), Some("30 9 * * MON,FRI"));
}
#[test]
fn wb_schedule_draft_month_days_sorted_deduplicated() {
    let r =
        draft(CronScheduleRule::Monthly { days: vec![31, 1, 31], time: time() }).request().unwrap();
    assert_eq!(r.schedule.as_deref(), Some("30 9 1,31 * *"));
}
#[test]
fn wb_schedule_draft_impossible_year_date_rejected() {
    assert!(draft(CronScheduleRule::Yearly { month: 2, day: 30, time: time() }).request().is_err());
}
#[test]
fn wb_schedule_draft_leap_day_allowed() {
    assert_eq!(
        draft(CronScheduleRule::Yearly { month: 2, day: 29, time: time() })
            .request()
            .unwrap()
            .schedule
            .as_deref(),
        Some("30 9 29 2 *")
    );
}
#[test]
fn wb_schedule_draft_zero_and_overflow_rejected() {
    for amount in [0, u64::MAX] {
        assert!(
            draft(CronScheduleRule::Every { amount, unit: IntervalUnit::Days }).request().is_err()
        );
    }
}
#[test]
fn wb_schedule_draft_interval_units() {
    for (unit, ms) in [
        (IntervalUnit::Seconds, 1000),
        (IntervalUnit::Minutes, 60000),
        (IntervalUnit::Hours, 3600000),
        (IntervalUnit::Days, 86400000),
    ] {
        assert_eq!(
            draft(CronScheduleRule::Every { amount: 2, unit }).request().unwrap().every_ms,
            Some(2 * ms)
        );
    }
}
#[test]
fn wb_schedule_draft_local_at_timezone_passthrough() {
    let mut d = draft(CronScheduleRule::At { date: date(), time: time() });
    d.timezone = "Asia/Shanghai".into();
    let r = d.request().unwrap();
    assert_eq!(r.local_at.as_deref(), Some("2028-02-29T09:30"));
    assert_eq!(r.timezone.as_deref(), Some("Asia/Shanghai"));
    assert!(r.at.is_none());
}
#[test]
fn wb_schedule_draft_window_patch_three_states() {
    let mut d = CronScheduleDraft::default();
    assert!(d.request().unwrap().start_date.is_none());
    d.start_date = Some(None);
    assert_eq!(d.request().unwrap().start_date, Some(None));
    d.start_date = Some(Some(date()));
    assert_eq!(d.request().unwrap().start_date, Some(Some("2028-02-29".into())));
}
#[test]
fn wb_schedule_draft_advanced_preserves_original() {
    for expression in ["*/17 * * * *", "15 0 9 * * *", "0 0 9 * * * 2028", " 0 9 * * MON-FRI "] {
        let r = CronPreviewRequest {
            schedule_kind: "cron".into(),
            schedule: Some(expression.into()),
            ..Default::default()
        };
        let d = CronScheduleDraft::from_request(&r).unwrap();
        assert!(matches!(d.rule, CronScheduleRule::Advanced(_)));
        assert_eq!(d.request().unwrap().schedule, r.schedule);
    }
}
#[test]
fn wb_schedule_draft_switch_clears_incompatible_fields() {
    let mut d =
        draft(CronScheduleRule::Biweekly { anchor: date(), weekdays: vec![1], time: time() });
    d.rule = CronScheduleRule::Daily { time: time() };
    let r = d.request().unwrap();
    assert!(
        r.anchor_date.is_none()
            && r.weekdays.is_none()
            && r.every_ms.is_none()
            && r.local_at.is_none()
    );
}
#[test]
fn wb_schedule_draft_empty_selections_rejected() {
    for rule in [
        CronScheduleRule::Weekly { weekdays: vec![], time: time() },
        CronScheduleRule::Monthly { days: vec![], time: time() },
    ] {
        assert!(draft(rule).request().is_err());
    }
}

#[test]
fn wb_schedule_draft_existing_window_used_by_preview_until_date_patch() {
    let mut d = CronScheduleDraft {
        starts_at: Some("2028-01-01T00:00:00Z".parse().unwrap()),
        ends_at: Some("2029-01-01T00:00:00Z".parse().unwrap()),
        ..Default::default()
    };
    let r = d.request().unwrap();
    assert_eq!(r.starts_at, Some(Some("2028-01-01T00:00:00+00:00".into())));
    assert!(r.start_date.is_none());
    d.start_date = Some(None);
    // 网关禁止日期与绝对窗口混用，拒绝不完整转换，不能显示丢失结束边界的预览。
    assert!(d.request().is_err());
    d.end_date = Some(None);
    let r = d.request().unwrap();
    assert_eq!(r.start_date, Some(None));
    assert_eq!(r.end_date, Some(None));
    assert!(r.starts_at.is_none());
    assert!(r.ends_at.is_none());
}

#[test]
fn wb_schedule_acceptance_gateway_cron_normalization_preserves_edit_rules() {
    for (expression, expected) in [
        ("30 9 * * *", CronScheduleRule::Daily { time: time() }),
        ("30 9 * * MON,WED", CronScheduleRule::Weekly { weekdays: vec![1, 3], time: time() }),
        ("30 9 1,15 * *", CronScheduleRule::Monthly { days: vec![1, 15], time: time() }),
        ("30 9 29 2 *", CronScheduleRule::Yearly { month: 2, day: 29, time: time() }),
    ] {
        // The gateway stores the normalized six-field expression in its actual job JSON.
        let job: CronJobDto = serde_json::from_value(serde_json::json!({
            "id":"calendar-roundtrip", "schedule_kind":"cron",
            "expression":format!("0 {expression}"), "timezone":"Asia/Shanghai",
            "next_run":"2096-12-24T01:30:00+00:00", "enabled":true
        }))
        .unwrap();
        let restored = CronScheduleDraft::from_job(&job).unwrap();
        assert_eq!(restored.rule, expected);
        assert_eq!(restored.timezone, "Asia/Shanghai");
        assert_eq!(restored.request().unwrap().schedule.as_deref(), Some(expression));
    }
    for expression in ["15 30 9 * * *", "0 30 9 * * * 2096", "0 */40 * * * *"] {
        let request = CronPreviewRequest {
            schedule_kind: "cron".into(),
            schedule: Some(expression.into()),
            timezone: Some("UTC".into()),
            ..Default::default()
        };
        let restored = CronScheduleDraft::from_request(&request).unwrap();
        assert_eq!(restored.rule, CronScheduleRule::Advanced(expression.into()));
        assert_eq!(restored.request().unwrap().schedule.as_deref(), Some(expression));
    }
}
