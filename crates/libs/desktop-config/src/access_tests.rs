use super::*;
#[test]
fn normalize_identity_format_always_returns_openclaw() {
    assert_eq!(normalize_identity_format(None), "openclaw");
    assert_eq!(normalize_identity_format(Some(" aieos ")), "openclaw");
}
#[test]
fn gateway_blocking_runtime_reuses_successful_runtime() {
    let first = gateway_blocking_runtime().expect("runtime should build") as *const _;
    let second = gateway_blocking_runtime().expect("runtime should be reused") as *const _;

    assert_eq!(first, second);
}
