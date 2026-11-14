use crate::security::{self, secrets};

#[test]
fn secret_store_production_boundary() {
    let dir = tempfile::tempdir().expect("temporary secret directory");
    let key_path = dir.path().join(".secret_key");
    let fixture_key = "42".repeat(32);
    std::fs::write(&key_path, &fixture_key).expect("write fixture key");

    let store = secrets::SecretStore::new(dir.path(), true);
    let package_store: &secret_store::secrets::SecretStore = &store;
    let root_store: &security::SecretStore = package_store;
    let encrypted = root_store.encrypt("boundary-fixture-secret").expect("encrypt fixture");
    assert!(encrypted.starts_with("enc2:"));
    assert_ne!(encrypted, "boundary-fixture-secret");
    assert_eq!(package_store.decrypt(&encrypted).expect("decrypt fixture"), "boundary-fixture-secret");

    let reopened = secrets::SecretStore::new(dir.path(), true);
    assert_eq!(reopened.decrypt(&encrypted).expect("reopen fixture"), "boundary-fixture-secret");
    assert_eq!(std::fs::read_to_string(key_path).expect("read fixture key"), fixture_key);
}
