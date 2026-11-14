use super::*;
#[test]
fn hermes_dashboard_sdk_preserves_scope_rejects_operation_path_injection() {
    assert_eq!(BASE, "/v1/agent-apps/hermes/dashboard");
    for id in ["", "../start", "a?targetHostId=other", "a/b"] {
        assert_eq!(operation_path(id), Err(HermesDashboardError::InvalidInput));
    }
    assert_eq!(operation_path("ab-12").unwrap(), "operations/ab-12");
}
