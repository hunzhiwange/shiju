use crate::parse_temperature;

#[test]
fn parse_temperature_accepts_inclusive_bounds_and_rejects_invalid_values() {
    assert_eq!(parse_temperature("0").unwrap(), 0.0);
    assert_eq!(parse_temperature("2.0").unwrap(), 2.0);
    assert_eq!(parse_temperature("2.01").unwrap_err(), "temperature must be between 0.0 and 2.0");
    assert!(parse_temperature("not-a-number").is_err());
}

#[test]
fn parse_temperature_rejects_negative_and_out_of_range_values() {
    let err_neg = parse_temperature("-1").unwrap_err();
    assert!(err_neg.contains("between 0.0 and 2.0"), "unexpected error: {err_neg}");

    let err_slightly_above = parse_temperature("2.0001").unwrap_err();
    assert!(
        err_slightly_above.contains("between 0.0 and 2.0"),
        "unexpected error: {err_slightly_above}"
    );

    let err_slightly_below = parse_temperature("-0.0001").unwrap_err();
    assert!(
        err_slightly_below.contains("between 0.0 and 2.0"),
        "unexpected error: {err_slightly_below}"
    );
}

#[test]
fn parse_temperature_succeeds_within_range() {
    assert_eq!(parse_temperature("0.5").unwrap(), 0.5);
    assert_eq!(parse_temperature("1.0").unwrap(), 1.0);
    assert_eq!(parse_temperature("1.5").unwrap(), 1.5);
}

#[test]
fn parse_temperature_propagates_parse_error_formatted() {
    let err = parse_temperature("").unwrap_err();
    assert!(err.contains("cannot parse"), "unexpected error: {err}");

    let err = parse_temperature("abc").unwrap_err();
    assert!(err.contains("invalid float"), "unexpected error: {err}");
}

#[test]
fn parse_temperature_zero_and_two_exact_boundaries() {
    assert_eq!(parse_temperature("0.0").unwrap(), 0.0);
    assert_eq!(parse_temperature("2.0").unwrap(), 2.0);
}
