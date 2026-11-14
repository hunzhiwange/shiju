use super::computer_control::{feature, scope, setting};
use crate::computer_control::policy::{ComputerControlPolicy, ControlFeature, GrantScope};
use vw_api_types::computer_control::*;
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

#[test]
fn cc_control_18_disabled_does_not_claim_ready() {
    let status = feature(false, ControlReadiness::Ready, SystemAuthorization::Granted, ControlBackend::Available, vec![]);
    assert_eq!(status.readiness, ControlReadiness::Disabled);
}
#[test]
fn cc_control_18_locked_setting_maps_to_independent_gate() {
    assert_eq!(setting(ControlSetting::LockedUse), ControlFeature::LockedUse);
}
#[test]
fn cc_control_18_document_scope_preserves_identity() {
    assert_eq!(scope(ControlGrantScope::WordDocument("doc".into())), GrantScope::WordDocument("doc".into()));
}
#[test]
fn cc_control_18_revocation_overrides_broad_native_allow() {
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    policy.set_enabled("uid:1", ControlFeature::NativeApps, true).unwrap();
    policy.set_grant("uid:1", GrantScope::Application("com.example.App".into()), Some(GrantDecision::Deny)).unwrap();
    assert_eq!(policy.authorize("uid:1", &ComputerControlTarget::NativeApp { bundle_id: "com.example.App".into(), process_id: 1 }, false, policy.version()), Err(ControlErrorCode::PermissionDenied));
}
#[test]
fn cc_control_18_saved_edit_invalidates_old_permission() {
    let mut policy = ComputerControlPolicy::new(ComputerControlConfig::default());
    let old = policy.version();
    policy.set_enabled("uid:1", ControlFeature::Chrome, true).unwrap();
    assert_eq!(policy.authorize("uid:1", &ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() }, false, old), Err(ControlErrorCode::StalePermission));
}
#[test]
fn cc_control_18_settings_reject_unknown_authority_fields() {
    assert!(serde_json::from_value::<ControlSettingsUpdate>(serde_json::json!({"permission_version":0,"feature":"chrome","enabled":true,"local_user":"uid:0"})).is_err());
}

macro_rules! unauthenticated_route {
    ($name:ident, $method:literal, $path:literal, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            use axum::{body::Body, http::{Request, StatusCode}, Extension};
            use tower::ServiceExt;
            let response = super::computer_control::router()
                .layer(Extension(vw_api_types::id::TenantScope::default()))
                .oneshot(Request::builder().method($method).uri($path)
                    .header("content-type", "application/json")
                    .body(Body::from($body)).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    };
}
unauthenticated_route!(cc_control_18_status_requires_auth, "GET", "/computer-control/status", "");
unauthenticated_route!(cc_control_18_approvals_require_auth, "GET", "/computer-control/approvals", "");
unauthenticated_route!(cc_control_18_stop_requires_auth, "POST", "/computer-control/stop", "");
unauthenticated_route!(cc_control_18_save_requires_auth, "PUT", "/computer-control/settings", r#"{"permission_version":0,"feature":"chrome","enabled":true}"#);
unauthenticated_route!(cc_control_18_revoke_requires_auth, "DELETE", "/computer-control/approvals", r#"{"permission_version":0,"scope":{"kind":"application","identity":"com.example.App"}}"#);
unauthenticated_route!(cc_control_18_pair_requires_auth, "POST", "/computer-control/pairing", r#"{"permission_version":0,"feature":"word","session_id":"session","workspace_root":null,"document_id":"doc"}"#);
