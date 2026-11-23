#[test]
fn current_version_matches_product_version() {
    assert_eq!(super::current_version(), product_version::VERSION);
    assert!(!super::current_version().is_empty());
}
