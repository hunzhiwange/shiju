use super::*;
use chacha20poly1305::aead::{Aead as _, KeyInit as _};

fn store_with_fixture_key(dir: &std::path::Path, key_byte: u8) -> SecretStore {
    let store = SecretStore::new(dir, true);
    std::fs::write(dir.join(".secret_key"), hex_encode(&vec![key_byte; KEY_LEN])).unwrap();
    store
}

#[test]
fn hex_decode_rejects_invalid_input() {
    assert_eq!(hex_decode("00ff").unwrap(), vec![0, 255]);
    assert!(hex_decode("not hex").is_err());
}

#[test]
fn encrypted_prefix_checks_are_distinct() {
    assert!(SecretStore::is_encrypted("encrypted:v1:data"));
    assert!(SecretStore::is_secure_encrypted("encrypted:v2:data"));
    assert!(!SecretStore::is_secure_encrypted("encrypted:v1:data"));
}

#[test]
fn disabled_store_and_empty_plaintext_bypass_encryption() {
    let dir = tempfile::tempdir().unwrap();
    let disabled = SecretStore::new(dir.path(), false);
    let enabled = SecretStore::new(dir.path(), true);

    assert_eq!(disabled.encrypt("secret").unwrap(), "secret");
    assert_eq!(enabled.encrypt("").unwrap(), "");
    assert_eq!(disabled.decrypt("plain").unwrap(), "plain");
    assert_eq!(disabled.decrypt_and_migrate("plain").unwrap(), ("plain".to_string(), None));
}

#[test]
fn chacha20_encryption_roundtrips_and_rejects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);

    let encrypted = store.encrypt("top-secret").unwrap();
    assert!(encrypted.starts_with("enc2:"));
    assert_ne!(encrypted, "top-secret");
    assert_eq!(store.decrypt(&encrypted).unwrap(), "top-secret");
    assert_eq!(store.decrypt_and_migrate(&encrypted).unwrap(), ("top-secret".to_string(), None));

    let mut tampered = encrypted.clone();
    tampered.pop();
    tampered.push(if encrypted.ends_with('0') { '1' } else { '0' });
    assert!(store.decrypt(&tampered).is_err());
}

#[test]
fn legacy_xor_secret_decrypts_and_migrates_to_enc2() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let _ = store.encrypt("warm-key").unwrap();
    let key = hex_decode(&std::fs::read_to_string(&store.key_path).unwrap()).unwrap();
    let legacy = format!("enc:{}", hex_encode(&xor_cipher(b"legacy-secret", &key)));

    assert!(SecretStore::needs_migration(&legacy));
    let (plaintext, migrated) = store.decrypt_and_migrate(&legacy).unwrap();

    assert_eq!(plaintext, "legacy-secret");
    let migrated = migrated.expect("legacy value should migrate");
    assert!(migrated.starts_with("enc2:"));
    assert_eq!(store.decrypt(&migrated).unwrap(), "legacy-secret");
}

#[test]
fn decrypt_routes_legacy_value_without_rewriting_key_file() {
    let dir = tempfile::tempdir().unwrap();
    let key = vec![0x3c; KEY_LEN];
    let store = store_with_fixture_key(dir.path(), 0x3c);
    let key_path = dir.path().join(".secret_key");
    let key_fixture = std::fs::read_to_string(&key_path).unwrap();
    let legacy = format!("enc:{}", hex_encode(&xor_cipher(b"legacy-direct", &key)));

    let plaintext = store.decrypt(&legacy).unwrap();

    assert_eq!(plaintext, "legacy-direct");
    assert_eq!(std::fs::read_to_string(key_path).unwrap(), key_fixture);
}

#[test]
fn malformed_encrypted_values_return_errors() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);

    assert!(store.decrypt("enc2:not-hex").unwrap_err().to_string().contains("corrupt hex"));
    assert!(store.decrypt("enc2:00").unwrap_err().to_string().contains("too short"));
    assert!(store.decrypt("enc:not-hex").unwrap_err().to_string().contains("legacy"));
}

#[test]
fn key_file_is_reused_and_corruption_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let first = store.encrypt("first").unwrap();
    let second = SecretStore::new(dir.path(), true);

    assert_eq!(second.decrypt(&first).unwrap(), "first");

    std::fs::write(&store.key_path, "bad-key").unwrap();
    assert!(second.encrypt("fails").is_err());
}

#[test]
fn hex_and_xor_helpers_cover_edge_cases() {
    assert_eq!(hex_encode(&[0x00, 0xab, 0xff]), "00abff");
    assert!(hex_decode("abc").unwrap_err().to_string().contains("odd length"));
    assert_eq!(xor_cipher(b"data", b""), b"data");

    let encrypted = xor_cipher(b"data", b"k");
    assert_eq!(xor_cipher(&encrypted, b"k"), b"data");
    assert_eq!(build_windows_icacls_grant_arg("  Ada "), Some("Ada:F".to_string()));
    assert_eq!(build_windows_icacls_grant_arg("  "), None);
}

#[test]
fn prefix_recognizers_cover_all_supported_formats() {
    for value in ["enc2:data", "enc:data", "encrypted:v1:data", "encrypted:v2:data"] {
        assert!(SecretStore::is_encrypted(value), "{value}");
    }
    for value in ["plain", "ENC2:data", "enc2data", "encrypted:v3:data"] {
        assert!(!SecretStore::is_encrypted(value), "{value}");
    }

    assert!(SecretStore::is_secure_encrypted("enc2:data"));
    assert!(SecretStore::is_secure_encrypted("encrypted:v2:data"));
    assert!(!SecretStore::is_secure_encrypted("enc:data"));
    assert!(!SecretStore::is_secure_encrypted("encrypted:v1:data"));

    assert!(SecretStore::needs_migration("enc:data"));
    assert!(!SecretStore::needs_migration("enc2:data"));
    assert!(!SecretStore::needs_migration("encrypted:v1:data"));
}

#[test]
fn encrypting_same_plaintext_uses_fresh_nonce() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_with_fixture_key(dir.path(), 0x11);

    let first = store.encrypt("repeatable-secret").unwrap();
    let second = store.encrypt("repeatable-secret").unwrap();
    let nonce_hex_len = NONCE_LEN * 2;

    assert_ne!(&first[5..5 + nonce_hex_len], &second[5..5 + nonce_hex_len]);
    assert_eq!(store.decrypt(&first).unwrap(), "repeatable-secret");
    assert_eq!(store.decrypt(&second).unwrap(), "repeatable-secret");
}

#[test]
fn unicode_secret_roundtrips_without_loss() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_with_fixture_key(dir.path(), 0x22);
    let plaintext = "密钥-🦀-é-тест";

    let encrypted = store.encrypt(plaintext).unwrap();

    assert_eq!(store.decrypt(&encrypted).unwrap(), plaintext);
}

#[test]
fn ciphertext_from_different_key_is_rejected() {
    let first_dir = tempfile::tempdir().unwrap();
    let second_dir = tempfile::tempdir().unwrap();
    let first = store_with_fixture_key(first_dir.path(), 0x33);
    let second = store_with_fixture_key(second_dir.path(), 0x44);
    let encrypted = first.encrypt("key-bound-secret").unwrap();

    let error = second.decrypt(&encrypted).unwrap_err();

    assert!(error.to_string().contains("wrong key or tampered data"));
}

#[cfg(unix)]
#[test]
fn key_creation_writes_decodable_key_with_restricted_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let key_path = dir.path().join(".secret_key");

    store.encrypt("create-key").unwrap();
    let key_hex = std::fs::read_to_string(&key_path).unwrap();

    assert_eq!(key_hex.len(), KEY_LEN * 2);
    assert_eq!(hex_decode(&key_hex).unwrap().len(), KEY_LEN);
    assert_eq!(key_hex, key_hex.to_ascii_lowercase());

    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(&key_path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn existing_key_with_whitespace_is_trimmed_and_reused() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let expected_key = vec![0x5a; KEY_LEN];
    let key_path = dir.path().join(".secret_key");
    let fixture = format!("  {}\n", hex_encode(&expected_key));
    std::fs::write(&key_path, &fixture).unwrap();

    let encrypted = store.encrypt("existing-key").unwrap();
    assert_eq!(store.decrypt(&encrypted).unwrap(), "existing-key");
    assert_eq!(std::fs::read_to_string(key_path).unwrap(), fixture);
}

#[test]
fn key_path_directory_reports_read_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    std::fs::create_dir(dir.path().join(".secret_key")).unwrap();

    let error = store.encrypt("secret").unwrap_err();

    assert!(error.to_string().contains("Failed to read secret key file"));
}

#[test]
fn non_directory_parent_prevents_key_creation() {
    let dir = tempfile::tempdir().unwrap();
    let blocking_parent = dir.path().join("not-a-directory");
    std::fs::write(&blocking_parent, "fixture").unwrap();
    let store = SecretStore::new(&blocking_parent, true);

    assert!(store.encrypt("secret").is_err());
    assert_eq!(std::fs::read_to_string(blocking_parent).unwrap(), "fixture");
}

#[test]
fn chacha20_decryption_rejects_authenticated_non_utf8_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let key = vec![0x66; KEY_LEN];
    let store = store_with_fixture_key(dir.path(), 0x66);
    let cipher = chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key).unwrap();
    let nonce_bytes = [0x07; NONCE_LEN];
    let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, &[0xff, 0xfe][..]).unwrap();
    let mut blob = nonce_bytes.to_vec();
    blob.extend_from_slice(&ciphertext);

    let error = store.decrypt(&format!("enc2:{}", hex_encode(&blob))).unwrap_err();

    assert!(error.to_string().contains("not valid UTF-8"));
}

#[test]
fn legacy_decryption_rejects_non_utf8_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let key = vec![0x77; KEY_LEN];
    let store = store_with_fixture_key(dir.path(), 0x77);
    let ciphertext = xor_cipher(&[0xff, 0xfe], &key);

    let error = store.decrypt(&format!("enc:{}", hex_encode(&ciphertext))).unwrap_err();

    assert!(error.to_string().contains("not valid UTF-8"));
}

#[test]
fn helpers_cover_empty_uppercase_and_repeating_key_inputs() {
    assert_eq!(hex_decode("").unwrap(), Vec::<u8>::new());
    assert_eq!(hex_decode("A0fF").unwrap(), vec![0xa0, 0xff]);
    assert_eq!(xor_cipher(b"abcdef", b"xy"), vec![0x19, 0x1b, 0x1b, 0x1d, 0x1d, 0x1f]);
    assert_eq!(generate_random_key().len(), KEY_LEN);
}
