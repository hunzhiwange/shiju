use super::access::*;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use {
    vw_api_app_operations::AppLifecycleAction,
    vw_api_transport::id::TenantScope,
};

#[test]
fn local_apps_generated_grant_uses_existing_authorization_and_rejects_other_scopes() {
    let home = tempfile::tempdir().unwrap();
    let credential = local_app_auth::initialize(home.path(), "p", "c").unwrap();
    let policy = local_app_auth::policy_path(home.path(), "p", "c");
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", HeaderValue::from_str(&credential.token).unwrap());
    let tenant = TenantScope::new("p", "c").unwrap();
    let host = local_app_auth::HOST_ID;
    assert!(authorize(Some(&policy), &headers, &tenant, host).is_ok());
    assert!(authorize_preferences(Some(&policy), &headers, &tenant, host, None, true).is_ok());
    assert!(authorize_launch_session(Some(&policy), &headers, &tenant, host).is_ok());
    for action in
        [AppLifecycleAction::Install, AppLifecycleAction::Update, AppLifecycleAction::Uninstall]
    {
        assert!(authorize_action(Some(&policy), &headers, &tenant, host, Some(action)).is_ok());
    }
    assert_eq!(
        authorize(Some(&policy), &headers, &TenantScope::new("p", "other").unwrap(), host).err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        authorize(Some(&policy), &headers, &tenant, "other-host").err(),
        Some(StatusCode::FORBIDDEN)
    );
    assert_eq!(
        authorize(Some(&policy), &HeaderMap::new(), &tenant, host).err(),
        Some(StatusCode::UNAUTHORIZED)
    );
    assert_eq!(
        authorize(Some(&home.path().join("explicit-missing-policy")), &headers, &tenant, host)
            .err(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
}
