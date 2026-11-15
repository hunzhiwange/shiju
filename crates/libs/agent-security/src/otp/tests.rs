//! OTP（一次性密码）模块的单元测试
//!
//! 本模块包含对 OTP 验证器功能的全面测试，主要验证：
//! - TOTP（基于时间的一次性密码）的生成和验证
//! - 重放攻击防护机制
//! - 过期验证码的拒绝机制
//! - 密钥的生成、加密存储和重用
//!
//! # 测试覆盖范围
//!
//! - 正向场景：有效验证码的接受
//! - 安全场景：重放攻击防护、过期验证码拒绝、错误验证码拒绝
//! - 持久化场景：密钥的生成、加密存储和重用

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    /// 创建用于测试的 OTP 配置
    ///
    /// 返回一个启用了 OTP 功能的配置实例，包含以下参数：
    /// - `enabled`: true - 启用 OTP 功能
    /// - `token_ttl_secs`: 30 - 验证码有效期 30 秒
    /// - `cache_valid_secs`: 120 - 缓存有效期 120 秒
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `OtpConfig` 实例，其他字段使用默认值
    fn test_config() -> OtpConfig {
        OtpConfig {
            enabled: true,
            token_ttl_secs: 30,
            cache_valid_secs: 120,
            ..OtpConfig::default()
        }
    }

    /// 测试有效的 TOTP 验证码能够被正确接受
    ///
    /// 验证场景：
    /// 1. 创建临时目录和密钥存储
    /// 2. 初始化 OTP 验证器
    /// 3. 为指定时间戳生成验证码
    /// 4. 验证该验证码能够被接受
    ///
    /// # 预期结果
    ///
    /// 有效时间戳生成的验证码应该通过验证，返回 `Ok(true)`
    #[test]
    fn valid_totp_code_is_accepted() {
        // 创建临时目录用于存储密钥
        let dir = tempdir().unwrap();
        // 创建密钥存储（启用加密）
        let store = SecretStore::new(dir.path(), true);
        // 从配置初始化 OTP 验证器
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        // 设置固定的时间戳用于测试
        let now = 1_700_000_000u64;
        // 为该时间戳生成验证码
        let code = validator.code_for_timestamp(now);
        // 验证生成的验证码应该被接受
        assert!(validator.validate_at(&code, now).unwrap());
    }

    /// 测试重放的 TOTP 验证码会被正确拒绝
    ///
    /// 验证场景：
    /// 1. 创建 OTP 验证器
    /// 2. 为指定时间戳生成验证码
    /// 3. 第一次验证该验证码（应该成功）
    /// 4. 再次使用相同的验证码进行验证（应该失败）
    ///
    /// # 预期结果
    ///
    /// - 第一次验证返回 `Ok(true)`
    /// - 第二次验证返回 `Ok(false)`，表示重放攻击被成功防护
    ///
    /// # 安全意义
    ///
    /// 此测试验证了 OTP 系统的重放攻击防护机制，即使验证码在有效期内，
    /// 也只能使用一次，防止攻击者截获并重用验证码
    #[test]
    fn replayed_totp_code_is_rejected() {
        // 初始化验证器环境
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        // 设置测试时间戳
        let now = 1_700_000_000u64;
        // 生成验证码
        let code = validator.code_for_timestamp(now);
        // 第一次验证：应该成功
        assert!(validator.validate_at(&code, now).unwrap());
        // 第二次验证：应该失败（重放攻击防护）
        assert!(!validator.validate_at(&code, now).unwrap());
    }

    /// 测试过期的 TOTP 验证码会被正确拒绝
    ///
    /// 验证场景：
    /// 1. 为过去的时间戳（stale）生成验证码
    /// 2. 在当前时间（now）尝试验证该过期验证码
    /// 3. 时间差设置为 300 秒（5 分钟），远超配置的 token_ttl_secs（30 秒）
    ///
    /// # 预期结果
    ///
    /// 过期验证码的验证应该返回 `Ok(false)`，表示被拒绝
    ///
    /// # 安全意义
    ///
    /// 此测试验证了 OTP 系统的时间窗口机制，确保验证码只能在有效期内使用，
    /// 防止攻击者使用旧的验证码进行认证
    #[test]
    fn expired_totp_code_is_rejected() {
        // 初始化验证器环境
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        // 设置过期时间戳（5 分钟前）
        let stale = 1_700_000_000u64;
        // 当前时间戳（5 分钟后）
        let now = stale + 300;
        // 为过期时间生成验证码
        let code = validator.code_for_timestamp(stale);
        // 验证应该失败：验证码已过期
        assert!(!validator.validate_at(&code, now).unwrap());
    }

    /// 测试错误的 TOTP 验证码会被正确拒绝
    ///
    /// 验证场景：
    /// 1. 初始化 OTP 验证器
    /// 2. 使用一个固定的错误验证码（"123456"）尝试验证
    /// 3. 该验证码与实际生成的验证码不匹配
    ///
    /// # 预期结果
    ///
    /// 错误验证码的验证应该返回 `Ok(false)`，表示被拒绝
    ///
    /// # 安全意义
    ///
    /// 此测试验证了 OTP 系统的验证码匹配机制，确保只有正确的验证码才能通过验证，
    /// 防止暴力破解或猜测攻击
    #[test]
    fn wrong_totp_code_is_rejected() {
        // 初始化验证器环境
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
        // 使用错误的验证码进行验证，应该失败
        assert!(!validator.validate_at("123456", 1_700_000_000).unwrap());
    }

    /// 测试密钥的生成、加密存储和重用机制
    ///
    /// 验证场景：
    /// 1. 第一次初始化验证器：
    ///    - 应该生成新的密钥
    ///    - 返回的 URI 应该是 Some（包含 otpauth:// 协议的 URI）
    ///    - 密钥应该被加密存储到文件系统
    /// 2. 第二次初始化验证器（相同目录）：
    ///    - 应该重用已存储的密钥
    ///    - 返回的 URI 应该是 None（不再提供，因为密钥已存在）
    ///    - 两次验证器为相同时间戳生成的验证码应该相同
    ///
    /// # 预期结果
    ///
    /// - 第一次初始化返回 `Some(uri)`
    /// - 存储的密钥文件内容应该是加密的
    /// - 第二次初始化返回 `None`
    /// - 两次初始化的验证器生成的验证码完全一致
    ///
    /// # 安全意义
    ///
    /// 此测试验证了密钥的持久化和加密存储机制：
    /// - 密钥在首次生成后会被加密保存
    /// - 系统重启后可以重用密钥，保持验证器的一致性
    /// - 密钥文件使用加密存储，防止明文泄露
    #[test]
    fn secret_is_generated_and_reused() {
        // 创建临时目录
        let dir = tempdir().unwrap();
        // 创建密钥存储（启用加密）
        let store = SecretStore::new(dir.path(), true);

        // 第一次初始化：应该生成新密钥
        let (first, first_uri) =
            OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
        // 第一次应该返回 URI（用于用户添加到认证器应用）
        assert!(first_uri.is_some());

        // 验证密钥文件已存储且已加密
        let secret_path = secret_file_path(dir.path());
        let stored = fs::read_to_string(&secret_path).unwrap();
        // 确认存储的内容是加密的
        assert!(SecretStore::is_encrypted(stored.trim()));

        // 第二次初始化：应该重用已存储的密钥
        let (second, second_uri) =
            OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();
        // 第二次不应该返回 URI（密钥已存在）
        assert!(second_uri.is_none());

        // 验证两次初始化的验证器使用相同的密钥
        let ts = 1_700_000_000u64;
        // 为相同时间戳生成的验证码应该完全相同
        assert_eq!(first.code_for_timestamp(ts), second.code_for_timestamp(ts));
    }

    #[test]
    fn invalid_code_formats_are_rejected_without_cache_side_effects() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        for code in ["", "12345", "1234567", "12 456", "abcdef", "１２３４５６"] {
            assert!(!validator.validate_at(code, 1_700_000_000).unwrap(), "{code:?}");
        }

        let valid = validator.code_for_timestamp(1_700_000_000);
        assert!(validator.validate_at(&format!(" {valid}\n"), 1_700_000_000).unwrap());
    }

    #[test]
    fn public_validate_rejects_malformed_code() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        assert!(!validator.validate("not-a-code").unwrap());
    }

    #[test]
    fn adjacent_time_windows_are_accepted_once() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        let (validator, _) = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap();

        let now = 1_700_000_060u64;
        let previous_window_code = validator.code_for_timestamp(now - 30);
        let next_window_code = validator.code_for_timestamp(now + 30);

        assert!(validator.validate_at(&previous_window_code, now).unwrap());
        assert!(validator.validate_at(&next_window_code, now).unwrap());
        assert!(!validator.validate_at(&previous_window_code, now).unwrap());
    }

    #[test]
    fn expired_replay_cache_allows_same_window_code_again() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), false);
        let config = OtpConfig { token_ttl_secs: 300, cache_valid_secs: 1, ..test_config() };
        let (validator, _) = OtpValidator::from_config(&config, dir.path(), &store).unwrap();

        let now = 1_700_000_000u64;
        let code = validator.code_for_timestamp(now);

        assert!(validator.validate_at(&code, now).unwrap());
        assert!(validator.validate_at(&code, now + 2).unwrap());
    }

    #[test]
    fn zero_ttl_uses_one_second_period_in_uri_and_codes() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), false);
        let config = OtpConfig { token_ttl_secs: 0, cache_valid_secs: 1, ..test_config() };
        let (validator, uri) = OtpValidator::from_config(&config, dir.path(), &store).unwrap();

        assert!(uri.as_deref().unwrap().contains("period=1"));
        assert_ne!(validator.code_for_timestamp(100), validator.code_for_timestamp(101));
    }

    #[test]
    fn existing_secret_directory_reports_read_failure() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), false);
        fs::create_dir_all(secret_file_path(dir.path())).unwrap();

        let err = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap_err();
        assert!(format!("{err:#}").contains("Failed to read OTP secret file"));
    }

    #[test]
    fn encrypted_existing_secret_reports_decrypt_failure() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), true);
        fs::write(secret_file_path(dir.path()), "enc2:not-hex").unwrap();

        let err = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap_err();
        assert!(format!("{err:#}").contains("Failed to decrypt OTP secret file"));
    }

    #[test]
    fn secret_write_reports_parent_directory_failure() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("not-a-directory");
        fs::write(&config_path, "file blocks directory creation").unwrap();
        let store = SecretStore::new(&config_path, false);

        let err = OtpValidator::from_config(&test_config(), &config_path, &store).unwrap_err();
        assert!(format!("{err:#}").contains("Failed to create directory"));
    }

    #[test]
    fn corrupt_existing_secret_is_reported() {
        let dir = tempdir().unwrap();
        let store = SecretStore::new(dir.path(), false);
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(secret_file_path(dir.path()), "not base32!").unwrap();

        let err = OtpValidator::from_config(&test_config(), dir.path(), &store).unwrap_err();
        assert!(format!("{err:#}").to_ascii_lowercase().contains("base32"));
    }

    #[test]
    fn base32_roundtrip_and_known_totp_vector_are_stable() {
        let secret = b"12345678901234567890";
        let encoded = encode_base32_secret(secret);
        assert_eq!(decode_base32_secret(&encoded).unwrap(), secret);
        assert_eq!(compute_totp_code(secret, 1), "287082");
    }

    #[test]
    fn base32_decoder_reports_empty_and_incomplete_secrets() {
        let empty = decode_base32_secret(" \t-\n==").unwrap_err();
        assert!(format!("{empty:#}").contains("OTP secret is empty"));

        let incomplete = decode_base32_secret("A").unwrap_err();
        assert!(format!("{incomplete:#}").contains("did not decode to any bytes"));

        let invalid = decode_base32_secret("JBSW*3DP").unwrap_err();
        assert!(format!("{invalid:#}").contains("invalid base32 character '*'"));
    }

    #[test]
    fn base32_helpers_handle_empty_input_and_normalized_encoding() {
        assert_eq!(encode_base32_secret(&[]), "");
        assert_eq!(encode_base32_secret(b"f"), "MY");
        assert_eq!(decode_base32_secret("jbsw-y3dp====").unwrap(), b"Hello");
    }

    #[test]
    fn secret_file_path_appends_fixed_filename() {
        assert!(secret_file_path(Path::new("/tmp/vw")).ends_with("otp-secret"));
    }
}
