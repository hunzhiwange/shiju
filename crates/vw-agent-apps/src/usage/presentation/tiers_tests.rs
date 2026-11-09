use super::tiers::{QuotaDisplay, QuotaTone, tier_card_rows, tier_lines};
use vw_api_types::apps_usage::UsageQuotaWindow;

fn window(name: &str, utilization: Option<f64>) -> UsageQuotaWindow {
    UsageQuotaWindow {
        name: name.into(),
        plan_label: None,
        utilization: utilization.map(|value| value.try_into().unwrap()),
        used: None,
        total: None,
        remaining: None,
        unit: None,
        resets_at: None,
    }
}

#[test]
fn usage_presentation_tiers_returns_expected_result() {
    let mut weekly = window("seven_day", Some(80.5));
    weekly.resets_at = Some("2026-11-11T00:00:00Z".into());
    let windows = [
        window("monthly", Some(95.0)),
        weekly,
        window("five_hour", Some(31.4)),
        window("seven_day_opus", Some(10.0)),
        window("seven_day_fable", Some(81.0)),
    ];

    let left = tier_lines(&windows, QuotaDisplay::default());
    assert_eq!(
        left.iter().map(|line| line.window.name.as_str()).collect::<Vec<_>>(),
        ["five_hour", "seven_day", "seven_day_opus", "seven_day_fable", "monthly"]
    );
    let used = tier_lines(&windows, QuotaDisplay::Used);
    assert_eq!(left[0].shown_percent, Some(69.0));
    assert_eq!(used[0].shown_percent, Some(31.0));
    assert_eq!(left[0].bar_percent(), Some(69.0));
    assert_eq!(used[0].bar_percent(), Some(31.0));
    assert_eq!(used[1].window.resets_at.as_deref(), Some("2026-11-11T00:00:00Z"));
    assert_eq!(used[1].remaining_percent, Some(20.0));
    assert_eq!(used[3].remaining_percent, Some(19.0));
    assert_eq!(used[1].tone, Some(QuotaTone::Normal));
    assert_eq!(used[3].tone, Some(QuotaTone::Warning));
    assert_eq!(
        left.iter().map(|line| line.tone).collect::<Vec<_>>(),
        used.iter().map(|line| line.tone).collect::<Vec<_>>()
    );

    for display in [QuotaDisplay::Remaining, QuotaDisplay::Used] {
        let rows = tier_card_rows(&windows, display);
        let names: Vec<Vec<_>> = rows
            .iter()
            .map(|row| row.iter().map(|line| line.window.name.as_str()).collect())
            .collect();
        assert_eq!(names, [vec!["five_hour"], vec!["seven_day_fable", "monthly"]]);
    }
}

#[test]
fn usage_presentation_tiers_rejects_invalid_input() {
    let windows = [
        window("five_hour", Some(120.0)),
        window("seven_day", Some(100.0)),
        window("monthly", None),
        window("premium", Some(-2.0)),
    ];
    for (display, exhausted, out_of_range, bar) in
        [(QuotaDisplay::Remaining, 0.0, 102.0, 100.0), (QuotaDisplay::Used, 100.0, -2.0, 0.0)]
    {
        let lines = tier_lines(&windows, display);
        for line in &lines[..2] {
            assert_eq!(line.tone, Some(QuotaTone::Danger));
            assert_eq!(line.remaining_percent, Some(0.0));
            assert_eq!(line.shown_percent, Some(exhausted));
        }
        assert_eq!(lines[2].tone, None);
        assert_eq!(lines[2].shown_percent, None);
        assert_eq!(lines[2].bar_percent(), None);
        assert_eq!(lines[3].shown_percent, Some(out_of_range));
        assert_eq!(lines[3].bar_percent(), Some(bar));
    }
}

#[test]
fn usage_presentation_tiers_keeps_short_lists_and_stable_ties() {
    let windows = [
        window("custom", Some(90.0)),
        window("monthly", Some(90.0)),
        window("agy_third_party_weekly", Some(90.0)),
        window("agy_gemini_five_hour", Some(5.0)),
    ];
    let rows = tier_card_rows(&windows, QuotaDisplay::Remaining);
    assert_eq!(rows[0][0].window.name, "agy_gemini_five_hour");
    assert_eq!(rows[1][0].window.name, "agy_third_party_weekly");
    assert_eq!(rows[1][1].window.name, "monthly");

    let rows = tier_card_rows(&windows[..2], QuotaDisplay::Remaining);
    assert_eq!(rows.iter().map(Vec::len).collect::<Vec<_>>(), [1, 1]);
    assert_eq!(rows[0][0].window.name, "monthly");
    assert_eq!(rows[1][0].window.name, "custom");
    assert!(tier_card_rows(&[], QuotaDisplay::Remaining).is_empty());
}
