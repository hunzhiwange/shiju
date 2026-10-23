use super::*;

#[test]
fn common_frequencies_round_trip_without_changing_schedule() {
    for frequency in CronFrequency::ALL {
        assert_eq!(CronFrequency::from_expression(frequency.default_expression()), frequency);
    }
    for (expression, expected) in [
        ("*/17 * * * *", CronFrequency::Minutes),
        ("59 * * * *", CronFrequency::Hourly),
        ("30 23 * * *", CronFrequency::Daily),
        ("15 8 * * SUN", CronFrequency::Weekly),
        ("0 0 31 * *", CronFrequency::Monthly),
    ] {
        assert_eq!(CronFrequency::from_expression(expression), expected);
    }
}

#[test]
fn complex_or_invalid_expressions_remain_in_advanced_mode() {
    for expression in [
        "0 9 * * MON-FRI",
        "0 0 9 * * *",
        "0 9 * * 1",
        "0 9 1 JAN *",
        "*/0 * * * *",
        "*/60 * * * *",
        "60 * * * *",
        "0 24 * * *",
        "0 9 0 * *",
        "0 9 32 * *",
        "",
        "invalid",
    ] {
        assert_eq!(
            CronFrequency::from_expression(expression),
            CronFrequency::Advanced,
            "{expression}"
        );
    }
}
