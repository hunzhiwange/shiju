use super::policy::{ComputerControlPolicy, ControlFeature, GrantScope};
use vw_api_computer_control::{ComputerControlTarget, PermissionVersion};
use config_computer_control::{ComputerControlConfig, GrantDecision};

fn app(id: &str) -> ComputerControlTarget {
    ComputerControlTarget::NativeApp { bundle_id: id.into(), process_id: 10 }
}

#[test]
fn cc_control_03_defaults_deny_native_and_locked_use() {
    let p = ComputerControlPolicy::new(ComputerControlConfig::default());
    let target = app("com.apple.TextEdit");
    assert!(p.authorize("uid:501", &target, false, p.version()).is_err());
    assert!(p.authorize("uid:501", &target, true, p.version()).is_err());
}

#[test]
fn cc_control_03_explicit_app_denial_overrides_any_app() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled("uid:501", ControlFeature::NativeApps, true).unwrap();
    assert!(p.authorize("uid:501", &app("com.apple.TextEdit"), false, p.version()).is_ok());
    p.set_grant(
        "uid:501",
        GrantScope::Application("com.apple.TextEdit".into()),
        Some(GrantDecision::Deny),
    )
    .unwrap();
    assert!(p.authorize("uid:501", &app("com.apple.TextEdit"), false, p.version()).is_err());
    assert!(p.authorize("uid:501", &app("com.apple.Preview"), false, p.version()).is_ok());
}

#[test]
fn native_policy_preserves_existing_application_exclusions() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled("uid:501", ControlFeature::NativeApps, true).unwrap();
    for id in ["com.microsoft.Excel", "com.microsoft.Word", "com.google.Chrome", "com.google.Chrome.beta"] {
        assert!(p.authorize("uid:501", &app(id), false, p.version()).is_err());
        assert!(p.set_grant("uid:501", GrantScope::Application(id.into()), Some(GrantDecision::Allow)).is_err());
    }
}

#[test]
fn cc_control_03_revoke_removes_persistent_approval_and_invalidates_version() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    let scope = GrantScope::Application("com.apple.TextEdit".into());
    p.set_grant("uid:501", scope.clone(), Some(GrantDecision::Allow)).unwrap();
    let old = p.version();
    assert!(p.authorize("uid:501", &app("com.apple.TextEdit"), false, old).is_ok());
    p.set_grant("uid:501", scope, None).unwrap();
    assert!(p.version().0 > old.0);
    assert!(p.config().users["uid:501"].applications.is_empty());
    assert!(p.authorize("uid:501", &app("com.apple.TextEdit"), false, old).is_err());
    assert!(p.authorize("uid:501", &app("com.apple.TextEdit"), false, p.version()).is_err());
}

#[test]
fn cc_control_03_local_users_are_isolated_and_stale_versions_fail() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled("uid:501", ControlFeature::NativeApps, true).unwrap();
    assert!(p.authorize("uid:502", &app("com.apple.TextEdit"), false, p.version()).is_err());
    assert!(
        p.authorize("uid:501", &app("com.apple.TextEdit"), false, PermissionVersion(0)).is_err()
    );
    assert!(p.authorize("", &app("com.apple.TextEdit"), false, p.version()).is_err());
}

#[test]
fn cc_control_03_invalid_identity_and_version_overflow_fail_without_mutation() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    let scope = GrantScope::Application("window title".into());
    assert!(p.set_grant("uid:501", scope, Some(GrantDecision::Allow)).is_err());
    assert_eq!(p.version(), PermissionVersion(0));
    assert!(p.config().users.is_empty());
    let mut p = ComputerControlPolicy::new(ComputerControlConfig {
        permission_version: u64::MAX,
        ..Default::default()
    });
    assert!(p.set_enabled("uid:501", ControlFeature::NativeApps, true).is_err());
    assert!(p.config().users.is_empty());
}
