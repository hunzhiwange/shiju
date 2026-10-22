//! OTP（一次性密码）验证模块
//!
//! 本模块实现了基于时间的一次性密码（TOTP，Time-based One-Time Password）验证功能，
//! 用于为 VibeWindow 代理系统提供双因素认证（2FA）支持。
//!
//! # 主要功能
//!
//! - **密钥管理**：自动生成或加载 OTP 密钥，使用加密存储保护密钥安全
//! - **TOTP 验证**：基于 RFC 6238 标准实现 TOTP 验证，支持时间窗口容错
//! - **防重放攻击**：通过缓存已使用的验证码，防止同一验证码被重复使用
//! - **otpauth URI 生成**：生成标准的 `otpauth://` URI，便于与认证器应用（如 Google Authenticator）集成
//!
//! # 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     OtpValidator                         │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
//! │  │   config    │  │   secret    │  │  cached_codes   │  │
//! │  │ (OtpConfig) │  │  (Vec<u8>)  │  │ (Mutex<HashMap>)│  │
//! │  └─────────────┘  └─────────────┘  └─────────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//!           │                  │                    │
//!           ▼                  ▼                    ▼
//!    token_ttl_secs     密钥文件（加密存储）    防重放缓存
//!    cache_valid_secs
//! ```
//!
//! # 安全特性
//!
//! - 密钥文件使用 `SecretStore` 加密存储
//! - Unix 系统上密钥文件权限设置为 0o600（仅所有者可读写）
//! - 使用原子写入（临时文件 + rename）确保密钥文件完整性
//! - 已使用的验证码会被缓存，防止重放攻击
//!
//! # 平台兼容性
//!
//! - **原生平台**：完整支持所有功能，使用 `ring` crate 进行 HMAC 计算
//! - **WASM**：仅提供存根实现，不支持密钥持久化和实际验证

use super::secrets::SecretStore;
use crate::app::agent::config::OtpConfig;
use anyhow::{Context, Result};
use parking_lot::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use ring::hmac;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// OTP 密钥文件名
///
/// 此文件存储在 VibeWindow 配置目录下，包含加密后的 Base32 编码密钥
const OTP_SECRET_FILE: &str = "otp-secret";

/// TOTP 验证码位数
///
/// 标准的 6 位数字验证码，与大多数认证器应用兼容
const OTP_DIGITS: u32 = 6;

/// OTP 发行者名称
///
/// 用于在认证器应用中显示的服务提供商名称
const OTP_ISSUER: &str = "VibeWindow";

/// OTP 验证器
///
/// 负责管理 TOTP 密钥并验证用户提供的一次性密码。
///
/// # 线程安全
///
/// 内部使用 `Mutex` 保护已缓存验证码的 `HashMap`，确保在多线程环境下安全使用。
///
/// # 示例
///
/// ```ignore
/// use vibe_agent::app::agent::security::otp::OtpValidator;
/// use vibe_agent::app::agent::config::OtpConfig;
///
/// let config = OtpConfig::default();
/// let (validator, uri) = OtpValidator::from_config(
///     &config,
///     &vibewindow_dir,
///     &secret_store,
/// )?;
///
/// // 如果是新创建的密钥，uri 包含 otpauth:// URI
/// if let Some(uri) = uri {
///     println!("请使用认证器应用扫描此 URI: {}", uri);
/// }
///
/// // 验证用户输入的验证码
/// let is_valid = validator.validate("123456")?;
/// ```
#[derive(Debug)]
pub struct OtpValidator {
    /// OTP 配置，包含令牌有效期和缓存有效期等参数
    config: OtpConfig,
    /// 原始密钥字节（20 字节，160 位）
    secret: Vec<u8>,
    /// 已使用验证码的缓存，用于防止重放攻击
    ///
    /// Key: 验证码字符串
    /// Value: 过期时间戳（Unix 秒）
    cached_codes: Mutex<HashMap<String, u64>>,
}

impl OtpValidator {
    /// 从配置创建 OTP 验证器
    ///
    /// 此方法会尝试从指定目录加载已存在的密钥文件。如果密钥文件不存在，
    /// 则会自动生成新的随机密钥并加密保存。
    ///
    /// # 参数
    ///
    /// - `config`：OTP 配置，包含令牌有效期和缓存有效期
    /// - `vibewindow_dir`：VibeWindow 配置目录路径，密钥文件将存储在此目录下
    /// - `store`：密钥存储服务，用于加密/解密密钥文件
    ///
    /// # 返回值
    ///
    /// 返回元组 `(OtpValidator, Option<String>)`：
    /// - `OtpValidator`：验证器实例
    /// - `Option<String>`：如果密钥是新创建的，返回 `otpauth://` URI；否则返回 `None`
    ///
    /// # 错误
    ///
    /// - 密钥文件存在但无法读取
    /// - 密钥文件解密失败
    /// - 密钥文件格式无效（非有效 Base32）
    /// - 新密钥创建时写入文件失败
    ///
    /// # 平台差异
    ///
    /// - **WASM**：返回空密钥的验证器，不进行文件操作
    /// - **原生平台**：正常执行密钥加载/创建逻辑
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (validator, maybe_uri) = OtpValidator::from_config(
    ///     &config,
    ///     Path::new("/home/user/.shiju-agent"),
    ///     &secret_store,
    /// )?;
    ///
    /// if let Some(uri) = maybe_uri {
    ///     // 新密钥已创建，需要用户配置认证器应用
    ///     println!("新密钥 URI: {}", uri);
    /// }
    /// ```
    pub fn from_config(
        config: &OtpConfig,
        vibewindow_dir: &Path,
        store: &SecretStore,
    ) -> Result<(Self, Option<String>)> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (vibewindow_dir, store);
            Ok((
                Self {
                    config: config.clone(),
                    secret: Vec::new(),
                    cached_codes: Mutex::new(HashMap::new()),
                },
                None,
            ))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 构建密钥文件路径
            let secret_path = secret_file_path(vibewindow_dir);

            // 尝试加载现有密钥或创建新密钥
            let (secret, generated) = if secret_path.exists() {
                // 读取并解密现有密钥文件
                let encoded = fs::read_to_string(&secret_path).with_context(|| {
                    format!("Failed to read OTP secret file {}", secret_path.display())
                })?;
                let decrypted =
                    store.decrypt(encoded.trim()).context("Failed to decrypt OTP secret file")?;
                (decode_base32_secret(&decrypted)?, false)
            } else {
                // 生成新的 20 字节随机密钥（160 位，符合 TOTP 标准）
                let raw: [u8; 20] = rand::random();
                let encoded_secret = encode_base32_secret(&raw);
                let encrypted =
                    store.encrypt(&encoded_secret).context("Failed to encrypt OTP secret")?;
                write_secret_file(&secret_path, &encrypted)?;
                (raw.to_vec(), true)
            };

            // 创建验证器实例
            let validator =
                Self { config: config.clone(), secret, cached_codes: Mutex::new(HashMap::new()) };

            // 如果是新创建的密钥，生成 otpauth URI 供用户配置认证器
            let uri = if generated { Some(validator.otpauth_uri()) } else { None };
            Ok((validator, uri))
        }
    }

    /// 验证用户提供的一次性密码
    ///
    /// 此方法会自动获取当前时间戳进行验证。为了处理时钟偏差，
    /// 验证时会检查当前时间窗口及其前后各一个窗口的验证码。
    ///
    /// # 参数
    ///
    /// - `code`：用户输入的 6 位数字验证码
    ///
    /// # 返回值
    ///
    /// - `Ok(true)`：验证码有效
    /// - `Ok(false)`：验证码无效（格式错误、已过期、已使用或与密钥不匹配）
    /// - `Err(...)`：验证过程中发生错误
    ///
    /// # 防重放机制
    ///
    /// 验证通过的验证码会被缓存 `cache_valid_secs` 秒，在此期间再次使用相同验证码将被拒绝。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let is_valid = validator.validate("123456")?;
    /// if is_valid {
    ///     println!("验证成功");
    /// } else {
    ///     println!("验证码无效");
    /// }
    /// ```
    pub fn validate(&self, code: &str) -> Result<bool> {
        self.validate_at(code, unix_timestamp_now())
    }

    /// 在指定时间戳验证一次性密码
    ///
    /// 这是 `validate` 方法的内部实现，允许指定任意时间戳，
    /// 主要用于测试目的。
    ///
    /// # 参数
    ///
    /// - `code`：用户输入的验证码
    /// - `now_secs`：Unix 时间戳（秒）
    ///
    /// # 返回值
    ///
    /// 返回验证码是否有效
    fn validate_at(&self, code: &str, now_secs: u64) -> Result<bool> {
        // 标准化验证码：去除空白字符
        let normalized = code.trim();

        // 快速格式检查：必须是 6 位纯数字
        if normalized.len() != OTP_DIGITS as usize
            || !normalized.chars().all(|ch| ch.is_ascii_digit())
        {
            return Ok(false);
        }

        // 检查防重放缓存
        {
            let mut cache = self.cached_codes.lock();

            // 清理已过期的缓存条目
            cache.retain(|_, expiry| *expiry >= now_secs);

            // 如果验证码仍在缓存中且未过期，拒绝使用（防重放）
            if cache.get(normalized).is_some_and(|expiry| *expiry >= now_secs) {
                return Ok(false);
            }
        }

        // 计算时间窗口计数器
        // step 是每个验证码的有效时间（秒）
        let step = self.config.token_ttl_secs.max(1);
        let counter = now_secs / step;

        // 为了容忍时钟偏差，检查前后三个时间窗口
        // 例如：如果 step=30，当前时间是 12:00:45
        // counter = 45/30 = 1
        // counters = [0, 1, 2] 即 [12:00:00-12:00:30, 12:00:30-12:01:00, 12:01:00-12:01:30]
        let counters = [counter.saturating_sub(1), counter, counter.saturating_add(1)];

        // 尝试匹配任意一个时间窗口的验证码
        let is_valid = counters
            .iter()
            .map(|c| compute_totp_code(&self.secret, *c))
            .any(|candidate| candidate == normalized);

        // 如果验证成功，将验证码加入防重放缓存
        if is_valid {
            let mut cache = self.cached_codes.lock();
            cache.insert(
                normalized.to_string(),
                now_secs.saturating_add(self.config.cache_valid_secs),
            );
        }

        Ok(is_valid)
    }

    /// 生成 otpauth URI
    ///
    /// 生成符合 Google Authenticator 格式的 `otpauth://` URI，
    /// 用户可以通过扫描二维码或手动输入此 URI 来配置认证器应用。
    ///
    /// # URI 格式
    ///
    /// ```text
    /// otpauth://totp/{issuer}:{account}?secret={secret}&issuer={issuer}&period={period}
    /// ```
    ///
    /// # 返回值
    ///
    /// 返回完整的 otpauth URI 字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let uri = validator.otpauth_uri();
    /// // otpauth://totp/VibeWindow:vibewindow?secret=JBSWY3DPEHPK3PXP&issuer=VibeWindow&period=30
    /// ```
    pub fn otpauth_uri(&self) -> String {
        let secret = encode_base32_secret(&self.secret);
        let account = "vibewindow";
        format!(
            "otpauth://totp/{issuer}:{account}?secret={secret}&issuer={issuer}&period={period}",
            issuer = OTP_ISSUER,
            period = self.config.token_ttl_secs.max(1)
        )
    }

    /// 为指定时间戳生成验证码（仅用于测试）
    ///
    /// 此方法跳过防重放检查，直接根据密钥和时间戳计算验证码，
    /// 仅用于单元测试。
    ///
    /// # 参数
    ///
    /// - `timestamp`：Unix 时间戳（秒）
    ///
    /// # 返回值
    ///
    /// 返回 6 位数字验证码字符串
    #[cfg(test)]
    pub(crate) fn code_for_timestamp(&self, timestamp: u64) -> String {
        let counter = timestamp / self.config.token_ttl_secs.max(1);
        compute_totp_code(&self.secret, counter)
    }
}

/// 获取 OTP 密钥文件的完整路径
///
/// # 参数
///
/// - `vibewindow_dir`：VibeWindow 配置目录路径
///
/// # 返回值
///
/// 返回密钥文件的完整路径 `{vibewindow_dir}/otp-secret`
pub fn secret_file_path(vibewindow_dir: &Path) -> PathBuf {
    vibewindow_dir.join(OTP_SECRET_FILE)
}

/// 原子性地写入密钥文件
///
/// 使用"临时文件 + rename"模式确保文件写入的原子性：
/// 1. 创建临时文件（带随机 UUID 后缀）
/// 2. 写入数据并同步到磁盘
/// 3. 原子性地重命名为目标文件
///
/// # 安全措施
///
/// - Unix 系统：设置文件权限为 0o600（仅所有者可读写）
/// - 使用 `sync_all()` 确保数据写入物理磁盘
/// - 使用 `rename()` 保证原子性
///
/// # 参数
///
/// - `path`：目标文件路径
/// - `value`：要写入的内容（加密后的字符串）
///
/// # 错误
///
/// - 创建目录失败
/// - 创建临时文件失败
/// - 设置权限失败
/// - 写入或同步失败
/// - 重命名失败
///
/// # 平台差异
///
/// - **WASM**：直接返回错误，不支持文件操作
/// - **Unix**：设置文件权限为 0o600
/// - **非 Unix 原生平台**：不设置特殊权限
fn write_secret_file(path: &Path, value: &str) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, value);
        anyhow::bail!("Writing secret file is not supported in WASM");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // 创建带随机 UUID 的临时文件，避免并发冲突
        let temp_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        let mut temp_file =
            fs::OpenOptions::new().create_new(true).write(true).open(&temp_path).with_context(
                || format!("Failed to create temporary OTP secret {}", temp_path.display()),
            )?;

        // Unix 系统：设置严格的文件权限（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp_file.set_permissions(fs::Permissions::from_mode(0o600)).with_context(|| {
                format!("Failed to set permissions on temporary OTP secret {}", temp_path.display())
            })?;
        }

        // 写入数据并同步到磁盘
        temp_file.write_all(value.as_bytes()).with_context(|| {
            format!("Failed to write temporary OTP secret {}", temp_path.display())
        })?;
        temp_file.sync_all().with_context(|| {
            format!("Failed to fsync temporary OTP secret {}", temp_path.display())
        })?;
        // 显式关闭文件句柄，确保数据完全写入
        drop(temp_file);

        // 原子性地替换目标文件
        fs::rename(&temp_path, path).with_context(|| {
            format!("Failed to atomically replace OTP secret file {}", path.display())
        })?;

        // 再次确保最终文件的权限正确
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).with_context(|| {
                format!("Failed to enforce permissions on OTP secret file {}", path.display())
            })?;
        }
        Ok(())
    }
}

/// 获取当前 Unix 时间戳（秒）
///
/// # 返回值
///
/// 返回自 1970-01-01 00:00:00 UTC 以来的秒数。
/// 如果系统时间早于 Unix 纪元（极不可能），返回 0。
fn unix_timestamp_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0)
}

/// 计算 TOTP 验证码
///
/// 基于 RFC 6238 和 RFC 4226 实现 TOTP 算法：
/// 1. 使用 HMAC-SHA1 计算密钥和计数器的哈希值
/// 2. 使用动态截取算法提取 6 位数字
///
/// # 参数
///
/// - `secret`：原始密钥字节
/// - `counter`：时间窗口计数器（时间戳 / 时间步长）
///
/// # 返回值
///
/// 返回 6 位数字验证码字符串，不足 6 位前面补零
///
/// # 平台差异
///
/// - **WASM**：返回固定值 "000000"（占位实现）
/// - **原生平台**：使用 `ring` crate 进行 HMAC-SHA1 计算
///
/// # 算法详解
///
/// ```text
/// 1. HMAC-SHA1(secret, counter) → 20 字节哈希
/// 2. 取哈希最后一个字节的低 4 位作为偏移量 (0-15)
/// 3. 从偏移量开始取 4 字节，转换为 31 位整数（首字节最高位清零）
/// 4. 对 10^6 取模得到 6 位验证码
/// ```
fn compute_totp_code(secret: &[u8], counter: u64) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (secret, counter);
        "000000".to_string() // Dummy TOTP for WASM since we can't use ring::hmac
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 使用 HMAC-SHA1 计算哈希
        let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
        let counter_bytes = counter.to_be_bytes();
        let digest = hmac::sign(&key, &counter_bytes);
        let hash = digest.as_ref();

        // 动态截取：取最后一个字节的低 4 位作为偏移量
        let offset = (hash[19] & 0x0f) as usize;

        // 从偏移位置提取 4 字节，构造 31 位整数
        // & 0x7f 确保最高位为 0，避免符号位问题
        let binary = ((u32::from(hash[offset]) & 0x7f) << 24)
            | (u32::from(hash[offset + 1]) << 16)
            | (u32::from(hash[offset + 2]) << 8)
            | u32::from(hash[offset + 3]);

        // 取模得到指定位数的验证码
        let code = binary % 10_u32.pow(OTP_DIGITS);

        // 格式化为固定位数字符串，前面补零
        format!("{code:0>6}")
    }
}

/// 将字节数组编码为 Base32 字符串
///
/// Base32 使用 32 个字符（A-Z 和 2-7）表示数据，
/// 每 5 位一组转换为一个字符。
///
/// # 参数
///
/// - `input`：要编码的字节数组
///
/// # 返回值
///
/// 返回 Base32 编码的字符串（不包含填充字符 '='）
///
/// # 示例
///
/// ```
/// let secret = b"Hello";
/// let encoded = encode_base32_secret(secret);
/// assert_eq!(encoded, "JBSWY3DP");
/// ```
///
/// # 编码过程
///
/// ```text
/// 输入字节：  H        e        l        l        o
/// ASCII:    72       101      108      108      111
/// 二进制:  01001000 01100101 01101100 01101100 01101111
///
/// 按 5 位分组：
/// 01001 00001 10010 10110 11000 11011 00011 01111
/// ↓     ↓     ↓     ↓     ↓     ↓     ↓     ↓
/// J     B     S     W     Y     3     D     P
/// ```
fn encode_base32_secret(input: &[u8]) -> String {
    /// Base32 字母表
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

    if input.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut buffer = 0u16; // 16 位缓冲区，足够容纳 2 个字节（16 位）
    let mut bits_left = 0u8; // 缓冲区中剩余的有效位数

    for byte in input {
        // 将新字节加入缓冲区
        buffer = (buffer << 8) | u16::from(*byte);
        bits_left += 8;

        // 当缓冲区中有足够的位时，提取 5 位组
        while bits_left >= 5 {
            // 取最高 5 位
            let index = ((buffer >> (bits_left - 5)) & 0x1f) as usize;
            result.push(ALPHABET[index] as char);
            bits_left -= 5;
        }
    }

    // 处理剩余不足 5 位的部分（左对齐补零）
    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1f) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

/// 将 Base32 字符串解码为字节数组
///
/// 支持标准 Base32 格式，会自动忽略空格、制表符、换行符和连字符等分隔符。
///
/// # 参数
///
/// - `raw`：Base32 编码的字符串（可以包含分隔符和填充字符）
///
/// # 返回值
///
/// - `Ok(Vec<u8>)`：解码成功，返回原始字节
/// - `Err(...)`：解码失败（空字符串、无效字符等）
///
/// # 错误
///
/// - 输入为空或清理后为空
/// - 包含非 Base32 字符（除 A-Z 和 2-7 以外的字符）
/// - 解码后没有产生任何字节
///
/// # 示例
///
/// ```
/// let decoded = decode_base32_secret("JBSWY3DP")?;
/// assert_eq!(decoded, b"Hello");
///
/// // 支持带分隔符的输入
/// let decoded = decode_base32_secret("JBSW-Y3DP")?;
/// assert_eq!(decoded, b"Hello");
/// ```
fn decode_base32_secret(raw: &str) -> Result<Vec<u8>> {
    /// 解码单个 Base32 字符
    ///
    /// - 'A'-'Z' → 0-25
    /// - '2'-'7' → 26-31
    fn decode_char(ch: char) -> Option<u8> {
        match ch {
            'A'..='Z' => Some((ch as u8) - b'A'),
            '2'..='7' => Some((ch as u8) - b'2' + 26),
            _ => None,
        }
    }

    // 清理输入：移除分隔符和填充字符，转换为大写
    let mut cleaned = raw
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '\t' | '\n' | '\r' | '-'))
        .collect::<String>()
        .to_ascii_uppercase();

    // 移除末尾的填充字符 '='
    while cleaned.ends_with('=') {
        cleaned.pop();
    }

    if cleaned.is_empty() {
        anyhow::bail!("OTP secret is empty");
    }

    let mut output = Vec::new();
    let mut buffer = 0u32; // 32 位缓冲区，足够容纳多个 5 位组
    let mut bits_left = 0u8;

    for ch in cleaned.chars() {
        // 解码字符为 5 位值
        let value = decode_char(ch)
            .with_context(|| format!("OTP secret contains invalid base32 character '{ch}'"))?;

        // 将 5 位值加入缓冲区
        buffer = (buffer << 5) | u32::from(value);
        bits_left += 5;

        // 当缓冲区中有足够的位时，提取完整的字节
        if bits_left >= 8 {
            let byte = ((buffer >> (bits_left - 8)) & 0xff) as u8;
            output.push(byte);
            bits_left -= 8;
        }
    }

    if output.is_empty() {
        anyhow::bail!("OTP secret did not decode to any bytes");
    }
    Ok(output)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
