use super::*;
#[test]
fn hermes_dashboard_requires_host_authority_before_probe_or_start() {
    let scope = DashboardScope { project_id: "project".into(), target_host_id: "remote".into() };
    let result = authorize(&State::default(), TenantScope::new("p", "c").unwrap(), &HeaderMap::new(), &scope);
    assert!(matches!(result, Err(StatusCode::UNAUTHORIZED)));
}
