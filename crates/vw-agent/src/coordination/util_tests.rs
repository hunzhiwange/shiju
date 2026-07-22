use super::errors::CoordinationError;
use super::util::{
    normalized_non_empty, parse_delegate_context_correlation_from_key, require_non_empty,
};

#[test]
fn normalized_non_empty_trims_and_filters_blank_values() {
    assert_eq!(normalized_non_empty(Some(" corr-a ")), Some("corr-a"));
    assert_eq!(normalized_non_empty(Some("")), None);
    assert_eq!(normalized_non_empty(Some("   ")), None);
    assert_eq!(normalized_non_empty(None), None);
}

#[test]
fn delegate_context_correlation_parser_requires_full_shape() {
    assert_eq!(
        parse_delegate_context_correlation_from_key("delegate/corr-a/context"),
        Some("corr-a")
    );
    assert_eq!(
        parse_delegate_context_correlation_from_key("delegate/ corr-a /context"),
        Some("corr-a")
    );
    assert_eq!(parse_delegate_context_correlation_from_key("delegate//context"), None);
    assert_eq!(parse_delegate_context_correlation_from_key("other/corr-a/context"), None);
    assert_eq!(parse_delegate_context_correlation_from_key("delegate/corr-a"), None);
    assert_eq!(parse_delegate_context_correlation_from_key("delegate/corr-a/"), None);
    assert_eq!(parse_delegate_context_correlation_from_key("delegate/corr-a/   "), None);
    assert!(require_non_empty("agent", "from").is_ok());
    assert_eq!(
        require_non_empty(" ", "from"),
        Err(CoordinationError::EmptyField { field: "from" })
    );
}
