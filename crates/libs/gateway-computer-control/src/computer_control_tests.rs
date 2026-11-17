use super::{feature, scope, setting};
use computer_control::policy::{ComputerControlPolicy, ControlFeature, GrantScope};
use vw_api_computer_control::*;
use config_computer_control::{ComputerControlConfig, GrantDecision};

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

#[test]
fn cc_control_18_auth_disabled_allows_desktop_without_skey() {
    let guard = gateway_pairing::pairing::PairingGuard::new(false, &[]);
    assert_eq!(super::authorize(&guard, &axum::http::HeaderMap::new()), Ok(()));
}

#[test]
fn cc_control_18_auth_enabled_requires_valid_skey() {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    let guard = gateway_pairing::pairing::PairingGuard::new(true, &["test-control-key".into()]);
    let mut headers = HeaderMap::new();
    assert_eq!(super::authorize(&guard, &headers), Err(StatusCode::UNAUTHORIZED));
    headers.insert("authorization", HeaderValue::from_static("Bearer wrong-key"));
    assert_eq!(super::authorize(&guard, &headers), Err(StatusCode::UNAUTHORIZED));
    headers.insert("authorization", HeaderValue::from_static("Bearer test-control-key"));
    assert_eq!(super::authorize(&guard, &headers), Ok(()));
    headers.append("authorization", HeaderValue::from_static("Bearer test-control-key"));
    assert_eq!(super::authorize(&guard, &headers), Err(StatusCode::UNAUTHORIZED));
}
