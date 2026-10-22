//! 加密密钥存储模块 — 为 API 密钥和令牌提供深度防御。
//!
//! # 模块概述
//!
//! 本模块实现了安全的密钥加密存储机制，使用 ChaCha20-Poly1305 AEAD（认证加密）算法，
//! 密钥以受限文件权限（0600）存储在 `~/.shiju-agent/.secret_key` 文件中。
//! 配置文件仅存储十六进制编码的密文，绝不存储明文密钥。
//!
//! # 加密机制
//!
//! 每次加密都会生成新的 12 字节随机 nonce，并预置到密文之前。
//! Poly1305 认证标签防止密文被篡改。
//!
//! # 安全特性
//!
//! 本机制可防止以下安全威胁：
//!   - 配置文件中的明文泄露
//!   - 通过 `grep` 或 `git log` 的意外泄露
//!   - 意外提交原始 API 密钥
//!   - 已知明文攻击（不同于之前的 XOR 密码）
//!   - 密文篡改（认证加密）
//!
//! # 明文模式
//!
//! 对于偏好明文的自主用户，可通过设置 `secrets.encrypt = false` 禁用加密。
//!
//! # 迁移兼容性
//!
//! 带有旧版 `enc:` 前缀的值（XOR 密码）使用旧算法解密以保证向后兼容性。
//! 新加密始终生成 `enc2:` 前缀（ChaCha20-Poly1305）。
//!
//! # 示例
//!
//! ```rust,ignore
//! use std::path::Path;
//! use vibewindow::security::SecretStore;
//!
//! // 创建密钥存储实例
//! let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
//!
//! // 加密密钥
//! let encrypted = store.encrypt("my_api_key")?;
//! assert!(encrypted.starts_with("enc2:"));
//!
//! // 解密密钥
//! let decrypted = store.decrypt(&encrypted)?;
//! assert_eq!(decrypted, "my_api_key");
//! ```

use anyhow::{Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, Nonce};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 随机加密密钥的长度（以字节为单位，256位，匹配 ChaCha20 算法要求）。
const KEY_LEN: usize = 32;

/// ChaCha20-Poly1305 的 nonce 长度（以字节为单位）。
const NONCE_LEN: usize = 12;

/// 密钥加密存储管理器。
///
/// 负责管理 API 密钥、令牌等敏感信息的加密存储。
/// 支持加密/解密操作以及从旧格式到新格式的迁移。
///
/// # 安全性
///
/// - 使用 ChaCha20-Poly1305 AEAD 算法进行加密
/// - 密钥文件具有严格的文件权限（0600）
/// - 支持禁用加密的明文模式（不推荐）
///
/// # 示例
///
/// ```rust,ignore
/// let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
/// let encrypted = store.encrypt("secret_key")?;
/// let decrypted = store.decrypt(&encrypted)?;
/// ```
#[derive(Debug, Clone)]
pub struct SecretStore {
    /// 密钥文件路径（`~/.shiju-agent/.secret_key`）。
    key_path: PathBuf,
    /// 是否启用加密功能。
    enabled: bool,
}

impl SecretStore {
    /// 创建新的密钥存储实例。
    ///
    /// # 参数
    ///
    /// - `vibewindow_dir`: VibeWindow 配置目录路径，密钥文件将存储在此目录下的 `.secret_key` 文件中
    /// - `enabled`: 是否启用加密功能，`true` 启用加密，`false` 则使用明文存储
    ///
    /// # 返回值
    ///
    /// 返回新创建的 `SecretStore` 实例。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::path::Path;
    ///
    /// let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
    /// ```
    pub fn new(vibewindow_dir: &Path, enabled: bool) -> Self {
        Self { key_path: vibewindow_dir.join(".secret_key"), enabled }
    }

    /// 加密明文密钥。
    ///
    /// 返回以 `enc2:` 为前缀的十六进制编码密文。
    /// 格式：`enc2:<hex(nonce ‖ ciphertext ‖ tag)>`（12 + N + 16 字节）。
    ///
    /// 如果加密被禁用或明文为空，则直接返回原始明文。
    ///
    /// # 参数
    ///
    /// - `plaintext`: 要加密的明文字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 加密后的字符串（`enc2:` 前缀 + 十六进制密文）或原始明文（未启用加密）
    /// - `Err`: 加密失败时返回错误
    ///
    /// # 错误
    ///
    /// 以下情况可能返回错误：
    /// - 密钥文件读取或创建失败
    /// - 加密算法执行失败
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
    /// let encrypted = store.encrypt("my_secret")?;
    /// assert!(encrypted.starts_with("enc2:"));
    /// ```
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // 如果未启用加密或明文为空，直接返回原始值
        if !self.enabled || plaintext.is_empty() {
            return Ok(plaintext.to_string());
        }

        // 加载或创建加密密钥
        let key_bytes = self.load_or_create_key()?;
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);

        // 生成随机 nonce 并执行加密
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {e}"))?;

        // 将 nonce 预置到密文之前以便存储
        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);

        // 返回带前缀的十六进制编码结果
        Ok(format!("enc2:{}", hex_encode(&blob)))
    }

    /// 解密密钥值。
    ///
    /// 根据前缀自动识别并处理不同的格式：
    /// - `enc2:` 前缀 → ChaCha20-Poly1305（当前安全格式）
    /// - `enc:` 前缀 → 旧版 XOR 密码（向后兼容迁移）
    /// - 无前缀 → 直接返回（明文配置）
    ///
    /// # 参数
    ///
    /// - `value`: 要解密的字符串值
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 解密后的明文字符串
    /// - `Err`: 解密失败时返回错误
    ///
    /// # 警告
    ///
    /// 旧版 `enc:` 值是不安全的。建议使用 `decrypt_and_migrate` 方法
    /// 自动将它们升级为安全的 `enc2:` 格式。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
    ///
    /// // 解密新格式
    /// let plaintext = store.decrypt("enc2:...")?;
    ///
    /// // 解密旧格式
    /// let legacy = store.decrypt("enc:...")?;
    ///
    /// // 返回明文
    /// let plain = store.decrypt("plain_text")?;
    /// ```
    pub fn decrypt(&self, value: &str) -> Result<String> {
        if let Some(hex_str) = value.strip_prefix("enc2:") {
            self.decrypt_chacha20(hex_str)
        } else if let Some(hex_str) = value.strip_prefix("enc:") {
            self.decrypt_legacy_xor(hex_str)
        } else {
            Ok(value.to_string())
        }
    }

    /// 解密密钥并在需要时返回迁移后的值。
    ///
    /// 如果输入使用旧版 `enc:` 格式，将返回迁移后的 `enc2:` 值。
    ///
    /// # 参数
    ///
    /// - `value`: 要解密的字符串值
    ///
    /// # 返回值
    ///
    /// 返回元组 `(明文字符串, Option<迁移后的加密值>)`：
    /// - 如果发生了迁移，返回 `(plaintext, Some(new_enc2_value))`
    /// - 如果无需迁移，返回 `(plaintext, None)`
    ///
    /// # 用途
    ///
    /// 允许调用者将升级后的值持久化回配置文件。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let store = SecretStore::new(Path::new("~/.shiju-agent"), true);
    ///
    /// // 解密旧格式并获取迁移后的新格式
    /// let (plaintext, migrated) = store.decrypt_and_migrate("enc:...")?;
    /// if let Some(new_encrypted) = migrated {
    ///     // 将 new_encrypted 保存到配置文件
    ///     config.api_key = new_encrypted;
    /// }
    /// ```
    pub fn decrypt_and_migrate(&self, value: &str) -> Result<(String, Option<String>)> {
        if let Some(hex_str) = value.strip_prefix("enc2:") {
            // 已使用安全格式 — 无需迁移
            let plaintext = self.decrypt_chacha20(hex_str)?;
            Ok((plaintext, None))
        } else if let Some(hex_str) = value.strip_prefix("enc:") {
            // 旧版 XOR 密码 — 解密并使用 ChaCha20-Poly1305 重新加密
            tracing::warn!(
                "Decrypting legacy XOR-encrypted secret (enc: prefix). \
                 This format is insecure and will be removed in a future release. \
                 The secret will be automatically migrated to enc2: (ChaCha20-Poly1305)."
            );
            let plaintext = self.decrypt_legacy_xor(hex_str)?;
            let migrated = self.encrypt(&plaintext)?;
            Ok((plaintext, Some(migrated)))
        } else {
            // 明文 — 无需迁移
            Ok((value.to_string(), None))
        }
    }

    /// 检查值是否使用需要迁移的旧版 `enc:` 格式。
    ///
    /// # 参数
    ///
    /// - `value`: 要检查的字符串值
    ///
    /// # 返回值
    ///
    /// 如果值以 `enc:` 开头则返回 `true`，否则返回 `false`。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// assert!(SecretStore::needs_migration("enc:abc123"));
    /// assert!(!SecretStore::needs_migration("enc2:abc123"));
    /// assert!(!SecretStore::needs_migration("plain_text"));
    /// ```
    pub fn needs_migration(value: &str) -> bool {
        value.starts_with("enc:")
    }

    /// 使用 ChaCha20-Poly1305 解密（当前安全格式）。
    ///
    /// # 参数
    ///
    /// - `hex_str`: 十六进制编码的加密数据（不含 `enc2:` 前缀）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 解密后的明文字符串
    /// - `Err`: 解密失败时返回错误
    ///
    /// # 错误
    ///
    /// 以下情况可能返回错误：
    /// - 十六进制解码失败（损坏的十六进制数据）
    /// - 加密数据太短（缺少 nonce）
    /// - 解密失败（错误的密钥或被篡改的数据）
    /// - 解密后的数据不是有效的 UTF-8
    fn decrypt_chacha20(&self, hex_str: &str) -> Result<String> {
        // 解码十六进制数据
        let blob =
            hex_decode(hex_str).context("Failed to decode encrypted secret (corrupt hex)")?;

        // 验证数据长度
        anyhow::ensure!(blob.len() > NONCE_LEN, "Encrypted value too short (missing nonce)");

        // 分离 nonce 和密文
        let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 加载密钥并创建解密器
        let key_bytes = self.load_or_create_key()?;
        let key = Key::from_slice(&key_bytes);
        let cipher = ChaCha20Poly1305::new(key);

        // 执行解密
        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Decryption failed — wrong key or tampered data"))?;

        // 将解密结果转换为 UTF-8 字符串
        String::from_utf8(plaintext_bytes)
            .context("Decrypted secret is not valid UTF-8 — corrupt data")
    }

    /// 使用旧版 XOR 密码解密（不安全，仅用于向后兼容）。
    ///
    /// # 参数
    ///
    /// - `hex_str`: 十六进制编码的加密数据（不含 `enc:` 前缀）
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 解密后的明文字符串
    /// - `Err`: 解密失败时返回错误
    ///
    /// # 错误
    ///
    /// 以下情况可能返回错误：
    /// - 十六进制解码失败（损坏的十六进制数据）
    /// - 解密后的数据不是有效的 UTF-8（错误的密钥或损坏的数据）
    ///
    /// # 安全性警告
    ///
    /// 此方法使用不安全的 XOR 密码，仅用于向后兼容。
    /// 建议尽快迁移到 `enc2:` 格式。
    fn decrypt_legacy_xor(&self, hex_str: &str) -> Result<String> {
        // 解码十六进制数据
        let ciphertext = hex_decode(hex_str)
            .context("Failed to decode legacy encrypted secret (corrupt hex)")?;

        // 加载密钥并执行 XOR 解密
        let key = self.load_or_create_key()?;
        let plaintext_bytes = xor_cipher(&ciphertext, &key);

        // 将解密结果转换为 UTF-8 字符串
        String::from_utf8(plaintext_bytes)
            .context("Decrypted legacy secret is not valid UTF-8 — wrong key or corrupt data")
    }

    /// 检查值是否已加密（当前格式或旧版格式）。
    ///
    /// # 参数
    ///
    /// - `value`: 要检查的字符串值
    ///
    /// # 返回值
    ///
    /// 如果值以 `enc2:` 或 `enc:` 开头则返回 `true`，否则返回 `false`。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// assert!(SecretStore::is_encrypted("enc2:abc123"));
    /// assert!(SecretStore::is_encrypted("enc:abc123"));
    /// assert!(!SecretStore::is_encrypted("plain_text"));
    /// ```
    pub fn is_encrypted(value: &str) -> bool {
        value.starts_with("enc2:")
            || value.starts_with("enc:")
            || value.starts_with("encrypted:v1:")
            || value.starts_with("encrypted:v2:")
    }

    /// 检查值是否使用安全的 `enc2:` 格式。
    ///
    /// # 参数
    ///
    /// - `value`: 要检查的字符串值
    ///
    /// # 返回值
    ///
    /// 如果值以 `enc2:` 开头则返回 `true`，否则返回 `false`。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// assert!(SecretStore::is_secure_encrypted("enc2:abc123"));
    /// assert!(!SecretStore::is_secure_encrypted("enc:abc123"));
    /// assert!(!SecretStore::is_secure_encrypted("plain_text"));
    /// ```
    pub fn is_secure_encrypted(value: &str) -> bool {
        value.starts_with("enc2:") || value.starts_with("encrypted:v2:")
    }

    /// 从磁盘加载加密密钥，如果不存在则创建新密钥。
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<u8>)`: 32 字节的加密密钥
    /// - `Err`: 读取或创建密钥文件失败时返回错误
    ///
    /// # 错误
    ///
    /// 以下情况可能返回错误：
    /// - 读取现有密钥文件失败
    /// - 密钥文件内容损坏
    /// - 创建目录或文件失败
    /// - 设置文件权限失败
    ///
    /// # 并发安全
    ///
    /// 此方法处理并发创建密钥文件的竞态条件。如果另一个进程
    /// 同时创建了密钥文件，本方法将读取并使用该文件。
    ///
    /// # 平台特定行为
    ///
    /// - **Unix**: 设置文件权限为 0600（仅所有者可读写）
    /// - **Windows**: 使用 icacls 限制权限为当前用户
    fn load_or_create_key(&self) -> Result<Vec<u8>> {
        // 如果密钥文件已存在，直接读取
        if self.key_path.exists() {
            let hex_key =
                fs::read_to_string(&self.key_path).context("Failed to read secret key file")?;
            hex_decode(hex_key.trim()).context("Secret key file is corrupt")
        } else {
            // 生成新的随机密钥
            let key = generate_random_key();

            // 确保父目录存在
            if let Some(parent) = self.key_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let key_hex = hex_encode(&key);

            // 使用 create_new 标志创建文件（原子操作，如果文件已存在则失败）
            match fs::OpenOptions::new().create_new(true).write(true).open(&self.key_path) {
                Ok(mut key_file) => {
                    // 在写入密钥字节之前设置限制性权限
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        key_file
                            .set_permissions(fs::Permissions::from_mode(0o600))
                            .context("Failed to set key file permissions")?;
                    }

                    // 写入密钥并同步到磁盘
                    key_file
                        .write_all(key_hex.as_bytes())
                        .context("Failed to write secret key file")?;
                    key_file.sync_all().context("Failed to fsync secret key file")?;
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    // 并发创建者赢得了竞争；读取已存在的密钥
                    let hex_key = fs::read_to_string(&self.key_path)
                        .context("Failed to read concurrently created secret key file")?;
                    return hex_decode(hex_key.trim())
                        .context("Secret key file is corrupt after concurrent create");
                }
                Err(err) => {
                    return Err(err).context("Failed to create secret key file");
                }
            }

            // Windows 平台：使用 icacls 限制权限
            #[cfg(windows)]
            {
                // 在 Windows 上，使用 icacls 将权限限制为仅当前用户
                let username = std::env::var("USERNAME").unwrap_or_default();
                let Some(grant_arg) = build_windows_icacls_grant_arg(&username) else {
                    tracing::warn!(
                        "USERNAME environment variable is empty; \
                         cannot restrict key file permissions via icacls"
                    );
                    return Ok(key);
                };

                // 执行 icacls 命令设置权限
                match std::process::Command::new("icacls")
                    .arg(&self.key_path)
                    .args(["/inheritance:r", "/grant:r"])
                    .arg(grant_arg)
                    .output()
                {
                    Ok(o) if !o.status.success() => {
                        tracing::warn!(
                            "Failed to set key file permissions via icacls (exit code {:?})",
                            o.status.code()
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Could not set key file permissions: {e}");
                    }
                    _ => {
                        tracing::debug!("Key file permissions restricted via icacls");
                    }
                }
            }

            Ok(key)
        }
    }
}

/// XOR 密码函数，使用重复密钥。
///
/// 由于 XOR 的对称性，同一个函数既用于加密也用于解密。
///
/// # 参数
///
/// - `data`: 要加密/解密的数据
/// - `key`: 密钥字节
///
/// # 返回值
///
/// 返回加密/解密后的数据向量。
///
/// # 安全性警告
///
/// 此函数实现的是不安全的 XOR 密码，仅用于向后兼容旧版格式。
/// 不应在新代码中使用此函数。
///
/// # 示例
///
/// ```rust,ignore
/// let encrypted = xor_cipher(b"hello", b"key");
/// let decrypted = xor_cipher(&encrypted, b"key");
/// assert_eq!(decrypted, b"hello");
/// ```
fn xor_cipher(data: &[u8], key: &[u8]) -> Vec<u8> {
    // 如果密钥为空，直接返回数据的副本
    if key.is_empty() {
        return data.to_vec();
    }
    // 对每个字节与密钥的对应字节进行 XOR 操作（密钥循环使用）
    data.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect()
}

/// 使用操作系统 CSPRNG 生成随机 256 位密钥。
///
/// 直接使用 `OsRng`（通过 `getrandom`），提供完整的 256 位熵，
/// 而不像 UUID v4 那样包含固定的版本/变体位。
///
/// # 返回值
///
/// 返回包含 32 字节随机密钥的向量。
///
/// # 安全性
///
/// 此函数使用操作系统的加密安全随机数生成器（CSPRNG），
/// 适用于加密密钥生成。
///
/// # 示例
///
/// ```rust,ignore
/// let key = generate_random_key();
/// assert_eq!(key.len(), 32);
/// ```
fn generate_random_key() -> Vec<u8> {
    ChaCha20Poly1305::generate_key(&mut OsRng).to_vec()
}

/// 将字节编码为小写十六进制字符串。
///
/// # 参数
///
/// - `data`: 要编码的字节数据
///
/// # 返回值
///
/// 返回小写十六进制编码的字符串。
///
/// # 示例
///
/// ```rust,ignore
/// let hex = hex_encode(&[0x12, 0xab, 0xff]);
/// assert_eq!(hex, "12abff");
/// ```
fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// 为 `icacls` 命令构建 `/grant` 参数，使用规范化的用户名。
///
/// # 参数
///
/// - `username`: Windows 用户名
///
/// # 返回值
///
/// - `Some(String)`: 格式为 `<username>:F` 的授权参数字符串
/// - `None`: 当用户名为空或仅包含空白字符时
///
/// # 平台
///
/// 仅用于 Windows 平台，配合 `icacls` 命令设置文件权限。
///
/// # 示例
///
/// ```rust,ignore
/// let arg = build_windows_icacls_grant_arg("  JohnDoe  ");
/// assert_eq!(arg, Some("JohnDoe:F".to_string()));
///
/// let empty = build_windows_icacls_grant_arg("   ");
/// assert_eq!(empty, None);
/// ```
fn build_windows_icacls_grant_arg(username: &str) -> Option<String> {
    // 去除用户名两端的空白字符
    let normalized = username.trim();
    if normalized.is_empty() {
        return None;
    }
    // 格式：<username>:F （F 表示完全控制）
    Some(format!("{normalized}:F"))
}

/// 将十六进制字符串解码为字节数组。
///
/// # 参数
///
/// - `hex`: 要解码的十六进制字符串
///
/// # 返回值
///
/// - `Ok(Vec<u8>)`: 解码后的字节数组
/// - `Err`: 解码失败时返回错误
///
/// # 错误
///
/// 以下情况会返回错误：
/// - 十六进制字符串长度为奇数
/// - 包含无效的十六进制字符
///
/// # 示例
///
/// ```rust,ignore
/// let bytes = hex_decode("12abff")?;
/// assert_eq!(bytes, vec![0x12, 0xab, 0xff]);
///
/// // 奇数长度会报错
/// assert!(hex_decode("12a").is_err());
///
/// // 无效字符会报错
/// assert!(hex_decode("12xy").is_err());
/// ```
#[allow(clippy::manual_is_multiple_of)]
fn hex_decode(hex: &str) -> Result<Vec<u8>> {
    // 检查十六进制字符串长度是否为偶数
    if (hex.len() & 1) != 0 {
        anyhow::bail!("Hex string has odd length");
    }
    // 每两个字符解析为一个字节
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex at position {i}: {e}"))
        })
        .collect()
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod secrets_tests;
