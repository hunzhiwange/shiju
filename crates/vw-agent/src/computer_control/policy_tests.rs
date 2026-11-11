use super::policy::{ComputerControlPolicy, ControlFeature, GrantScope};
use vw_api_types::computer_control::{ComputerControlTarget, ControlDocumentId, PermissionVersion};
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

fn app(id: &str) -> ComputerControlTarget {
    ComputerControlTarget::NativeApp { bundle_id: id.into(), process_id: 10 }
}

#[test]
fn cc_control_03_defaults_deny_all_five_features() {
    let p = ComputerControlPolicy::new(ComputerControlConfig::default());
    for target in [
        app("com.apple.TextEdit"),
        ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() },
        ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId("book-1".into()) },
        ComputerControlTarget::WordDocument { document_id: ControlDocumentId("doc-1".into()) },
    ] {
        assert!(p.authorize("uid:501", &target, false, p.version()).is_err());
        assert!(p.authorize("uid:501", &target, true, p.version()).is_err());
    }
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
fn cc_control_03_office_chrome_and_lock_require_independent_grants() {
    let mut p = ComputerControlPolicy::new(ComputerControlConfig::default());
    p.set_enabled("uid:501", ControlFeature::NativeApps, true).unwrap();
    let book =
        ComputerControlTarget::ExcelWorkbook { document_id: ControlDocumentId("book-1".into()) };
    let doc =
        ComputerControlTarget::WordDocument { document_id: ControlDocumentId("book-1".into()) };
    let tab = ComputerControlTarget::ChromeTab { tab_id: 1, origin: "https://example.com".into() };
    for target in [
        &book,
        &doc,
        &tab,
        &app("com.microsoft.Excel"),
        &app("com.microsoft.Word"),
        &app("com.google.Chrome"),
    ] {
        assert!(p.authorize("uid:501", target, false, p.version()).is_err());
    }
    p.set_enabled("uid:501", ControlFeature::Excel, true).unwrap();
    assert!(p.authorize("uid:501", &book, false, p.version()).is_err());
    p.set_grant("uid:501", GrantScope::ExcelDocument("book-1".into()), Some(GrantDecision::Allow))
        .unwrap();
    assert!(p.authorize("uid:501", &book, false, p.version()).is_ok());
    assert!(p.authorize("uid:501", &doc, false, p.version()).is_err());
    assert!(p.authorize("uid:501", &book, true, p.version()).is_err());
    p.set_enabled("uid:501", ControlFeature::LockedUse, true).unwrap();
    assert!(p.authorize("uid:501", &book, true, p.version()).is_ok());
    p.set_enabled("uid:501", ControlFeature::Word, true).unwrap();
    p.set_grant("uid:501", GrantScope::WordDocument("book-1".into()), Some(GrantDecision::Allow))
        .unwrap();
    assert!(p.authorize("uid:501", &doc, false, p.version()).is_ok());
    p.set_enabled("uid:501", ControlFeature::Chrome, true).unwrap();
    p.set_grant(
        "uid:501",
        GrantScope::ChromeOrigin("https://example.com".into()),
        Some(GrantDecision::Allow),
    )
    .unwrap();
    assert!(p.authorize("uid:501", &tab, false, p.version()).is_ok());
    assert!(
        p.authorize(
            "uid:501",
            &ComputerControlTarget::ChromeTab {
                tab_id: 2,
                origin: "https://other.example.com".into()
            },
            false,
            p.version()
        )
        .is_err()
    );
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
    for scope in [
        GrantScope::Application("window title".into()),
        GrantScope::ChromeOrigin("https://example.com/path".into()),
        GrantScope::ChromeOrigin("https://user:pass@example.com".into()),
        GrantScope::ExcelDocument("".into()),
    ] {
        assert!(p.set_grant("uid:501", scope, Some(GrantDecision::Allow)).is_err());
    }
    assert_eq!(p.version(), PermissionVersion(0));
    assert!(p.config().users.is_empty());
    let mut p = ComputerControlPolicy::new(ComputerControlConfig {
        permission_version: u64::MAX,
        ..Default::default()
    });
    assert!(p.set_enabled("uid:501", ControlFeature::NativeApps, true).is_err());
    assert!(p.config().users.is_empty());
}
