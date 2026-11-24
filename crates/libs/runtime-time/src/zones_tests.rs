use super::*;
use chrono::TimeZone;

#[test]
fn new_york_local_times_preserve_daylight_saving_gaps_and_folds() {
    assert_eq!(NEW_YORK.with_ymd_and_hms(2026, 3, 8, 2, 30, 0), LocalResult::None,);
    let LocalResult::Ambiguous(first, last) = NEW_YORK.with_ymd_and_hms(2026, 11, 1, 1, 30, 0)
    else {
        panic!("the clock repeats 01:30 when daylight saving time ends");
    };
    assert_eq!(first.offset().fix().local_minus_utc(), -4 * 3600);
    assert_eq!(last.offset().fix().local_minus_utc(), -5 * 3600);
    assert_eq!(last.timestamp() - first.timestamp(), 3600);
}

#[test]
fn stored_iana_names_resolve_to_bundled_zones() {
    for zone in [SHANGHAI, UTC, NEW_YORK, BERLIN] {
        assert_eq!(zone.name().parse(), Ok(zone));
    }
    assert!("Mars/Olympus_Mons".parse::<NamedTimeZone>().is_err());
}
