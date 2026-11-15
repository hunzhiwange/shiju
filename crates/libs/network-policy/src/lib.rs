//! URL 验证工具模块
//!
//! 本模块提供统一的 URL 验证逻辑，用于在代理运行时中对外部访问进行安全控制。
//!
//! # 主要功能
//!
//! - **域名访问控制**：通过白名单和黑名单机制限制可访问的域名范围
//! - **协议限制**：支持仅 HTTPS 或 HTTP/HTTPS 混合的协议策略
//! - **本地地址防护**：自动阻止对本地回环地址、私有 IP 地址的访问
//! - **IPv6 地址过滤**：出于安全考虑，当前不支持 IPv6 地址
//!
//! # 安全设计
//!
//! 该模块的设计目标是防止 SSRF（服务端请求伪造）攻击，确保代理只能访问
//! 预先授权的外部资源。所有验证都是严格的"默认拒绝"策略。
//!
//! # 使用示例
//!
//! ```ignore
//! use network_policy::{validate_url, DomainPolicy, UrlSchemePolicy};
//!
//! let allowed = vec!["api.example.com".to_string(), "*.cdn.example.com".to_string()];
//! let blocked = vec!["malicious.example.com".to_string()];
//!
//! let policy = DomainPolicy {
//!     allowed_domains: &allowed,
//!     blocked_domains: &blocked,
//!     allowed_field_name: "allowed domains",
//!     blocked_field_name: Some("blocked domains"),
//!     empty_allowed_message: "No allowed domains configured",
//!     scheme_policy: UrlSchemePolicy::HttpsOnly,
//!     ipv6_error_context: "external API calls",
//! };
//!
//! let url = "https://api.example.com/endpoint";
//! let validated = validate_url(url, &policy)?;
//! ```

use anyhow::Result;

/// URL 协议策略枚举
///
/// 定义允许的 URL 协议类型，用于在验证时限制可接受的协议范围。
#[derive(Debug, Clone, Copy)]
pub enum UrlSchemePolicy {
    /// 仅允许 HTTPS 协议
    ///
    /// 适用于需要高安全性的外部 API 调用场景，确保所有通信都经过加密。
    HttpsOnly,

    /// 允许 HTTP 或 HTTPS 协议
    ///
    /// 适用于需要兼容旧版服务或内部非加密通信的场景。
    HttpOrHttps,
}

/// 域名访问策略配置
///
/// 包含域名白名单、黑名单及相关验证配置，用于控制 URL 验证的行为。
/// 该结构体采用生命周期参数以避免不必要的字符串复制。
#[derive(Debug, Clone)]
pub struct DomainPolicy<'a> {
    /// 允许访问的域名列表
    ///
    /// 支持以下格式：
    /// - 精确匹配：`"example.com"` 匹配 `example.com`
    /// - 通配符子域名：`"*.example.com"` 匹配 `sub.example.com` 和 `a.b.example.com`
    /// - 全局通配：`"*"` 允许所有域名（不推荐在生产环境使用）
    pub allowed_domains: &'a [String],

    /// 禁止访问的域名列表
    ///
    /// 即使域名在 `allowed_domains` 中匹配，也会被 `blocked_domains` 优先拒绝。
    /// 格式与 `allowed_domains` 相同。
    pub blocked_domains: &'a [String],

    /// 白名单字段的显示名称
    ///
    /// 用于生成错误消息时提示用户该白名单的名称（如 "allowed webhook domains"）。
    pub allowed_field_name: &'a str,

    /// 黑名单字段的显示名称（可选）
    ///
    /// 如果为 `None`，则不执行黑名单检查。如果为 `Some("...")`，
    /// 则会检查域名是否在黑名单中并使用该名称生成错误消息。
    pub blocked_field_name: Option<&'a str>,

    /// 白名单为空时的错误消息
    ///
    /// 当 `allowed_domains` 为空时返回的错误消息，用于提示用户配置域名白名单。
    pub empty_allowed_message: &'a str,

    /// URL 协议策略
    ///
    /// 决定是否接受 HTTP 协议的 URL，或仅接受 HTTPS。
    pub scheme_policy: UrlSchemePolicy,

    /// IPv6 错误消息中的上下文描述
    ///
    /// 当用户尝试访问 IPv6 地址时，错误消息会包含此上下文信息，
    /// 以帮助用户理解为什么在该场景下不允许 IPv6（如 "webhook URLs"）。
    pub ipv6_error_context: &'a str,
}

/// 验证 URL 是否符合域名访问策略
///
/// 该函数执行完整的安全验证流程，包括：
/// 1. 基本格式检查（非空、无空白字符）
/// 2. 白名单配置检查
/// 3. 主机名提取和验证
/// 4. 本地/私有地址检查
/// 5. 黑名单检查
/// 6. 白名单匹配检查
///
/// # 参数
///
/// - `raw_url`: 待验证的原始 URL 字符串
/// - `policy`: 域名访问策略配置
///
/// # 返回值
///
/// - `Ok(String)`: 验证通过，返回规范化后的 URL（去除首尾空白）
/// - `Err`: 验证失败，包含具体的错误原因
///
/// # 错误类型
///
/// 该函数可能在以下情况返回错误：
/// - URL 为空或包含空白字符
/// - 白名单为空
/// - URL 协议不符合策略要求
/// - 主机名为本地地址、私有 IP 或 IPv6 地址
/// - 主机名在黑名单中
/// - 主机名不在白名单中
///
/// # 示例
///
/// ```ignore
/// let policy = DomainPolicy {
///     allowed_domains: &["api.example.com".to_string()],
///     blocked_domains: &[],
///     allowed_field_name: "allowed domains",
///     blocked_field_name: None,
///     empty_allowed_message: "No domains configured",
///     scheme_policy: UrlSchemePolicy::HttpsOnly,
///     ipv6_error_context: "external calls",
/// };
///
/// // 有效 URL
/// assert!(validate_url("https://api.example.com/endpoint", &policy).is_ok());
///
/// // 协议错误
/// assert!(validate_url("http://api.example.com/endpoint", &policy).is_err());
///
/// // 域名不在白名单
/// assert!(validate_url("https://other.com/endpoint", &policy).is_err());
/// ```
pub fn validate_url(raw_url: &str, policy: &DomainPolicy<'_>) -> Result<String> {
    // 去除首尾空白字符
    let url = raw_url.trim();

    // 检查 URL 是否为空
    if url.is_empty() {
        anyhow::bail!("URL cannot be empty");
    }

    // 检查 URL 中是否包含空白字符（如空格、制表符等）
    // 这是防止 URL 注入攻击的基本检查
    if url.chars().any(char::is_whitespace) {
        anyhow::bail!("URL cannot contain whitespace");
    }

    // 检查白名单是否已配置
    // 如果没有配置任何允许的域名，则拒绝所有请求
    if policy.allowed_domains.is_empty() {
        anyhow::bail!("{}", policy.empty_allowed_message);
    }

    // 从 URL 中提取主机名，同时验证协议是否符合策略
    let host = extract_host(url, policy.scheme_policy, policy.ipv6_error_context)?;

    // 检查是否为本地或私有地址
    // 这可以防止 SSRF 攻击，避免代理访问内网资源
    if is_private_or_local_host(&host) {
        anyhow::bail!("Blocked local/private host: {host}");
    }

    // 如果配置了黑名单，检查主机名是否在黑名单中
    // 黑名单检查优先于白名单
    if let Some(blocked_field_name) = policy.blocked_field_name
        && host_matches_allowlist(&host, policy.blocked_domains)
    {
        anyhow::bail!("Host '{host}' is in {blocked_field_name}");
    }

    // 检查主机名是否在白名单中
    // 必须匹配白名单才能通过验证
    if !host_matches_allowlist(&host, policy.allowed_domains) {
        anyhow::bail!("Host '{host}' is not in {}", policy.allowed_field_name);
    }

    // 所有检查通过，返回规范化后的 URL
    Ok(url.to_string())
}

/// 规范化域名列表
///
/// 对域名列表进行统一的格式化处理，包括：
/// - 去除无效条目
/// - 统一转换为小写
/// - 去除重复项
/// - 按字典序排序
///
/// # 参数
///
/// - `domains`: 待规范化的域名列表
///
/// # 返回值
///
/// 返回规范化后的域名列表，已排序且无重复
///
/// # 示例
///
/// ```ignore
/// let domains = vec![
///     "HTTPS://API.EXAMPLE.COM/path".to_string(),
///     "api.example.com".to_string(),
///     "  .cdn.example.com.  ".to_string(),
/// ];
/// let normalized = normalize_allowed_domains(domains);
/// // 结果: ["api.example.com", "cdn.example.com"]
/// ```
pub fn normalize_allowed_domains(domains: Vec<String>) -> Vec<String> {
    // 对每个域名进行规范化处理，过滤掉无效条目
    let mut normalized =
        domains.into_iter().filter_map(|d| normalize_domain(&d)).collect::<Vec<_>>();

    // 按字典序排序，便于后续处理和展示
    normalized.sort_unstable();

    // 去除重复项
    normalized.dedup();

    normalized
}

/// 规范化单个域名
///
/// 从原始输入中提取并规范化域名，处理以下情况：
/// - 去除首尾空白
/// - 转换为小写
/// - 去除协议前缀（http:// 或 https://）
/// - 去除路径部分
/// - 去除端口部分
/// - 去除首尾的点号
///
/// # 参数
///
/// - `raw`: 原始域名或 URL 字符串
///
/// # 返回值
///
/// - `Some(String)`: 规范化后的域名
/// - `None`: 输入无效（空字符串、仅包含空白、规范化后为空等）
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_domain("HTTPS://API.EXAMPLE.COM/path"), Some("api.example.com".to_string()));
/// assert_eq!(normalize_domain("  .cdn.example.com.  "), Some("cdn.example.com".to_string()));
/// assert_eq!(normalize_domain("example.com:8080"), Some("example.com".to_string()));
/// assert_eq!(normalize_domain(""), None);
/// assert_eq!(normalize_domain("   "), None);
/// ```
pub fn normalize_domain(raw: &str) -> Option<String> {
    // 去除首尾空白并转换为小写
    let mut d = raw.trim().to_lowercase();

    // 空字符串直接返回 None
    if d.is_empty() {
        return None;
    }

    // 去除 HTTPS 协议前缀
    if let Some(stripped) = d.strip_prefix("https://") {
        d = stripped.to_string();
    } else if let Some(stripped) = d.strip_prefix("http://") {
        // 去除 HTTP 协议前缀
        d = stripped.to_string();
    }

    // 去除路径部分（/ 及之后的内容）
    if let Some((host, _)) = d.split_once('/') {
        d = host.to_string();
    }

    // 去除首尾的点号（防止 "..example.com" 或 "example.com." 这样的输入）
    d = d.trim_start_matches('.').trim_end_matches('.').to_string();

    // 去除端口号（: 及之后的内容）
    if let Some((host, _)) = d.split_once(':') {
        d = host.to_string();
    }

    // 最终检查：不能为空且不能包含空白字符
    if d.is_empty() || d.chars().any(char::is_whitespace) {
        return None;
    }

    Some(d)
}

/// 从 URL 中提取主机名
///
/// 根据协议策略验证 URL 格式，并提取其中的主机名部分。
/// 该函数会进行严格的安全检查，拒绝可能存在风险的 URL 格式。
///
/// # 参数
///
/// - `url`: 完整的 URL 字符串
/// - `scheme_policy`: 协议策略，决定允许的协议类型
/// - `ipv6_error_context`: IPv6 错误消息的上下文描述
///
/// # 返回值
///
/// - `Ok(String)`: 提取并规范化后的主机名（小写）
/// - `Err`: URL 格式无效或不符合安全要求
///
/// # 安全检查
///
/// 该函数会拒绝以下类型的 URL：
/// - 协议不符合策略要求
/// - URL 中包含用户信息（如 `user@host`）
/// - IPv6 地址格式（出于安全考虑暂不支持）
/// - 缺少主机名的 URL
///
/// # 示例
///
/// ```ignore
/// use UrlSchemePolicy::HttpsOnly;
///
/// let host = extract_host("https://api.example.com/path?query=1", HttpsOnly, "external API")?;
/// assert_eq!(host, "api.example.com");
///
/// // 协议错误
/// assert!(extract_host("http://api.example.com", HttpsOnly, "test").is_err());
///
/// // IPv6 不支持
/// assert!(extract_host("https://[::1]/path", HttpsOnly, "test").is_err());
/// ```
pub fn extract_host(
    url: &str,
    scheme_policy: UrlSchemePolicy,
    ipv6_error_context: &str,
) -> anyhow::Result<String> {
    // 根据协议策略提取 URL 的主体部分（去除协议前缀）
    let rest = match scheme_policy {
        // 仅允许 HTTPS 协议
        UrlSchemePolicy::HttpsOnly => url
            .strip_prefix("https://")
            .ok_or_else(|| anyhow::anyhow!("Only https:// URLs are allowed"))?,
        // 允许 HTTP 或 HTTPS 协议
        UrlSchemePolicy::HttpOrHttps => url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .ok_or_else(|| anyhow::anyhow!("Only http:// and https:// URLs are allowed"))?,
    };

    // 提取授权部分（authority），即主机名和可选的端口
    // 通过查找第一个 /、? 或 # 字符来确定边界
    let authority =
        rest.split(['/', '?', '#']).next().ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

    // 检查授权部分是否为空
    if authority.is_empty() {
        anyhow::bail!("URL must include a host");
    }

    // 安全检查：拒绝包含用户信息的 URL（如 user:pass@host）
    // 这可以防止通过 URL 注入凭证信息
    if authority.contains('@') {
        anyhow::bail!("URL userinfo is not allowed");
    }

    // 安全检查：拒绝 IPv6 地址
    // IPv6 地址使用方括号包裹（如 [::1]），当前不支持以简化安全审计
    if authority.starts_with('[') {
        anyhow::bail!("IPv6 hosts are not supported in {ipv6_error_context}");
    }

    // 提取主机名部分（去除端口号）
    // 同时规范化：去除空白、尾部点号、转换为小写
    let host =
        authority.split(':').next().unwrap_or_default().trim().trim_end_matches('.').to_lowercase();

    // 最终检查：主机名不能为空
    if host.is_empty() {
        anyhow::bail!("URL must include a valid host");
    }

    Ok(host)
}

/// 检查主机名是否匹配白名单模式
///
/// 支持三种匹配模式：
/// 1. 精确匹配：`"example.com"` 匹配 `example.com` 及其子域名
/// 2. 通配符子域名：`"*.example.com"` 匹配 `sub.example.com`（但不匹配 `example.com` 本身）
/// 3. 全局通配：`"*"` 匹配所有主机名
///
/// # 参数
///
/// - `host`: 待检查的主机名（应已转换为小写）
/// - `allowed_domains`: 白名单域名模式列表
///
/// # 返回值
///
/// - `true`: 主机名匹配白名单中的至少一个模式
/// - `false`: 主机名不匹配任何白名单模式
///
/// # 匹配规则说明
///
/// - 精确匹配 `example.com` 会同时匹配 `example.com` 和 `sub.example.com`
/// - 通配符 `*.example.com` 只匹配子域名（如 `sub.example.com`），
///   但实现中也匹配 `example.com` 本身以便于配置
///
/// # 示例
///
/// ```ignore
/// let whitelist = vec!["example.com".to_string(), "*.cdn.example.com".to_string()];
///
/// assert!(host_matches_allowlist("example.com", &whitelist));
/// assert!(host_matches_allowlist("api.example.com", &whitelist));
/// assert!(host_matches_allowlist("static.cdn.example.com", &whitelist));
/// assert!(!host_matches_allowlist("other.com", &whitelist));
/// ```
pub fn host_matches_allowlist(host: &str, allowed_domains: &[String]) -> bool {
    allowed_domains.iter().any(|pattern| {
        // 全局通配：允许所有域名
        if pattern == "*" {
            return true;
        }

        // 通配符子域名匹配：*.example.com
        // 匹配 example.com 本身或其子域名
        if let Some(suffix) = pattern.strip_prefix("*.") {
            return host == suffix || host.ends_with(&format!(".{suffix}"));
        }

        // 精确匹配或子域名匹配
        // example.com 匹配 example.com 和 sub.example.com
        host == pattern || host.ends_with(&format!(".{pattern}"))
    })
}

/// 检查主机名是否为本地或私有地址
///
/// 该函数用于防止 SSRF（服务端请求伪造）攻击，识别并阻止对以下类型地址的访问：
/// - `localhost` 及其子域名（如 `app.localhost`）
/// - `.local` 顶级域名（mDNS 本地域名）
/// - 私有 IPv4 地址（10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 等）
/// - 回环地址（127.0.0.0/8, ::1）
/// - 链路本地地址（169.254.0.0/16, fe80::/10）
/// - 其他非全局可路由地址
///
/// # 参数
///
/// - `host`: 待检查的主机名或 IP 地址字符串
///
/// # 返回值
///
/// - `true`: 主机名为本地或私有地址，应被阻止
/// - `false`: 主机名不是本地或私有地址
///
/// # 示例
///
/// ```ignore
/// // 本地域名
/// assert!(is_private_or_local_host("localhost"));
/// assert!(is_private_or_local_host("app.localhost"));
/// assert!(is_private_or_local_host("printer.local"));
///
/// // 私有 IP 地址
/// assert!(is_private_or_local_host("192.168.1.1"));
/// assert!(is_private_or_local_host("10.0.0.1"));
/// assert!(is_private_or_local_host("127.0.0.1"));
///
/// // 公网地址
/// assert!(!is_private_or_local_host("api.example.com"));
/// assert!(!is_private_or_local_host("8.8.8.8"));
/// ```
pub fn is_private_or_local_host(host: &str) -> bool {
    // 去除 IPv6 地址的方括号（虽然当前不支持 IPv6，但保留防御性代码）
    let bare = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);

    // 检查 .local 顶级域名（mDNS 本地域名）
    let has_local_tld = bare.rsplit('.').next().is_some_and(|label| label == "local");

    // 检查 localhost 相关的域名
    if bare == "localhost" || bare.ends_with(".localhost") || has_local_tld {
        return true;
    }

    // 尝试解析为 IP 地址并检查是否为非全局地址
    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
            std::net::IpAddr::V6(v6) => is_non_global_v6(v6),
        };
    }

    // 不是本地/私有地址
    false
}

/// 检查 IPv4 地址是否为非全局可路由地址
///
/// 识别以下类型的 IPv4 地址：
/// - 回环地址（127.0.0.0/8）
/// - 私有地址（10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16）
/// - 链路本地地址（169.254.0.0/16）
/// - 未指定地址（0.0.0.0）
/// - 广播地址（255.255.255.255）
/// - 组播地址（224.0.0.0/4）
/// - CGNAT 地址（100.64.0.0/10）
/// - 保留地址（240.0.0.0/4）
/// - 文档/测试地址（192.0.0.0/24, 198.51.100.0/24, 203.0.113.0/24）
/// - 基准测试地址（198.18.0.0/15）
///
/// # 参数
///
/// - `v4`: IPv4 地址
///
/// # 返回值
///
/// - `true`: 地址为非全局可路由地址
/// - `false`: 地址为全局可路由地址
fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    // 提取地址的四个八位组
    let [a, b, c, _] = v4.octets();

    v4.is_loopback()                        // 127.0.0.0/8
        || v4.is_private()                  // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
        || v4.is_link_local()               // 169.254.0.0/16
        || v4.is_unspecified()              // 0.0.0.0
        || v4.is_broadcast()                // 255.255.255.255
        || v4.is_multicast()                // 224.0.0.0/4
        || (a == 100 && (64..=127).contains(&b))  // CGNAT: 100.64.0.0/10
        || a >= 240                         // 保留地址: 240.0.0.0/4 (除广播地址外)
        || (a == 192 && b == 0 && (c == 0 || c == 2))  // IETF 协议分配: 192.0.0.0/24
        || (a == 198 && b == 51)            // TEST-NET-2: 198.51.100.0/24
        || (a == 203 && b == 0)             // TEST-NET-3: 203.0.113.0/24
        || (a == 198 && (18..=19).contains(&b)) // 基准测试: 198.18.0.0/15
}

/// 检查 IPv6 地址是否为非全局可路由地址
///
/// 识别以下类型的 IPv6 地址：
/// - 回环地址（::1）
/// - 未指定地址（::）
/// - 组播地址（ff00::/8）
/// - 唯一本地地址（fc00::/7，类似于 IPv4 私有地址）
/// - 链路本地地址（fe80::/10）
/// - 文档地址（2001:db8::/32）
/// - IPv4 映射地址（::ffff:0:0/96）中映射到非全局 IPv4 的地址
///
/// # 参数
///
/// - `v6`: IPv6 地址
///
/// # 返回值
///
/// - `true`: 地址为非全局可路由地址
/// - `false`: 地址为全局可路由地址
fn is_non_global_v6(v6: std::net::Ipv6Addr) -> bool {
    // 提取地址的 8 个 16 位段
    let segs = v6.segments();

    v6.is_loopback()                        // ::1
        || v6.is_unspecified()              // ::
        || v6.is_multicast()                // ff00::/8
        || (segs[0] & 0xfe00) == 0xfc00    // 唯一本地地址: fc00::/7
        || (segs[0] & 0xffc0) == 0xfe80    // 链路本地地址: fe80::/10
        || (segs[0] == 0x2001 && segs[1] == 0x0db8)  // 文档地址: 2001:db8::/32
        || v6.to_ipv4_mapped().is_some_and(is_non_global_v4) // IPv4 映射地址
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
