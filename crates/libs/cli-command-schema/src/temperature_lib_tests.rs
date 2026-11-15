use crate::parse_temperature;

#[test]
fn parse_temperature_accepts_inclusive_model_bounds() {
    assert_eq!(parse_temperature("0"), Ok(0.0));
    assert_eq!(parse_temperature("2.0"), Ok(2.0));
    assert_eq!(parse_temperature("0.75"), Ok(0.75));
}

#[test]
fn parse_temperature_rejects_values_outside_supported_range() {
    assert_eq!(
        parse_temperature("-0.01"),
        Err("temperature must be between 0.0 and 2.0".to_string())
    );
    assert_eq!(
        parse_temperature("2.01"),
        Err("temperature must be between 0.0 and 2.0".to_string())
    );
}

#[test]
fn parse_temperature_rejects_non_finite_float_spellings() {
    for value in ["NaN", "inf", "-inf"] {
        assert_eq!(
            parse_temperature(value),
            Err("temperature must be between 0.0 and 2.0".to_string()),
            "{value} must not be accepted as a temperature"
        );
    }
}

#[test]
fn parse_temperature_keeps_parser_whitespace_rules_explicit() {
    assert!(parse_temperature(" 1.0 ").is_err());
    assert_eq!(parse_temperature("1e0"), Ok(1.0));
}
