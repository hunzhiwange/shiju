use super::*;

#[test]
fn sniff_types_handles_short_and_long_records() {
    assert_eq!(sniff_types(br#"{"type":"response_item","payload":{"type":"message"}}"#), Some(("response_item", Some("message"))));
    assert_eq!(sniff_types(&vec![b' '; SNIFF_BYTES + 1]), None);
    assert!(INJECTED_USER_PREFIXES.iter().any(|prefix| "# AGENTS.md instructions for project".starts_with(prefix)));
}
