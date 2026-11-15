use std::fs;
use tempfile::tempdir;
use config_security::OtpConfig;
use crate::otp::OtpValidator;
use crate::secrets::SecretStore;

fn test_config() -> OtpConfig {
    OtpConfig { enabled: true, token_ttl_secs: 30, cache_valid_secs: 120, ..OtpConfig::default() }
}

// 测试有效的 TOTP 验证码能够通过验证
#[test]
fn valid_totp_code_is_accepted() {
    let dir = tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

    let now = 1_700_000_000u64;
    let code = validator.code_for_timestamp(now);
    assert!(validator.validate_at(&code, now).unwrap());
}

// 测试重放的 TOTP 验证码会被拒绝
#[test]
fn replayed_totp_code_is_rejected() {
    let dir = tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

    let now = 1_700_000_000u64;
    let code = validator.code_for_timestamp(now);
    assert!(validator.validate_at(&code, now).unwrap());
    assert!(!validator.validate_at(&code, now).unwrap());
}

// 测试过期的 TOTP 验证码会被拒绝
#[test]
fn expired_totp_code_is_rejected() {
    let dir = tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

    let stale = 1_700_000_000u64;
    let now = stale + 300;
    let code = validator.code_for_timestamp(stale);
    assert!(!validator.validate_at(&code, now).unwrap());
}

// 测试错误的 TOTP 验证码会被拒绝
#[test]
fn wrong_totp_code_is_rejected() {
    let dir = tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
    assert!(!validator.validate_at("123456", 1_700_000_000).unwrap());
}

// 测试密钥能够正确生成并以加密形式存储和重用
#[test]
fn secret_is_generated_and_reused() {
    let dir = tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);

    let (first, first_uri) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
    assert!(first_uri.is_some());

    let secret_path = crate::otp::secret_file_path(dir.path());
    let stored = fs::read_to_string(&secret_path).unwrap();
    assert!(SecretStore::is_encrypted(stored.trim()));

    let (second, second_uri) =
        OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
    assert!(second_uri.is_none());

    let ts = 1_700_000_000u64;
    assert_eq!(first.code_for_timestamp(ts), second.code_for_timestamp(ts));
}
