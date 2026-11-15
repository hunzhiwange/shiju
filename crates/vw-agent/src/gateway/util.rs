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

use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

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
/// use crate::app::agent::gateway::util::webhook_memory_key;
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

/// 解析客户端 IP 地址字符串
///
/// 尝试从多种格式的字符串中提取 IP 地址，支持：
/// - 纯 IP 地址（IPv4 或 IPv6）
/// - 带端口的 Socket 地址（IP:Port）
/// - 带方括号的 IPv6 地址
/// - 带引号的地址字符串
///
/// # 参数
///
/// - `value`: IP 地址字符串，可能包含空格、引号、方括号等字符
///
/// # 返回值
///
/// - `Some(IpAddr)`: 成功解析出的 IP 地址
/// - `None`: 解析失败或输入为空
///
/// # 解析逻辑
///
/// 1. 去除首尾空格和引号
/// 2. 尝试直接解析为 IP 地址
/// 3. 尝试解析为 Socket 地址（IP:Port）
/// 4. 去除方括号后再次尝试解析（处理 `[IPv6]` 格式）
///
/// # 示例
///
/// ```rust
/// use std::net::{IpAddr, Ipv4Addr};
///
/// let ip1 = parse_client_ip("192.168.1.1");
/// let ip2 = parse_client_ip("192.168.1.1:8080");
/// let ip3 = parse_client_ip("  \"192.168.1.1\"  ");
/// assert_eq!(ip1, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
/// ```
pub fn parse_client_ip(value: &str) -> Option<IpAddr> {
    // 第一步：去除首尾空格、引号，再次去除空格
    let value = value.trim().trim_matches('"').trim();
    if value.is_empty() {
        return None;
    }

    // 第二步：尝试直接解析为 IP 地址（IPv4 或 IPv6）
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    // 第三步：尝试解析为 Socket 地址（IP:Port 格式）
    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    // 第四步：去除方括号后再次尝试（处理 [IPv6] 或 [IPv6]:Port 格式）
    let value = value.trim_matches(['[', ']']);
    value.parse::<IpAddr>().ok()
}

/// 从 HTTP 转发头部中提取客户端 IP 地址
///
/// 按照标准的代理转发链顺序，尝试从以下 HTTP 头部中提取客户端真实 IP：
/// 1. `X-Forwarded-For`: 逗号分隔的 IP 列表，第一个通常是原始客户端 IP
/// 2. `X-Real-IP`: 单个 IP 地址
///
/// # 参数
///
/// - `headers`: HTTP 请求头映射表
///
/// # 返回值
///
/// - `Some(IpAddr)`: 成功提取的客户端 IP
/// - `None`: 未找到有效的转发头部
///
/// # 注意事项
///
/// - 仅在信任反向代理的情况下使用
/// - `X-Forwarded-For` 中的 IP 可能被伪造，需要配合代理配置验证
/// - 如果有多个代理层级，X-Forwarded-For 会包含多个 IP（从左到右依次为客户端、代理1、代理2...）
///
/// # 示例
///
/// ```rust
/// use axum::http::HeaderMap;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("X-Forwarded-For", "203.0.113.1, 70.41.3.18".parse().unwrap());
/// let ip = forwarded_client_ip(&headers);
/// // ip 会是第一个有效的 IP 地址
/// ```
pub fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    // 优先检查 X-Forwarded-For 头部（包含逗号分隔的 IP 列表）
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        // 遍历 IP 列表，返回第一个有效的 IP 地址
        for candidate in xff.split(',') {
            if let Some(ip) = parse_client_ip(candidate) {
                return Some(ip);
            }
        }
    }

    // 如果 X-Forwarded-For 不存在或无有效 IP，则尝试 X-Real-IP
    headers.get("X-Real-IP").and_then(|v| v.to_str().ok()).and_then(parse_client_ip)
}

/// 从请求中获取客户端标识键
///
/// 根据配置策略，从请求的转发头部或对等地址中提取客户端标识（通常是 IP 地址）。
/// 该标识可用于限流、访问控制、日志记录等场景。
///
/// # 参数
///
/// - `peer_addr`: 对等地址（直接连接的客户端地址），可能为 None
/// - `headers`: HTTP 请求头映射表
/// - `trust_forwarded_headers`: 是否信任转发头部（如 X-Forwarded-For）
///
/// # 返回值
///
/// 返回客户端标识字符串（IP 地址或 "unknown"）
///
/// # 逻辑说明
///
/// 1. 如果配置为信任转发头部：
///    - 优先从 `X-Forwarded-For` 或 `X-Real-IP` 提取 IP
///    - 如果提取失败，回退到对等地址
/// 2. 如果不信任转发头部：
///    - 直接使用对等地址
///    - 如果对等地址也不可用，返回 "unknown"
///
/// # 安全性说明
///
/// - 仅在网关后方有可信的反向代理时，才应启用 `trust_forwarded_headers`
/// - 直接暴露在公网的服务应禁用转发头部信任，防止 IP 伪造攻击
///
/// # 示例
///
/// ```rust
/// use std::net::{SocketAddr, IpAddr, Ipv4Addr};
/// use axum::http::HeaderMap;
///
/// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
/// let headers = HeaderMap::new();
///
/// // 不信任转发头部
/// let key = client_key_from_request(Some(addr), &headers, false);
/// assert_eq!(key, "127.0.0.1");
/// ```
pub fn client_key_from_request(
    peer_addr: Option<SocketAddr>,
    headers: &HeaderMap,
    trust_forwarded_headers: bool,
) -> String {
    // 如果配置为信任转发头部，优先从头部提取客户端 IP
    if trust_forwarded_headers {
        if let Some(ip) = forwarded_client_ip(headers) {
            return ip.to_string();
        }
    }

    // 回退到对等地址，如果不可用则返回 "unknown"
    peer_addr.map(|addr| addr.ip().to_string()).unwrap_or_else(|| "unknown".to_string())
}

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
