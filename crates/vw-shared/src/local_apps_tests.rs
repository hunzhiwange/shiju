use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn local_apps_initialization_restores_private_credentials_and_scopes_tenants() {
    let home = tempfile::tempdir().unwrap();
    assert!(restore(home.path(), "p", "c").unwrap().is_none());
    let first = initialize(home.path(), "p", "c").unwrap();
    let second = initialize(home.path(), "p", "c").unwrap();
    assert_eq!(first.token, second.token);
    let other = initialize(home.path(), "p", "other").unwrap();
    assert_ne!(first.token, other.token);
    let path = policy_path(home.path(), "p", "c");
    let policy: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(policy["principals"][0]["grants"][0]["host_id"], HOST_ID);
    assert_eq!(policy["principals"][0]["grants"][0]["company_id"], "c");
    assert_eq!(policy["principals"][0]["grants"][0]["launch"], true);
    assert!(!std::fs::read_to_string(&path).unwrap().contains(&first.token));
    for file in [&path, &path.with_file_name("credential")] {
        assert_eq!(std::fs::metadata(file).unwrap().permissions().mode() & 0o777, 0o600);
    }
    assert_eq!(
        std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(restore(home.path(), "p", "missing").unwrap().is_none());
}

#[test]
fn local_apps_restore_rejects_exposed_or_replaced_credentials() {
    let home = tempfile::tempdir().unwrap();
    initialize(home.path(), "p", "c").unwrap();
    let path = policy_path(home.path(), "p", "c").with_file_name("credential");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(restore(home.path(), "p", "c").is_err());
    std::fs::remove_file(&path).unwrap();
    let target = home.path().join("other");
    std::fs::write(&target, "a".repeat(64)).unwrap();
    symlink(&target, &path).unwrap();
    assert!(initialize(home.path(), "p", "c").is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "a".repeat(64));
}

#[test]
fn local_apps_initialization_rejects_shared_or_symlinked_storage() {
    let home = tempfile::tempdir().unwrap();
    let config = vw_config_types::paths::home_config_dir(home.path());
    std::fs::create_dir(&config).unwrap();
    let root = config.join("local-apps");
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(initialize(home.path(), "p", "c").is_err());
    std::fs::remove_dir(&root).unwrap();
    symlink(home.path(), &root).unwrap();
    assert!(initialize(home.path(), "p", "c").is_err());
}
