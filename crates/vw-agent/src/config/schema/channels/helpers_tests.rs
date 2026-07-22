use super::helpers::{
    decrypt_optional_secret, decrypt_secret, encrypt_optional_secret, encrypt_secret,
    is_valid_env_var_name,
};
use crate::app::agent::security::SecretStore;

#[test]
fn env_var_names_follow_shell_safe_subset() {
    assert!(is_valid_env_var_name("API_KEY"));
    assert!(is_valid_env_var_name("api_key_2"));
    assert!(is_valid_env_var_name("_SECRET_2"));
    assert!(!is_valid_env_var_name("2FAST"));
    assert!(!is_valid_env_var_name("BAD-NAME"));
    assert!(!is_valid_env_var_name("BAD.NAME"));
    assert!(!is_valid_env_var_name("密钥"));
    assert!(!is_valid_env_var_name(""));
}

#[test]
fn disabled_secret_store_leaves_values_plaintext() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), false);
    let mut required = "secret".to_string();
    let mut optional = Some("optional".to_string());

    encrypt_secret(&store, &mut required, "required").unwrap();
    encrypt_optional_secret(&store, &mut optional, "optional").unwrap();
    decrypt_secret(&store, &mut required, "required").unwrap();
    decrypt_optional_secret(&store, &mut optional, "optional").unwrap();

    assert_eq!(required, "secret");
    assert_eq!(optional.as_deref(), Some("optional"));
}

#[test]
fn enabled_secret_store_encrypts_only_plain_values_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut required = "secret".to_string();
    let mut optional = Some("optional".to_string());

    encrypt_secret(&store, &mut required, "required").unwrap();
    encrypt_optional_secret(&store, &mut optional, "optional").unwrap();

    assert!(SecretStore::is_encrypted(&required));
    assert!(SecretStore::is_encrypted(optional.as_deref().unwrap()));
    let encrypted_required = required.clone();
    let encrypted_optional = optional.clone();
    encrypt_secret(&store, &mut required, "required").unwrap();
    encrypt_optional_secret(&store, &mut optional, "optional").unwrap();
    assert_eq!(required, encrypted_required);
    assert_eq!(optional, encrypted_optional);

    decrypt_secret(&store, &mut required, "required").unwrap();
    decrypt_optional_secret(&store, &mut optional, "optional").unwrap();

    assert_eq!(required, "secret");
    assert_eq!(optional.as_deref(), Some("optional"));
}

#[test]
fn optional_secret_helpers_leave_none_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut value = None;

    encrypt_optional_secret(&store, &mut value, "optional").unwrap();
    decrypt_optional_secret(&store, &mut value, "optional").unwrap();

    assert_eq!(value, None);
}

#[test]
fn empty_required_secret_remains_empty_when_encryption_is_enabled() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut value = String::new();

    encrypt_secret(&store, &mut value, "required").unwrap();
    decrypt_secret(&store, &mut value, "required").unwrap();

    assert_eq!(value, "");
}

#[test]
fn decrypt_secret_errors_include_field_name() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let mut required = "enc2:not-hex".to_string();
    let mut optional = Some("enc2:not-hex".to_string());

    let required_err = decrypt_secret(&store, &mut required, "required").unwrap_err().to_string();
    let optional_err =
        decrypt_optional_secret(&store, &mut optional, "optional").unwrap_err().to_string();

    assert!(required_err.contains("Failed to decrypt required"));
    assert!(optional_err.contains("Failed to decrypt optional"));
}

#[test]
fn encrypt_secret_errors_include_field_name() {
    let tmp = tempfile::tempdir().unwrap();
    let blocked_path = tmp.path().join("blocked");
    std::fs::write(&blocked_path, "not a directory").unwrap();
    let store = SecretStore::new(&blocked_path, true);
    let mut required = "secret".to_string();
    let mut optional = Some("optional".to_string());

    let required_err = encrypt_secret(&store, &mut required, "required").unwrap_err().to_string();
    let optional_err =
        encrypt_optional_secret(&store, &mut optional, "optional").unwrap_err().to_string();

    assert!(required_err.contains("Failed to encrypt required"));
    assert!(optional_err.contains("Failed to encrypt optional"));
}
