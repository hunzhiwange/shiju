use super::credits::{CreditsUsdEstimate, credits_line, reset_credits_line};
use super::tiers::QuotaTone;
use chrono::DateTime;
use vw_api_types::apps_usage::UsageNumber;

#[test]
fn usage_presentation_credits_returns_expected_result() {
    let now = DateTime::parse_from_rfc3339("2026-11-10T08:00:00+08:00").unwrap();
    let expiries = [
        None,
        Some("2026-11-20T00:00:00Z".into()),
        Some("2026-11-09T23:59:59Z".into()),
        Some("2026-11-12T00:01:00+08:00".into()),
        Some("2026-11-11T17:00:00Z".into()),
        Some("2026-11-11T15:59:59Z".into()),
        Some("unknown".into()),
    ];
    let resets = reset_credits_line(Some(&expiries), &now).unwrap();
    assert_eq!(resets.count, 6);
    assert_eq!(resets.tone, QuotaTone::Warning);
    assert_eq!(
        resets
            .groups
            .iter()
            .map(|group| (group.expiry_day.map(|day| day.to_string()), group.count, group.tone,))
            .collect::<Vec<_>>(),
        [
            (Some("2026-11-11".into()), 1, QuotaTone::Warning),
            (Some("2026-11-12".into()), 2, QuotaTone::Warning),
            (Some("2026-11-20".into()), 1, QuotaTone::Normal),
            (None, 2, QuotaTone::Normal),
        ]
    );
    assert_eq!(
        resets.groups[1].earliest_expiry,
        Some(DateTime::parse_from_rfc3339("2026-11-11T16:01:00Z").unwrap().to_utc())
    );
    assert_eq!(resets.groups[3].earliest_expiry, None);

    let balance = UsageNumber::try_from(1234.5).unwrap();
    let credits = credits_line(Some(balance)).unwrap();
    assert_eq!(credits.balance, balance);
    assert_eq!(credits.usd_estimate, CreditsUsdEstimate::RoundedDollars(49.0));
    let small_balance = UsageNumber::try_from(10.0).unwrap();
    assert_eq!(
        credits_line(Some(small_balance)).unwrap().usd_estimate,
        CreditsUsdEstimate::UnderOneDollar
    );
}

#[test]
fn usage_presentation_credits_rejects_invalid_input() {
    let now = DateTime::parse_from_rfc3339("2026-11-10T00:00:00Z").unwrap();
    let expired = [Some("2026-11-09T23:59:59Z".into()), Some("2026-11-10T08:00:00+08:00".into())];
    assert!(reset_credits_line(Some(&expired), &now).is_none());
    assert!(reset_credits_line(Some(&[]), &now).is_none());
    assert!(reset_credits_line(None, &now).is_none());
    assert!(credits_line(None).is_none());
    for invalid in [0.0, -1.0] {
        assert!(credits_line(Some(UsageNumber::try_from(invalid).unwrap())).is_none());
    }
}

#[test]
fn usage_presentation_credits_warns_only_inside_three_days() {
    let now = DateTime::parse_from_rfc3339("2026-11-10T00:00:00Z").unwrap();
    for (expiry, tone) in [
        ("2026-11-12T23:59:59.999Z", QuotaTone::Warning),
        ("2026-11-13T00:00:00Z", QuotaTone::Normal),
    ] {
        let resets = reset_credits_line(Some(&[Some(expiry.into())]), &now).unwrap();
        assert_eq!(resets.tone, tone);
        assert_eq!(resets.groups[0].tone, tone);
    }
}
