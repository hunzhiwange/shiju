//! # 网关工具模块
//!
//! 本模块提供了网关层（Gateway）使用的通用工具函数集合，主要用于处理各种消息通道的
//! 内存键生成、客户端 IP 解析、Webhook 密钥哈希等功能。
//!
//! ## 主要功能
//!
//! - **内存键生成**：为不同消息通道（Webhook、WhatsApp、Linq、WATI、Nextcloud Talk、QQ）
//!   生成唯一的内存存储键，用于消息去重和状态追踪。
//! - **IP 地址解析**：从 HTTP 请求头中提取真实的客户端 IP 地址，支持代理场景下的
//!   `X-Forwarded-For` 和 `X-Real-IP` 头部解析。
//! - **安全哈希**：对 Webhook 密钥进行 SHA-256 哈希处理，避免明文存储敏感信息。
//! - **配置规范化**：提供配置参数的规范化处理函数。
//!
//! ## 使用场景
//!
//! 这些工具函数主要被网关模块中的各个通道处理器调用，用于：
//! - 消息处理流程中的唯一标识生成
//! - 客户端请求的来源识别和限流
//! - Webhook 回调的安全验证

// Address parsing is owned by agent-gateway.

/// 生成 Webhook 消息的唯一内存键
///
/// 使用 UUID v4 生成全局唯一标识符，用于标识每个 Webhook 请求。
/// 适用于需要为 Webhook 消息创建唯一存储键的场景。
///
/// # 返回值
///
/// 返回格式为 `"webhook_msg_{uuid}"` 的字符串，其中 `{uuid}` 是一个随机的 UUID v4。
///
/// # 示例
///
/// ```rust
/// use agent_gateway::util::webhook_memory_key;
///
/// let key = webhook_memory_key();
/// assert!(key.starts_with("webhook_msg_"));
/// ```
pub fn webhook_memory_key() -> String {
    format!("webhook_msg_{}", uuid::Uuid::new_v4())
}

/// 生成 WhatsApp 消息的唯一内存键
///
/// 基于消息的发送者和消息 ID 组合生成内存键，确保同一消息具有相同的键，
/// 用于消息去重和状态追踪。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者信息和消息 ID
///
/// # 返回值
///
/// 返回格式为 `"whatsapp_{sender}_{id}"` 的字符串
///
/// # 示例
///
/// ```rust
/// let key = whatsapp_memory_key(&channel_message);
/// // key 格式: "whatsapp_user123_msg456"
/// ```
pub fn whatsapp_memory_key(msg: &channel_types::ChannelMessage) -> String {
    format!("whatsapp_{}_{}", msg.sender, msg.id)
}

/// 生成 Linq 消息的唯一内存键
///
/// 基于消息的发送者和消息 ID 组合生成内存键，用于 Linq 通道的消息标识。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者信息和消息 ID
///
/// # 返回值
///
/// 返回格式为 `"linq_{sender}_{id}"` 的字符串
pub fn linq_memory_key(msg: &channel_types::ChannelMessage) -> String {
    format!("linq_{}_{}", msg.sender, msg.id)
}

/// 生成 WATI 消息的唯一内存键
///
/// 基于消息的发送者和消息 ID 组合生成内存键，用于 WATI（WhatsApp Team Inbox）通道的消息标识。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者信息和消息 ID
///
/// # 返回值
///
/// 返回格式为 `"wati_{sender}_{id}"` 的字符串
pub fn wati_memory_key(msg: &channel_types::ChannelMessage) -> String {
    format!("wati_{}_{}", msg.sender, msg.id)
}

/// 生成 Nextcloud Talk 消息的唯一内存键
///
/// 基于消息的发送者和消息 ID 组合生成内存键，用于 Nextcloud Talk 通道的消息标识。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者信息和消息 ID
///
/// # 返回值
///
/// 返回格式为 `"nextcloud_talk_{sender}_{id}"` 的字符串
pub fn nextcloud_talk_memory_key(
    msg: &channel_types::ChannelMessage,
) -> String {
    format!("nextcloud_talk_{}_{}", msg.sender, msg.id)
}

/// 生成 QQ 消息的唯一内存键
///
/// 基于消息的发送者和消息 ID 组合生成内存键，用于 QQ 通道的消息标识。
///
/// # 参数
///
/// - `msg`: 通道消息引用，包含发送者信息和消息 ID
///
/// # 返回值
///
/// 返回格式为 `"qq_{sender}_{id}"` 的字符串
pub fn qq_memory_key(msg: &channel_types::ChannelMessage) -> String {
    format!("qq_{}_{}", msg.sender, msg.id)
}

/// 对 Webhook 密钥进行 SHA-256 哈希处理
///
/// 使用 SHA-256 算法对输入的密钥字符串进行哈希，并返回十六进制编码的结果。
/// 这用于安全地存储和验证 Webhook 密钥，避免明文存储。
///
/// # 参数
///
/// - `value`: 需要哈希的密钥字符串
///
/// # 返回值
///
/// 返回 64 个字符的十六进制字符串（SHA-256 哈希值的十六进制表示）
///
/// # 安全性说明
///
/// - 使用 SHA-256 单向哈希，无法从哈希值反推原始密钥
/// - 相同的输入总是产生相同的输出（确定性）
/// - 适用于密钥验证场景，不适用于密码存储（建议使用 Argon2/bcrypt）
///
/// # 示例
///
/// ```rust
/// let secret = "my_webhook_secret";
/// let hashed = hash_webhook_secret(secret);
/// assert_eq!(hashed.len(), 64);
/// ```
pub fn hash_webhook_secret(value: &str) -> String {
    use sha2::{Digest, Sha256};

    // 使用 SHA-256 算法计算哈希值
    let digest = Sha256::digest(value.as_bytes());
    // 将二进制哈希值转换为十六进制字符串
    hex::encode(digest)
}

pub use crate::client_address::{parse_client_ip, forwarded_client_ip, client_key_from_request};

/// 规范化最大键数量配置
///
/// 对配置中的最大键数量参数进行规范化处理，确保返回有效的值。
/// 主要用于处理默认值或零值的情况。
///
/// # 参数
///
/// - `configured`: 用户配置的最大键数量
/// - `fallback`: 当配置值为 0 时的回退值
///
/// # 返回值
///
/// - 如果 `configured > 0`，返回 `configured`
/// - 如果 `configured == 0`，返回 `fallback.max(1)`（确保至少为 1）
///
/// # 示例
///
/// ```rust
/// // 使用用户配置值
/// assert_eq!(normalize_max_keys(100, 50), 100);
///
/// // 使用回退值
/// assert_eq!(normalize_max_keys(0, 50), 50);
///
/// // 确保最小值为 1
/// assert_eq!(normalize_max_keys(0, 0), 1);
/// ```
pub fn normalize_max_keys(configured: usize, fallback: usize) -> usize {
    if configured == 0 { fallback.max(1) } else { configured }
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod util_tests;
