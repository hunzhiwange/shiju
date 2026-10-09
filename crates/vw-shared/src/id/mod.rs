//! ID 生成与管理模块
//!
//! 本模块提供结构化、可排序的唯一标识符生成功能，支持多种实体类型的 ID 生成。
//!
//! # 设计特性
//!
//! - **前缀标识**: 每种实体类型都有唯一的前缀（如 session、message、user 等）
//! - **时间戳编码**: ID 包含时间信息，可从中提取生成时间
//! - **可排序性**: 支持升序和降序生成，便于按时间排序
//! - **唯一性保证**: 通过时间戳 + 计数器 + 随机数确保全局唯一
//!
//! # ID 格式
//!
//! 生成的 ID 格式为: `{prefix}_{timestamp_hex}{random_base62}`
//!
//! - `prefix`: 3-4 个字符的实体类型前缀
//! - `timestamp_hex`: 12 个字符的十六进制时间戳
//! - `random_base62`: 14 个字符的 Base62 随机字符串
//! - 总长度: 26 个字符（不含前缀）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use vw_shared::id::{Prefix, ascending, descending, schema, timestamp};
//!
//! // 生成升序 ID
//! let session_id = ascending(Prefix::Session, None)?;
//! // 输出示例: "ses_0000001234567890abc123def456"
//!
//! // 生成降序 ID（用于倒序排序）
//! let msg_id = descending(Prefix::Message, None)?;
//!
//! // 验证 ID 前缀
//! assert!(schema(Prefix::Session, &session_id));
//!
//! // 提取时间戳
//! let ts = timestamp(&session_id);
//! ```

#![allow(unexpected_cfgs)]

use once_cell::sync::Lazy;
use std::sync::Mutex;
#[cfg(any(test, coverage))]
use std::sync::atomic::{AtomicBool, Ordering};

/// ID 前缀枚举
///
/// 定义系统中不同实体类型的标识符前缀，用于快速识别 ID 所属的实体类型。
///
/// # 前缀映射
///
/// | 变体 | 前缀字符串 | 用途 |
/// |------|-----------|------|
/// | Session | "ses" | 会话标识 |
/// | Message | "msg" | 消息标识 |
/// | Permission | "per" | 权限标识 |
/// | Question | "que" | 问题标识 |
/// | User | "usr" | 用户标识 |
/// | Part | "prt" | 部分/片段标识 |
/// | Pty | "pty" | 伪终端标识 |
/// | Tool | "tool" | 工具标识 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    /// 会话 ID 前缀
    Session,
    /// 消息 ID 前缀
    Message,
    /// 权限 ID 前缀
    Permission,
    /// 问题 ID 前缀
    Question,
    /// 用户 ID 前缀
    User,
    /// 部分/片段 ID 前缀
    Part,
    /// 伪终端 ID 前缀
    Pty,
    /// 工具 ID 前缀
    Tool,
}

impl Prefix {
    /// 将前缀枚举转换为字符串切片
    ///
    /// # 返回值
    ///
    /// 返回对应前缀的静态字符串引用，生命周期与程序相同。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// assert_eq!(Prefix::Session.as_str(), "ses");
    /// assert_eq!(Prefix::Message.as_str(), "msg");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Prefix::Session => "ses",
            Prefix::Message => "msg",
            Prefix::Permission => "per",
            Prefix::Question => "que",
            Prefix::User => "usr",
            Prefix::Part => "prt",
            Prefix::Pty => "pty",
            Prefix::Tool => "tool",
        }
    }
}

/// ID 生成和处理错误
///
/// 定义 ID 相关操作可能产生的错误类型。
#[derive(Debug)]
pub enum Error {
    /// ID 前缀不匹配错误
    ///
    /// 当提供的 ID 与预期的前缀不符时返回此错误。
    PrefixMismatch {
        /// 不匹配的 ID 字符串
        id: String,
        /// 期望的前缀字符串
        expected_prefix: &'static str,
    },
    /// 随机数生成错误
    ///
    /// 当底层随机数生成器失败时返回此错误。
    Random(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PrefixMismatch { id, expected_prefix } => {
                write!(f, "ID {} does not start with {}", id, expected_prefix)
            }
            Error::Random(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// ID 的随机部分长度（不含前缀和时间戳部分）
///
/// 总 ID 长度 = 前缀长度 + 1（下划线）+ 12（时间戳 hex）+ 14（随机部分）= 26 字符
const LENGTH: usize = 26;

/// ID 生成器的内部状态
///
/// 用于在同一毫秒内生成多个 ID 时保证唯一性和顺序性。
#[derive(Debug, Default, Clone, Copy)]
struct State {
    /// 上次生成 ID 时的时间戳（毫秒）
    last_timestamp_ms: u64,
    /// 当前毫秒内的计数器，用于处理同一毫秒内的多次 ID 生成
    counter: u64,
}

/// 全局 ID 生成状态
///
/// 使用 `Lazy` 和 `Mutex` 实现线程安全的全局状态管理。
/// 确保在多线程环境下 ID 的唯一性和正确的时间戳排序。
static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

#[cfg(any(test, coverage))]
static FORCE_RANDOM_ERROR: AtomicBool = AtomicBool::new(false);

/// 获取当前时间的毫秒级时间戳
///
/// # 返回值
///
/// 返回自 Unix 纪元（1970-01-01 00:00:00 UTC）以来的毫秒数。
/// 如果系统时间获取失败或转换溢出，返回 `u64::MAX`。
///
/// # 实现细节
///
/// - 使用 `SystemTime::now()` 获取当前时间
/// - 计算与 Unix 纪元的时长差
/// - 将时长转换为毫秒
fn now_ms() -> u64 {
    crate::time::now_ms()
}

/// 生成指定长度的 Base62 随机字符串
///
/// Base62 字符集包含: 0-9, A-Z, a-z（共 62 个字符）
///
/// # 参数
///
/// - `length`: 要生成的随机字符串长度
///
/// # 返回值
///
/// - `Ok(String)`: 成功生成的 Base62 随机字符串
/// - `Err(Error::Random)`: 随机数生成失败
///
/// # 实现细节
///
/// 1. 使用 `getrandom` 库获取密码学安全的随机字节
/// 2. 将每个随机字节映射到 Base62 字符集中的一个字符
/// 3. 通过取模运算确保字符在有效范围内
fn random_base62(length: usize) -> Result<String, Error> {
    const CHARS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut bytes = vec![0u8; length];
    #[cfg(any(test, coverage))]
    if FORCE_RANDOM_ERROR.swap(false, Ordering::SeqCst) {
        return Err(Error::Random("forced random error".to_string()));
    }
    fill_random(&mut bytes)?;

    let mut out = String::with_capacity(length);
    for b in bytes {
        out.push(CHARS[(b as usize) % 62] as char);
    }
    Ok(out)
}

#[cfg(not(any(test, coverage)))]
fn fill_random(bytes: &mut [u8]) -> Result<(), Error> {
    getrandom::fill(bytes).map_err(|e| Error::Random(e.to_string()))
}

#[cfg(any(test, coverage))]
fn fill_random(bytes: &mut [u8]) -> Result<(), Error> {
    getrandom::fill(bytes).expect("test randomness should be available");
    Ok(())
}

/// 验证 ID 是否符合指定的前缀模式
///
/// # 参数
///
/// - `prefix`: 期望的前缀类型
/// - `value`: 要验证的 ID 字符串
///
/// # 返回值
///
/// 如果 ID 以指定前缀开头返回 `true`，否则返回 `false`。
///
/// # 示例
///
/// ```rust,ignore
/// let id = "ses_0000001234567890abc123def456";
/// assert!(schema(Prefix::Session, id));
/// assert!(!schema(Prefix::Message, id));
/// ```
pub fn schema(prefix: Prefix, value: &str) -> bool {
    value.starts_with(prefix.as_str())
}

/// 生成升序排列的 ID
///
/// 生成的 ID 按时间戳从小到大排序，适合需要按时间正序排列的场景。
///
/// # 参数
///
/// - `prefix`: ID 的前缀类型
/// - `given`: 可选的已有 ID。如果提供，会验证其前缀是否匹配，匹配则直接返回
///
/// # 返回值
///
/// - `Ok(String)`: 成功生成的升序 ID
/// - `Err(Error::PrefixMismatch)`: 提供的 ID 前缀不匹配
/// - `Err(Error::Random)`: 随机数生成失败
///
/// # 示例
///
/// ```rust,ignore
/// // 生成新的升序 ID
/// let id1 = ascending(Prefix::Session, None)?;
/// let id2 = ascending(Prefix::Session, None)?;
/// assert!(id1 < id2); // 后生成的 ID 字符串更大
///
/// // 使用已有 ID（前缀必须匹配）
/// let existing = "ses_0000001234567890abc123def456";
/// let id = ascending(Prefix::Session, Some(existing))?;
/// assert_eq!(id, existing);
/// ```
pub fn ascending(prefix: Prefix, given: Option<&str>) -> Result<String, Error> {
    generate_id(prefix, false, given)
}

/// 生成降序排列的 ID
///
/// 生成的 ID 按时间戳从大到小排序，适合需要按时间倒序排列的场景。
/// 通过对时间戳进行按位取反实现降序效果。
///
/// # 参数
///
/// - `prefix`: ID 的前缀类型
/// - `given`: 可选的已有 ID。如果提供，会验证其前缀是否匹配，匹配则直接返回
///
/// # 返回值
///
/// - `Ok(String)`: 成功生成的降序 ID
/// - `Err(Error::PrefixMismatch)`: 提供的 ID 前缀不匹配
/// - `Err(Error::Random)`: 随机数生成失败
///
/// # 示例
///
/// ```rust,ignore
/// // 生成新的降序 ID
/// let id1 = descending(Prefix::Session, None)?;
/// let id2 = descending(Prefix::Session, None)?;
/// assert!(id1 > id2); // 后生成的 ID 字符串更小（时间戳越大，取反后越小）
///
/// // 使用已有 ID（前缀必须匹配）
/// let existing = "ses_0000001234567890abc123def456";
/// let id = descending(Prefix::Session, Some(existing))?;
/// assert_eq!(id, existing);
/// ```
pub fn descending(prefix: Prefix, given: Option<&str>) -> Result<String, Error> {
    generate_id(prefix, true, given)
}

/// ID 生成的核心逻辑
///
/// 根据参数生成或验证 ID。
///
/// # 参数
///
/// - `prefix`: ID 的前缀类型
/// - `descending`: 是否生成降序 ID（true 为降序，false 为升序）
/// - `given`: 可选的已有 ID
///
/// # 返回值
///
/// - `Ok(String)`: 生成的或验证通过的 ID
/// - `Err(Error::PrefixMismatch)`: 提供的 ID 前缀不匹配
/// - `Err(Error::Random)`: 随机数生成失败
///
/// # 处理逻辑
///
/// 1. 如果未提供 `given`，调用 `create` 生成新 ID
/// 2. 如果提供了 `given`，验证其前缀是否匹配：
///    - 匹配：直接返回该 ID
///    - 不匹配：返回 `PrefixMismatch` 错误
fn generate_id(prefix: Prefix, descending: bool, given: Option<&str>) -> Result<String, Error> {
    let Some(given) = given else {
        return create(prefix, descending, None);
    };
    let expected = prefix.as_str();
    if !given.starts_with(expected) {
        return Err(Error::PrefixMismatch { id: given.to_string(), expected_prefix: expected });
    }
    Ok(given.to_string())
}

/// 创建新的唯一标识符
///
/// 这是 ID 生成的核心函数，实现了时间戳编码、计数器管理和随机数生成。
///
/// # 参数
///
/// - `prefix`: ID 的前缀类型，用于标识实体类型
/// - `descending`: 是否生成降序 ID
///   - `false`: 升序，时间戳越大 ID 越大（适合正序排序）
///   - `true`: 降序，时间戳越大 ID 越小（适合倒序排序）
/// - `timestamp_ms`: 可选的自定义时间戳（毫秒）。如不提供，使用当前系统时间
///
/// # 返回值
///
/// - `Ok(String)`: 成功生成的 ID，格式为 `{prefix}_{timestamp_hex}{random}`
/// - `Err(Error::Random)`: 随机数生成失败
///
/// # 实现细节
///
/// ## 时间戳编码
///
/// 时间戳编码采用了以下算法确保在同一毫秒内生成的 ID 有序且唯一：
///
/// 1. **时间戳 + 计数器组合**:
///    - 将时间戳左移 12 位（乘以 0x1000）
///    - 加上同一毫秒内的计数器值
///    - 这样可以在同一毫秒内生成最多 4096 个有序 ID
///
/// 2. **计数器管理**:
///    - 如果时间戳改变，重置计数器为 0
///    - 如果时间戳相同，计数器递增（使用 saturating_add 防止溢出）
///
/// 3. **降序处理**:
///    - 对组合值进行按位取反（`!`），使时间戳大的 ID 字符串更小
///
/// ## ID 结构
///
/// 最终 ID 格式: `{prefix}_{6字节时间戳hex}{14字符随机base62}`
///
/// - 时间戳部分: 6 字节（12 个 hex 字符）
/// - 随机部分: 14 个 Base62 字符
///
/// # 并发安全
///
/// 使用全局 `Mutex<State>` 确保多线程环境下的唯一性和正确性。
/// 即使在高并发场景下，也能保证 ID 的唯一性和有序性。
///
/// # 示例
///
/// ```rust,ignore
/// // 生成基于当前时间的升序 ID
/// let id = create(Prefix::Session, false, None)?;
///
/// // 生成基于特定时间戳的降序 ID
/// let custom_ts = 1234567890000;
/// let id = create(Prefix::Message, true, Some(custom_ts))?;
/// ```
pub fn create(
    prefix: Prefix,
    descending: bool,
    timestamp_ms: Option<u64>,
) -> Result<String, Error> {
    let current_timestamp = timestamp_ms.unwrap_or_else(now_ms);

    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());

    if current_timestamp != state.last_timestamp_ms {
        state.last_timestamp_ms = current_timestamp;
        state.counter = 0;
    }
    state.counter = state.counter.saturating_add(1);

    let mut now = current_timestamp.saturating_mul(0x1000).saturating_add(state.counter);

    if descending {
        now = !now;
    }

    let mut time_hex = String::with_capacity(12);
    for i in 0..6 {
        let shift = 40 - 8 * i;
        let byte = ((now >> shift) & 0xff) as u8;
        time_hex.push_str(&format!("{:02x}", byte));
    }

    let rand = random_base62(LENGTH - 12)?;
    Ok(format!("{}_{}{}", prefix.as_str(), time_hex, rand))
}

/// 从 ID 中提取时间戳
///
/// 解析 ID 字符串，提取其中编码的毫秒级时间戳。
///
/// # 参数
///
/// - `id`: 要解析的 ID 字符串，格式应为 `{prefix}_{timestamp_hex}{random}`
///
/// # 返回值
///
/// - `Some(u64)`: 成功提取的毫秒级时间戳
/// - `None`: ID 格式无效或时间戳部分解析失败
///
/// # 实现细节
///
/// 1. 提取前缀部分（第一个下划线之前）
/// 2. 定位时间戳十六进制部分（前缀后 12 个字符）
/// 3. 将十六进制字符串转换为 u64
/// 4. 除以 0x1000 得到原始时间戳（去掉计数器部分）
///
/// # 注意事项
///
/// - 对于降序 ID，提取的时间戳是取反后的值，需要再次取反才能得到原始时间戳
/// - 此函数不区分升序和降序 ID，仅提取编码的数值部分
///
/// # 示例
///
/// ```rust,ignore
/// // 生成升序 ID 并提取时间戳
/// let id = ascending(Prefix::Session, None)?;
/// if let Some(ts) = timestamp(&id) {
///     println!("ID 生成时间: {} ms", ts);
/// }
///
/// // 从已知 ID 提取时间戳
/// let id = "ses_0000001234567890abc123def456";
/// let ts = timestamp(id);
/// ```
pub fn timestamp(id: &str) -> Option<u64> {
    let (prefix, _) = id.split_once('_')?;

    let start = prefix.len() + 1;
    let end = start + 12;

    let hex = id.get(start..end)?;
    let encoded = u64::from_str_radix(hex, 16).ok()?;

    Some(encoded / 0x1000)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
