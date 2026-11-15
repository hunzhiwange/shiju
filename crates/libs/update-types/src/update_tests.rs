#[test]
fn current_version_matches_cargo_package_version() {
    assert_eq!(super::current_version(), env!("CARGO_PKG_VERSION"));
    assert!(!super::current_version().is_empty());
}
