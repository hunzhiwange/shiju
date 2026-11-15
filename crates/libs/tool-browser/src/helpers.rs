//! 浏览器工具辅助函数模块
//!
//! 本模块提供浏览器自动化相关的辅助函数，用于支持浏览器工具的核心功能。
//! 主要包括以下功能：
//!
//! - 错误恢复判断：识别可恢复的 WebDriver 错误
//! - 域名处理：域名规范化和主机提取
//! - 网络连通性检测：检查端点是否可达
//! - 安全性检查：检测私有/本地主机并验证访问权限

use std::net::ToSocketAddrs;
use std::time::Duration;

/// 判断错误是否为可恢复的 Rust 原生错误
///
/// 检查给定的错误是否属于浏览器自动化过程中可能出现的暂时性错误，
/// 这类错误通常可以通过重试或重新建立会话来恢复。
///
/// # 参数
///
/// - `err`: 要检查的错误引用
///
/// # 返回值
///
/// 如果错误属于以下可恢复类型，返回 `true`：
/// - 无效的会话 ID（会话已过期或被销毁）
/// - 窗口不存在（窗口已关闭）
/// - 会话创建失败（临时资源问题）
/// - 连接被重置（网络波动）
/// - 管道破裂（连接中断）
/// - WebDriver 超时相关错误
///
/// # 示例
///
/// ```ignore
/// use anyhow::anyhow;
///
/// let err = anyhow!("invalid session id: abc123");
/// assert!(is_recoverable_rust_native_error(&err));
///
/// let err = anyhow!("unknown error");
/// assert!(!is_recoverable_rust_native_error(&err));
/// ```
pub fn is_recoverable_rust_native_error(err: &anyhow::Error) -> bool {
    let message = format!("{err:#}").to_ascii_lowercase();

    // 检查常见的可恢复错误模式
    if message.contains("invalid session id")
        || message.contains("no such window")
        || message.contains("session not created")
        || message.contains("connection reset")
        || message.contains("broken pipe")
    {
        return true;
    }

    // WebDriver 特定的超时错误
    message.contains("webdriver") && (message.contains("timed out") || message.contains("timeout"))
}

/// 规范化域名列表
///
/// 对输入的域名列表进行清理和标准化处理，
/// 包括去除空白字符、转换为小写，并过滤掉空字符串。
///
/// # 参数
///
/// - `domains`: 原始域名列表
///
/// # 返回值
///
/// 返回经过规范化处理的域名列表，其中：
/// - 每个域名的前后空白已被去除
/// - 所有域名已转换为小写
/// - 空字符串已被过滤掉
///
/// # 示例
///
/// ```ignore
/// let domains = vec!["  Example.COM  ".to_string(), "  ".to_string(), "Localhost".to_string()];
/// let normalized = normalize_domains(domains);
/// assert_eq!(normalized, vec!["example.com", "localhost"]);
/// ```
pub fn normalize_domains(domains: Vec<String>) -> Vec<String> {
    domains.into_iter().map(|d| d.trim().to_lowercase()).filter(|d| !d.is_empty()).collect()
}

/// 检查端点是否可达
///
/// 尝试建立 TCP 连接以验证给定的 URL 端点是否可以在指定超时时间内访问。
///
/// # 参数
///
/// - `endpoint`: 要检查的 URL 端点
/// - `timeout`: 连接超时时间
///
/// # 返回值
///
/// - `true`: 如果能够在超时时间内成功建立 TCP 连接
/// - `false`: 如果 URL 无效、无法解析主机、或连接超时/失败
///
/// # 示例
///
/// ```ignore
/// use std::time::Duration;
///
/// let url = reqwest::Url::parse("http://localhost:4444").unwrap();
/// let reachable = endpoint_reachable(&url, Duration::from_secs(5));
/// ```
pub fn endpoint_reachable(endpoint: &reqwest::Url, timeout: Duration) -> bool {
    // 提取主机名，无效则直接返回 false
    let host = match endpoint.host_str() {
        Some(host) if !host.is_empty() => host,
        _ => return false,
    };

    // 获取端口（使用协议默认端口如果未指定）
    let port = match endpoint.port_or_known_default() {
        Some(port) => port,
        None => return false,
    };

    // 解析主机名到 socket 地址
    let mut addrs = match (host, port).to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => return false,
    };

    // 获取第一个解析结果
    let addr = match addrs.next() {
        Some(addr) => addr,
        None => return false,
    };

    // 尝试在超时时间内建立 TCP 连接
    std::net::TcpStream::connect_timeout(&addr, timeout).is_ok()
}

/// 从 URL 字符串中提取主机名
///
/// 不依赖 url crate 的轻量级主机名提取实现，
/// 支持常见协议前缀和 IPv6 地址格式。
///
/// # 参数
///
/// - `url_str`: URL 字符串
///
/// # 返回值
///
/// - `Ok(String)`: 成功提取的主机名（已转换为小写）
/// - `Err`: 如果 URL 格式无效或无法提取主机名
///
/// # 支持的格式
///
/// - HTTP/HTTPS URL: `https://example.com/path`
/// - 本地文件 URL: `file:///path/to/file`
/// - IPv4 地址: `http://192.168.1.1:8080`
/// - IPv6 地址: `http://[::1]:8080`
/// - 无协议前缀: `example.com:443`
///
/// # 示例
///
/// ```ignore
/// assert_eq!(extract_host("https://Example.COM/path").unwrap(), "example.com");
/// assert_eq!(extract_host("http://[::1]:8080").unwrap(), "[::1]");
/// assert_eq!(extract_host("192.168.1.1:443").unwrap(), "192.168.1.1");
/// ```
pub fn extract_host(url_str: &str) -> anyhow::Result<String> {
    // 去除 URL 前后空白，不依赖 url crate 的简单提取
    let url = url_str.trim();

    // 移除协议前缀，支持 https、http 和 file
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("file://"))
        .unwrap_or(url);

    // 提取权限部分（主机和端口），处理路径分隔符
    // 例如：从 "example.com:443/path" 中提取 "example.com:443"
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);

    // 根据 IPv6 或 IPv4/主机名格式提取主机部分
    let host = if authority.starts_with('[') {
        // IPv6 地址：提取从开头到右方括号的部分（包含方括号）
        // 例如：从 "[::1]:8080" 中提取 "[::1]"
        authority.find(']').map_or(authority, |i| &authority[..=i])
    } else {
        // IPv4 或主机名：提取端口分隔符之前的部分
        // 例如：从 "example.com:443" 中提取 "example.com"
        authority.split(':').next().unwrap_or(authority)
    };

    // 验证主机名非空
    if host.is_empty() {
        anyhow::bail!("Invalid URL: no host");
    }

    // 返回小写形式的主机名以保证一致性
    Ok(host.to_lowercase())
}

/// 判断主机是否为私有/本地地址
///
/// 检查给定的主机名是否指向私有网络、本地主机或其他非全局可路由地址。
/// 用于安全策略中防止访问内网资源（SSRF 防护）。
///
/// # 参数
///
/// - `host`: 主机名或 IP 地址字符串
///
/// # 返回值
///
/// 如果主机属于以下任一类型，返回 `true`：
/// - `localhost` 或其子域名（如 `*.localhost`）
/// - `.local` TLD（mDNS 本地域名）
/// - 私有 IPv4 地址（10.0.0.0/8、172.16.0.0/12、192.168.0.0/16 等）
/// - 本地链路地址（169.254.0.0/16）
/// - 环回地址（127.0.0.0/8、::1）
/// - IPv6 唯一本地地址（fc00::/7）
/// - IPv6 本地链路地址（fe80::/10）
/// - IPv4 映射的 IPv6 地址（如果映射的 IPv4 是私有的）
///
/// # 示例
///
/// ```ignore
/// assert!(is_private_host("localhost"));
/// assert!(is_private_host("192.168.1.1"));
/// assert!(is_private_host("::1"));
/// assert!(is_private_host("[::1]"));
/// assert!(!is_private_host("example.com"));
/// ```
pub fn is_private_host(host: &str) -> bool {
    // 移除 IPv6 地址的方括号，例如将 "[::1]" 转换为 "::1"
    let bare = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);

    // 检查 localhost 及其子域名
    if bare == "localhost" || bare.ends_with(".localhost") {
        return true;
    }

    // 检查 .local TLD（mDNS 本地域名）
    if bare.rsplit('.').next().is_some_and(|label| label == "local") {
        return true;
    }

    // 尝试解析为 IP 地址以捕获所有可能的表示形式
    // 包括十进制、十六进制、八进制、IPv4 映射等格式
    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(v6),
        };
    }

    false
}

/// 判断 IPv4 地址是否为非全局可路由地址
///
/// 检查给定的 IPv4 地址是否属于私有、保留或其他非公网可路由地址范围。
/// 这些地址无法直接从互联网访问，仅用于内部网络或特殊用途。
///
/// # 参数
///
/// - `v4`: 要检查的 IPv4 地址
///
/// # 返回值
///
/// 如果地址属于以下任一非全局范围，返回 `true`：
/// - 环回地址（127.0.0.0/8）
/// - 私有地址（10.0.0.0/8、172.16.0.0/12、192.168.0.0/16）
/// - 本地链路地址（169.254.0.0/16）
/// - 未指定地址（0.0.0.0）
/// - 广播地址（255.255.255.255）
/// - 组播地址（224.0.0.0/4）
/// - 共享地址空间（100.64.0.0/10，用于运营商级 NAT）
/// - 保留地址（240.0.0.0/4）
/// - 文档地址（192.0.2.0/24、198.51.100.0/24、203.0.113.0/24）
/// - 基准测试地址（198.18.0.0/15）
fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let [a, b, _, _] = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_multicast()
        // 共享地址空间 (100.64.0.0/10) - 用于运营商级 NAT
        || (a == 100 && (64..=127).contains(&b))
        // 保留地址 (240.0.0.0/4)
        || a >= 240
        // 文档地址 (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24)
        || (a == 192 && b == 0)
        || (a == 198 && b == 51)
        || (a == 203 && b == 0)
        // 基准测试地址 (198.18.0.0/15)
        || (a == 198 && (18..=19).contains(&b))
}

/// 判断 IPv6 地址是否为非全局可路由地址
///
/// 检查给定的 IPv6 地址是否属于私有、保留或其他非公网可路由地址范围。
///
/// # 参数
///
/// - `v6`: 要检查的 IPv6 地址
///
/// # 返回值
///
/// 如果地址属于以下任一非全局范围，返回 `true`：
/// - 环回地址（::1）
/// - 未指定地址（::）
/// - 组播地址（ff00::/8）
/// - 唯一本地地址（fc00::/7）- IPv6 的 RFC 1918 等价物
/// - 本地链路地址（fe80::/10）
/// - IPv4 映射地址（::ffff:0:0/96）- 如果映射的 IPv4 是非全局的
fn is_non_global_v6(v6: std::net::Ipv6Addr) -> bool {
    let segs = v6.segments();
    v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        // 唯一本地地址 (fc00::/7) - IPv6 中相当于 RFC 1918 私有地址
        || (segs[0] & 0xfe00) == 0xfc00
        // 本地链路地址 (fe80::/10)
        || (segs[0] & 0xffc0) == 0xfe80
        // IPv4 映射地址 - 检查映射的 IPv4 是否为非全局地址
        || v6.to_ipv4_mapped().is_some_and(is_non_global_v4)
}

/// 检查主机是否匹配允许列表
///
/// 根据给定的模式列表验证主机名是否被授权访问。
/// 支持精确匹配、子域名匹配和通配符匹配。
///
/// # 参数
///
/// - `host`: 要检查的主机名
/// - `allowed`: 允许的模式列表
///
/// # 返回值
///
/// 如果主机匹配任一允许模式，返回 `true`；否则返回 `false`。
///
/// # 匹配规则
///
/// - `*`: 匹配所有主机（完全开放）
/// - `*.example.com`: 匹配 example.com 及其所有子域名
/// - `example.com`: 精确匹配 example.com，或匹配其子域名（如 www.example.com）
///
/// # 示例
///
/// ```ignore
/// let allowed = vec!["*.example.com".to_string(), "trusted.org".to_string()];
///
/// assert!(host_matches_allowlist("example.com", &allowed));
/// assert!(host_matches_allowlist("www.example.com", &allowed));
/// assert!(host_matches_allowlist("trusted.org", &allowed));
/// assert!(host_matches_allowlist("sub.trusted.org", &allowed));
/// assert!(!host_matches_allowlist("other.com", &allowed));
///
/// let open = vec!["*".to_string()];
/// assert!(host_matches_allowlist("any.host.com", &open));
/// ```
pub fn host_matches_allowlist(host: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|pattern| {
        // 通配符 "*" 匹配所有主机
        if pattern == "*" {
            return true;
        }
        if pattern.starts_with("*.") {
            // 通配符子域名匹配：*.example.com 匹配 example.com 及其所有子域名
            let suffix = &pattern[1..]; // ".example.com"
            host.ends_with(suffix) || host == &pattern[2..]
        } else {
            // 精确匹配或子域名匹配
            // example.com 匹配 example.com 和 www.example.com
            host == pattern || host.ends_with(&format!(".{pattern}"))
        }
    })
}
#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;
