use super::openclaw::*;
#[test]
fn cc04_openclaw_catalog_reads_members_without_unique_current_or_secrets() {
    use vw_api_app_provider_edit::catalog::*;
    use vw_api_app_identity::{AppId};
    let home = tempfile::tempdir().unwrap();
    let missing = super::catalog::read(home.path(), AppId::OpenClaw, vec![]);
    assert_eq!(missing.native_read, None);
    assert!(!home.path().join(".openclaw").exists());
    std::fs::create_dir(home.path().join(".openclaw")).unwrap();
    let file = home.path().join(".openclaw/openclaw.json");
    std::fs::write(&file, br#"{"models":{"providers":{"one":{"apiKey":"fake-secret"}}}}"#).unwrap();
    let result = super::catalog::read(home.path(), AppId::OpenClaw, vec![]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].provider.id, "one");
    assert_eq!(result.rows[0].native_member, ProviderMembership::Present);
    assert_eq!(result.direct, ProviderSelection::NotApplicable);
    assert_eq!(result.in_use, ProviderSelection::NotApplicable);
    assert!(!serde_json::to_string(&result).unwrap().contains("fake-secret"));
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "one").is_err());
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "unused").is_ok());
    let before = std::fs::read(&file).unwrap();
    assert_eq!(before, read(home.path()).unwrap().unwrap());
    std::fs::write(&file, b"{bad}").unwrap();
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::OpenClaw, "unused").is_err());
}
