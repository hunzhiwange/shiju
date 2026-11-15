use tempfile::TempDir;
use super::{
    hex_decode, hex_encode, xor_cipher, generate_random_key,
};
use super::{
    SecretStore, KEY_LEN, NONCE_LEN,
};

// 测试加密和解密的往返操作是否正确
#[test]
fn encrypt_decrypt_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let secret = "sk-my-secret-api-key-12345";

    let encrypted = store.encrypt(secret).unwrap();
    assert!(encrypted.starts_with("enc2:"), "Should have enc2: prefix");
    assert_ne!(encrypted, secret, "Should not be plaintext");

    let decrypted = store.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, secret, "Roundtrip must preserve original");
}

// 测试加密空字符串时返回空字符串
#[test]
fn encrypt_empty_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let result = store.encrypt("").unwrap();
    assert_eq!(result, "");
}

// 测试解密明文时直接透传返回原文
#[test]
fn decrypt_plaintext_passthrough() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let result = store.decrypt("sk-plaintext-key").unwrap();
    assert_eq!(result, "sk-plaintext-key");
}

// 测试禁用加密功能时返回明文
#[test]
fn disabled_store_returns_plaintext() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), false);
    let result = store.encrypt("sk-secret").unwrap();
    assert_eq!(result, "sk-secret", "Disabled store should not encrypt");
}

// 测试检测加密值的前缀识别
#[test]
fn is_encrypted_detects_prefix() {
    assert!(SecretStore::is_encrypted("enc2:aabbcc"));
    assert!(SecretStore::is_encrypted("enc:aabbcc"));
    assert!(!SecretStore::is_encrypted("sk-plaintext"));
    assert!(!SecretStore::is_encrypted(""));
}

// 测试首次加密时自动创建密钥文件
#[tokio::test]
async fn key_file_created_on_first_encrypt() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    assert!(!store.key_path.exists());

    store.encrypt("test").unwrap();
    assert!(store.key_path.exists(), "Key file should be created");

    let key_hex = tokio::fs::read_to_string(&store.key_path).await.unwrap();
    assert_eq!(key_hex.len(), KEY_LEN * 2, "Key should be {KEY_LEN} bytes hex-encoded");
}

// 测试相同明文每次加密产生不同的密文
#[test]
fn encrypting_same_value_produces_different_ciphertext() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let e1 = store.encrypt("secret").unwrap();
    let e2 = store.encrypt("secret").unwrap();
    assert_ne!(e1, e2, "AEAD with random nonce should produce different ciphertext each time");

    assert_eq!(store.decrypt(&e1).unwrap(), "secret");
    assert_eq!(store.decrypt(&e2).unwrap(), "secret");
}

// 测试同一目录下的不同存储实例可以互相解密
#[test]
fn different_stores_same_dir_interop() {
    let tmp = TempDir::new().unwrap();
    let store1 = SecretStore::new(tmp.path(), true);
    let store2 = SecretStore::new(tmp.path(), true);

    let encrypted = store1.encrypt("cross-store-secret").unwrap();
    let decrypted = store2.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, "cross-store-secret");
}

// 测试 Unicode 字符的加密解密往返
#[test]
fn unicode_secret_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let secret = "sk-日本語テスト-émojis-🦀";

    let encrypted = store.encrypt(secret).unwrap();
    let decrypted = store.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, secret);
}

// 测试长字符串的加密解密往返
#[test]
fn long_secret_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let secret = "a".repeat(10_000);

    let encrypted = store.encrypt(&secret).unwrap();
    let decrypted = store.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, secret);
}

// 测试损坏的十六进制字符串返回错误
#[test]
fn corrupt_hex_returns_error() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let result = store.decrypt("enc2:not-valid-hex!!");
    assert!(result.is_err());
}

// 测试检测被篡改的密文
#[test]
fn tampered_ciphertext_detected() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let encrypted = store.encrypt("sensitive-data").unwrap();

    let hex_str = &encrypted[5..];
    let mut blob = hex_decode(hex_str).unwrap();
    if blob.len() > NONCE_LEN {
        blob[NONCE_LEN] ^= 0xff;
    }
    let tampered = format!("enc2:{}", hex_encode(&blob));

    let result = store.decrypt(&tampered);
    assert!(result.is_err(), "Tampered ciphertext must be rejected");
}

// 测试使用错误密钥解密时被检测并失败
#[test]
fn wrong_key_detected() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();
    let store1 = SecretStore::new(tmp1.path(), true);
    let store2 = SecretStore::new(tmp2.path(), true);

    let encrypted = store1.encrypt("secret-for-store1").unwrap();
    let result = store2.decrypt(&encrypted);
    assert!(result.is_err(), "Decrypting with a different key must fail");
}

// 测试截断的密文返回错误
#[test]
fn truncated_ciphertext_returns_error() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let result = store.decrypt("enc2:aabbccdd");
    assert!(result.is_err(), "Too-short ciphertext must be rejected");
}

// 测试旧版 XOR 加密的值仍可正常解密
#[test]
fn legacy_xor_decrypt_still_works() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "sk-legacy-api-key";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let decrypted = store.decrypt(&legacy_value).unwrap();
    assert_eq!(decrypted, plaintext, "Legacy XOR values must still decrypt");
}

// 测试检测需要迁移的旧版加密前缀
#[test]
fn needs_migration_detects_legacy_prefix() {
    assert!(SecretStore::needs_migration("enc:aabbcc"));
    assert!(!SecretStore::needs_migration("enc2:aabbcc"));
    assert!(!SecretStore::needs_migration("sk-plaintext"));
    assert!(!SecretStore::needs_migration(""));
}

// 测试仅识别 enc2 前缀为安全加密
#[test]
fn is_secure_encrypted_detects_enc2_only() {
    assert!(SecretStore::is_secure_encrypted("enc2:aabbcc"));
    assert!(!SecretStore::is_secure_encrypted("enc:aabbcc"));
    assert!(!SecretStore::is_secure_encrypted("sk-plaintext"));
    assert!(!SecretStore::is_secure_encrypted(""));
}

// 测试 enc2 格式不触发迁移
#[test]
fn decrypt_and_migrate_returns_none_for_enc2() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let encrypted = store.encrypt("my-secret").unwrap();
    assert!(encrypted.starts_with("enc2:"));

    let (plaintext, migrated) = store.decrypt_and_migrate(&encrypted).unwrap();
    assert_eq!(plaintext, "my-secret");
    assert!(migrated.is_none(), "enc2: values should not trigger migration");
}

// 测试明文不触发迁移
#[test]
fn decrypt_and_migrate_returns_none_for_plaintext() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let (plaintext, migrated) = store.decrypt_and_migrate("sk-plaintext-key").unwrap();
    assert_eq!(plaintext, "sk-plaintext-key");
    assert!(migrated.is_none(), "Plaintext values should not trigger migration");
}

// 测试将旧版 XOR 加密升级为 enc2 格式
#[test]
fn decrypt_and_migrate_upgrades_legacy_xor() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "sk-legacy-secret-to-migrate";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    assert!(SecretStore::needs_migration(&legacy_value));

    let (decrypted, migrated) = store.decrypt_and_migrate(&legacy_value).unwrap();
    assert_eq!(decrypted, plaintext, "Plaintext must match original");
    assert!(migrated.is_some(), "Legacy value should trigger migration");

    let new_value = migrated.unwrap();
    assert!(new_value.starts_with("enc2:"), "Migrated value must use enc2: prefix");
    assert!(
        !SecretStore::needs_migration(&new_value),
        "Migrated value should not need migration"
    );

    let (decrypted2, migrated2) = store.decrypt_and_migrate(&new_value).unwrap();
    assert_eq!(decrypted2, plaintext, "Migrated value must decrypt to same plaintext");
    assert!(migrated2.is_none(), "Migrated value should not trigger another migration");
}

// 测试迁移功能正确处理 Unicode 字符
#[test]
fn decrypt_and_migrate_handles_unicode() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "sk-日本語-émojis-🦀-тест";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let (decrypted, migrated) = store.decrypt_and_migrate(&legacy_value).unwrap();
    assert_eq!(decrypted, plaintext);
    assert!(migrated.is_some());

    let new_value = migrated.unwrap();
    let (decrypted2, _) = store.decrypt_and_migrate(&new_value).unwrap();
    assert_eq!(decrypted2, plaintext);
}

// 测试迁移功能正确处理空密钥
#[test]
fn decrypt_and_migrate_handles_empty_secret() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let (decrypted, migrated) = store.decrypt_and_migrate(&legacy_value).unwrap();
    assert_eq!(decrypted, plaintext);
    assert!(migrated.is_some());
    assert_eq!(migrated.unwrap(), "");
}

// 测试迁移功能正确处理长字符串
#[test]
fn decrypt_and_migrate_handles_long_secret() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "a".repeat(10_000);
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let (decrypted, migrated) = store.decrypt_and_migrate(&legacy_value).unwrap();
    assert_eq!(decrypted, plaintext);
    assert!(migrated.is_some());

    let new_value = migrated.unwrap();
    let (decrypted2, _) = store.decrypt_and_migrate(&new_value).unwrap();
    assert_eq!(decrypted2, plaintext);
}

// 测试损坏的旧版十六进制数据迁移失败
#[test]
fn decrypt_and_migrate_fails_on_corrupt_legacy_hex() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    let _ = store.encrypt("setup").unwrap();

    let result = store.decrypt_and_migrate("enc:not-valid-hex!!");
    assert!(result.is_err(), "Corrupt hex should fail");
}

// 测试使用错误密钥迁移时产生乱码或失败
#[test]
fn decrypt_and_migrate_wrong_key_produces_garbage_or_fails() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();
    let store1 = SecretStore::new(tmp1.path(), true);
    let store2 = SecretStore::new(tmp2.path(), true);

    let _ = store1.encrypt("setup").unwrap();
    let _ = store2.encrypt("setup").unwrap();
    let key1 = store1.load_or_create_key().unwrap();

    let plaintext = "secret-for-store1";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key1);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    match store2.decrypt_and_migrate(&legacy_value) {
        Ok((decrypted, _)) => {
            assert_ne!(decrypted, plaintext, "Wrong key should produce garbage plaintext");
        }
        Err(e) => {
            assert!(e.to_string().contains("UTF-8"), "Error should be UTF-8 related: {e}");
        }
    }
}

// 测试每次迁移产生不同的密文
#[test]
fn migration_produces_different_ciphertext_each_time() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "sk-same-secret";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let (_, migrated1) = store.decrypt_and_migrate(&legacy_value).unwrap();
    let (_, migrated2) = store.decrypt_and_migrate(&legacy_value).unwrap();

    assert!(migrated1.is_some());
    assert!(migrated2.is_some());
    assert_ne!(
        migrated1.unwrap(),
        migrated2.unwrap(),
        "Each migration should produce different ciphertext (random nonce)"
    );
}

// 测试迁移后的值具有防篡改能力
#[test]
fn migrated_value_is_tamper_resistant() {
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);

    let _ = store.encrypt("setup").unwrap();
    let key = store.load_or_create_key().unwrap();

    let plaintext = "sk-sensitive-data";
    let ciphertext = xor_cipher(plaintext.as_bytes(), &key);
    let legacy_value = format!("enc:{}", hex_encode(&ciphertext));

    let (_, migrated) = store.decrypt_and_migrate(&legacy_value).unwrap();
    let new_value = migrated.unwrap();

    let hex_str = &new_value[5..];
    let mut blob = hex_decode(hex_str).unwrap();
    if blob.len() > NONCE_LEN {
        blob[NONCE_LEN] ^= 0xff;
    }
    let tampered = format!("enc2:{}", hex_encode(&blob));

    let result = store.decrypt_and_migrate(&tampered);
    assert!(result.is_err(), "Tampered migrated value must be rejected");
}

// 测试 XOR 密码的加密解密往返
#[test]
fn xor_cipher_roundtrip() {
    let key = b"testkey123";
    let data = b"hello world";
    let encrypted = xor_cipher(data, key);
    let decrypted = xor_cipher(&encrypted, key);
    assert_eq!(decrypted, data);
}

// 测试空密钥时 XOR 密码直接透传数据
#[test]
fn xor_cipher_empty_key() {
    let data = b"passthrough";
    let result = xor_cipher(data, &[]);
    assert_eq!(result, data);
}

// 测试十六进制编码解码往返
#[test]
fn hex_roundtrip() {
    let data = vec![0x00, 0x01, 0xfe, 0xff, 0xab, 0xcd];
    let encoded = hex_encode(&data);
    assert_eq!(encoded, "0001feffabcd");
    let decoded = hex_decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

// 测试奇数长度十六进制字符串解码失败
#[test]
fn hex_decode_odd_length_fails() {
    assert!(hex_decode("abc").is_err());
}

// 测试无效字符的十六进制字符串解码失败
#[test]
fn hex_decode_invalid_chars_fails() {
    assert!(hex_decode("zzzz").is_err());
}

// 测试 Windows icacls 参数拒绝空用户名
#[test]
fn windows_icacls_grant_arg_rejects_empty_username() {
    use super::build_windows_icacls_grant_arg;
    assert_eq!(build_windows_icacls_grant_arg(""), None);
    assert_eq!(build_windows_icacls_grant_arg("   \t\n"), None);
}

// 测试 Windows icacls 参数自动修剪用户名空白
#[test]
fn windows_icacls_grant_arg_trims_username() {
    use super::build_windows_icacls_grant_arg;
    assert_eq!(build_windows_icacls_grant_arg("  alice  "), Some("alice:F".to_string()));
}

// 测试 Windows icacls 参数保留有效字符
#[test]
fn windows_icacls_grant_arg_preserves_valid_characters() {
    use super::build_windows_icacls_grant_arg;
    assert_eq!(
        build_windows_icacls_grant_arg("DOMAIN\\svc-user"),
        Some("DOMAIN\\svc-user:F".to_string())
    );
}

// 测试生成的随机密钥长度正确
#[test]
fn generate_random_key_correct_length() {
    let key = generate_random_key();
    assert_eq!(key.len(), KEY_LEN);
}

// 测试生成的随机密钥不全为零
#[test]
fn generate_random_key_not_all_zeros() {
    let key = generate_random_key();
    assert!(key.iter().any(|&b| b != 0), "Key should not be all zeros");
}

// 测试两次生成的随机密钥不同
#[test]
fn two_random_keys_differ() {
    let k1 = generate_random_key();
    let k2 = generate_random_key();
    assert_ne!(k1, k2, "Two random keys should differ");
}

// 测试随机密钥不包含 UUID 固定位模式
#[test]
fn generate_random_key_has_no_uuid_fixed_bits() {
    let mut version_match = 0;
    let mut variant_match = 0;
    let samples = 100;
    for _ in 0..samples {
        let key = generate_random_key();
        if key[6] & 0xf0 == 0x40 {
            version_match += 1;
        }
        if key[8] & 0xc0 == 0x80 {
            variant_match += 1;
        }
    }
    assert!(
        version_match < 30,
        "byte[6] matched UUID v4 version nibble {version_match}/100 times — \
         likely still using UUID-based key generation"
    );
    assert!(
        variant_match < 50,
        "byte[8] matched UUID v4 variant bits {variant_match}/100 times — \
         likely still using UUID-based key generation"
    );
}

// 测试密钥文件权限仅限所有者访问
#[cfg(unix)]
#[test]
fn key_file_has_restricted_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let store = SecretStore::new(tmp.path(), true);
    store.encrypt("trigger key creation").unwrap();

    let perms = std::fs::metadata(&store.key_path).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o600, "Key file must be owner-only (0600)");
}
