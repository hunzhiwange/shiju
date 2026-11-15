use super::hermes::*;
#[test]
fn cc04_hermes_catalog_and_symlink_reads_do_not_expose_credentials() {
    use vw_api_app_provider_edit::catalog::*;
    use vw_api_app_identity::{AppId};
    let home = tempfile::tempdir().unwrap();
    assert!(read(home.path()).unwrap().is_none());
    assert!(!home.path().join(".hermes").exists());
    std::fs::create_dir(home.path().join(".hermes")).unwrap();
    let file = home.path().join(".hermes/config.yaml");
    std::fs::write(&file, b"custom_providers:\n  - name: one\n    api_key: fake-secret\n").unwrap();
    let result = super::catalog::read(home.path(), AppId::Hermes, vec![]);
    assert_eq!(result.rows[0].provider.id, "one");
    assert_eq!(result.rows[0].native_member, ProviderMembership::Present);
    assert_eq!(result.direct, ProviderSelection::NotApplicable);
    assert!(!serde_json::to_string(&result).unwrap().contains("fake-secret"));
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::Hermes, "one").is_err());
    assert!(super::delete::ensure_unreferenced(home.path(), AppId::Hermes, "unused").is_ok());
    std::fs::remove_file(&file).unwrap();
    let outside = home.path().join("outside");
    std::fs::write(&outside, b"agent: {}\n").unwrap();
    std::os::unix::fs::symlink(&outside, &file).unwrap();
    assert!(read(home.path()).is_err());
    assert!(HermesFiles::open(home.path()).unwrap().snapshot().is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"agent: {}\n");
}
