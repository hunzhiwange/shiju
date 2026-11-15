use super::fields::parse_bounded_number;
#[test]
fn parse_bounded_number_uses_value_fallback_and_limits() {
    assert_eq!(parse_bounded_number("12.5", 1.0, 20.0), 12.5);
    assert_eq!(parse_bounded_number("invalid", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("-4", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("25", 1.0, 20.0), 20.0);
}

#[test]
fn parse_bounded_number_clamps_infinite_values_and_preserves_bounds() {
    assert_eq!(parse_bounded_number("inf", 1.0, 20.0), 20.0);
    assert_eq!(parse_bounded_number("-inf", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("1", 1.0, 20.0), 1.0);
    assert_eq!(parse_bounded_number("20", 1.0, 20.0), 20.0);
}
