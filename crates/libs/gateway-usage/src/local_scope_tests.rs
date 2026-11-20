use crate::read::ReadQuery;
use agent_usage::context;

#[test]
fn usage_read_accepts_filters_without_a_project_or_host() {
    assert!(serde_json::from_str::<ReadQuery>(r#"{"time_zone":"UTC","page":2}"#).is_ok());
    for selector in [
        r#"{"project_id":"desktop-local"}"#,
        r#"{"target_host":"desktop-local"}"#,
    ] {
        assert!(serde_json::from_str::<ReadQuery>(selector).is_err());
    }
}

#[test]
fn usage_keeps_the_existing_local_ledger_key_for_each_tenant() {
    let tenant = vw_api_transport::TenantScope::new("platform", "company").unwrap();
    let scope = context::local_scope(tenant.clone());
    assert_eq!(scope.tenant_scope(), &tenant);
    assert_eq!(scope.project_id().as_ref(), vw_api_transport::LOCAL_PROJECT_ID);
    assert_eq!(context::local_host().unwrap(), vw_api_transport::LOCAL_HOST_ID);
}
