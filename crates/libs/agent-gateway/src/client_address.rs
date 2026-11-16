//! Client address parsing shared by pairing and gateway ingress.
use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

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
