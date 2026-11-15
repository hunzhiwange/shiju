//! URL 验证模块测试
//!
//! 本模块提供对 URL 验证相关功能的全面测试覆盖，确保 URL 处理的安全性和正确性。
//! URL 验证是代理系统的关键安全边界，用于防止 SSRF（服务端请求伪造）等攻击。
//!
//! # 测试覆盖范围
//!
//! - **域名规范化**：测试域名格式化、去重、大小写处理
//! - **主机名匹配**：测试精确匹配、子域名匹配、通配符模式
//! - **主机提取**：测试从 URL 中提取主机名的逻辑
//! - **私有主机检测**：测试识别本地地址和私有 IP 地址
//! - **URL 验证**：测试完整的 URL 验证流程
//!
//! # 安全考虑
//!
//! 这些测试验证以下安全策略：
//! - 阻止对私有网络地址（如 127.0.0.1、192.168.x.x）的请求
//! - 阻止对 localhost 及其子域名的请求
//! - 严格限制允许访问的域名范围
//! - 禁止包含用户信息的 URL（防止认证信息泄露）
//! - 可选地禁止 IPv6 地址（防止绕过 IPv4 过滤规则）

use super::{
    DomainPolicy, UrlSchemePolicy, extract_host, host_matches_allowlist, is_private_or_local_host,
    normalize_allowed_domains, normalize_domain, validate_url,
};

/// 测试域名规范化：移除协议和路径
///
/// 验证 `normalize_domain` 函数能够：
/// - 移除 URL 协议部分（如 https://）
/// - 移除 URL 路径部分（如 /path）
/// - 将域名转换为小写（规范化处理）
///
/// # 示例
///
/// 输入：`https://Docs.Example.com/path`
/// 输出：`docs.example.com`
#[test]
fn normalize_domain_strips_scheme_and_path() {
    let got = normalize_domain("https://Docs.Example.com/path").unwrap();
    assert_eq!(got, "docs.example.com");
}

/// 测试域名规范化：拒绝包含空白的域名
///
/// 验证 `normalize_domain` 函数能够拒绝包含空格等空白字符的无效域名。
/// 这可以防止域名伪造攻击，如尝试使用空格混淆域名。
#[test]
fn normalize_domain_rejects_whitespace() {
    assert!(normalize_domain("exa mple.com").is_none());
}

/// 测试允许域名列表去重
///
/// 验证 `normalize_allowed_domains` 函数能够：
/// - 去除重复的域名（即使大小写不同）
/// - 规范化域名格式（移除协议、路径、统一为小写）
///
/// # 示例
///
/// 输入：`["example.com", "EXAMPLE.COM", "https://example.com/"]`
/// 输出：`["example.com"]`
#[test]
fn normalize_allowed_domains_deduplicates() {
    let got = normalize_allowed_domains(vec![
        "example.com".into(),
        "EXAMPLE.COM".into(),
        "https://example.com/".into(),
    ]);
    assert_eq!(got, vec!["example.com".to_string()]);
}

/// 测试主机名匹配：精确匹配
///
/// 验证当请求的主机名与白名单中的域名完全一致时，匹配成功。
#[test]
fn host_matches_allowlist_exact() {
    assert!(host_matches_allowlist("example.com", &["example.com".into()]));
}

/// 测试主机名匹配：子域名匹配
///
/// 验证当白名单中包含父域名时，其子域名也能匹配成功。
/// 例如：白名单包含 "example.com"，则 "docs.example.com" 也应被允许。
#[test]
fn host_matches_allowlist_subdomain() {
    assert!(host_matches_allowlist("docs.example.com", &["example.com".into()]));
}

/// 测试主机名匹配：通配符模式
///
/// 验证使用 `*.example.com` 通配符模式时：
/// - 子域名（如 api.example.com）能够匹配
/// - 顶级域名（如 example.com）也能够匹配
#[test]
fn host_matches_allowlist_wildcard_pattern() {
    assert!(host_matches_allowlist("api.example.com", &["*.example.com".into()]));
    assert!(host_matches_allowlist("example.com", &["*.example.com".into()]));
}

/// 测试主机名匹配：全局通配符
///
/// 验证使用 `*` 全局通配符时，所有主机名都能匹配成功。
/// 这是一个"允许所有"的策略，通常仅在测试或特殊场景使用。
#[test]
fn host_matches_allowlist_global_wildcard() {
    assert!(host_matches_allowlist("example.net", &["*".into()]));
}

/// 测试主机提取：支持 HTTP 和 HTTPS
///
/// 验证从包含路径的 URL 中正确提取主机名。
/// 当 URL 协议策略为 `HttpOrHttps` 时，HTTP 和 HTTPS 都应被接受。
#[test]
fn extract_host_supports_http_and_https() {
    let host =
        extract_host("http://api.example.com/path", UrlSchemePolicy::HttpOrHttps, "http_request")
            .unwrap();
    assert_eq!(host, "api.example.com");
}

/// 测试主机提取：在仅 HTTPS 模式下拒绝非 HTTPS URL
///
/// 验证当协议策略为 `HttpsOnly` 时，HTTP URL 应被拒绝。
/// 这是浏览器打开等敏感操作的安全要求。
#[test]
fn extract_host_rejects_non_https_when_https_only() {
    let err = extract_host("http://example.com", UrlSchemePolicy::HttpsOnly, "browser_open")
        .unwrap_err()
        .to_string();
    assert!(err.contains("Only https://"));
}

/// 测试主机提取：拒绝包含用户信息的 URL
///
/// 验证包含用户信息（如 `user@example.com`）的 URL 被拒绝。
/// 这可以防止：
/// - 认证信息泄露
/// - URL 注入攻击
/// - 用户名/密码在日志中暴露
#[test]
fn extract_host_rejects_userinfo() {
    let err = extract_host("https://user@example.com", UrlSchemePolicy::HttpsOnly, "browser_open")
        .unwrap_err()
        .to_string();
    assert!(err.contains("userinfo"));
}

/// 测试主机提取：拒绝 IPv6 字面量地址
///
/// 验证 IPv6 地址格式（如 `[::1]`）被拒绝。
/// IPv6 地址可能被用于绕过基于 IPv4 的安全过滤规则，
/// 因此默认情况下拒绝处理。
#[test]
fn extract_host_rejects_ipv6_literal() {
    let err = extract_host("https://[::1]:8443", UrlSchemePolicy::HttpsOnly, "browser_open")
        .unwrap_err()
        .to_string();
    assert!(err.contains("IPv6"));
}

/// 测试私有主机检测：localhost 及其子域名
///
/// 验证以下地址被识别为私有/本地地址：
/// - `localhost`：本地回环域名
/// - `*.localhost`：localhost 的任意子域名
///
/// 这些地址应被阻止，以防止 SSRF 攻击访问本地服务。
#[test]
fn private_host_detection_localhost() {
    assert!(is_private_or_local_host("localhost"));
    assert!(is_private_or_local_host("api.localhost"));
}

/// 测试私有主机检测：私有 IPv4 地址
///
/// 验证 RFC 1918 定义的私有 IPv4 地址被正确识别：
/// - `10.0.0.0/8`：A 类私有地址（10.0.0.1）
/// - `172.16.0.0/12`：B 类私有地址（172.16.0.1）
/// - `192.168.0.0/16`：C 类私有地址（192.168.1.5）
#[test]
fn private_host_detection_private_ipv4() {
    assert!(is_private_or_local_host("10.0.0.1"));
    assert!(is_private_or_local_host("172.16.0.1"));
    assert!(is_private_or_local_host("192.168.1.5"));
}

/// 测试私有主机检测：公网 IPv4 地址
///
/// 验证公网 IP 地址（如 8.8.8.8）不被识别为私有地址。
/// 这些地址是正常的访问目标。
#[test]
fn private_host_detection_public_ipv4() {
    assert!(!is_private_or_local_host("8.8.8.8"));
}

/// 测试私有主机检测：本地 IPv6 地址
///
/// 验证 IPv6 本地地址被正确识别：
/// - `::1`：IPv6 本地回环地址
/// - `fd00::/8`：IPv6 唯一本地地址（ULA）
#[test]
fn private_host_detection_ipv6_local() {
    assert!(is_private_or_local_host("::1"));
    assert!(is_private_or_local_host("fd00::1"));
}

/// 测试私有主机检测：公网 IPv6 地址
///
/// 验证公网 IPv6 地址不被识别为私有地址。
#[test]
fn private_host_detection_ipv6_public() {
    assert!(!is_private_or_local_host("2607:f8b0:4004:800::200e"));
}

/// 创建测试用的域名策略配置
///
/// 这是一个辅助函数，用于简化测试用例中 `DomainPolicy` 的创建。
/// 所有测试用例都使用一致的默认配置，只改变域名列表。
///
/// # 参数
///
/// - `allowed_domains`：允许访问的域名列表
/// - `blocked_domains`：禁止访问的域名列表
///
/// # 返回值
///
/// 返回配置好的 `DomainPolicy` 实例，包含：
/// - 允许/禁止的域名列表
/// - 配置字段名称（用于错误提示）
/// - 协议策略：HTTP 或 HTTPS
/// - IPv6 错误上下文信息
fn policy<'a>(allowed_domains: &'a [String], blocked_domains: &'a [String]) -> DomainPolicy<'a> {
    DomainPolicy {
        allowed_domains,
        blocked_domains,
        allowed_field_name: "web_fetch.allowed_domains",
        blocked_field_name: Some("web_fetch.blocked_domains"),
        empty_allowed_message: "allowed domains must be configured",
        scheme_policy: UrlSchemePolicy::HttpOrHttps,
        ipv6_error_context: "web_fetch",
    }
}

/// 测试 URL 验证：接受允许列表中的公网主机
///
/// 验证完整的 URL 验证流程：
/// 1. URL 协议符合要求
/// 2. 主机名在允许列表中（支持子域名匹配）
/// 3. 主机名不在禁止列表中
/// 4. 主机名不是私有/本地地址
///
/// # 示例
///
/// 允许列表：`["example.com"]`
/// 请求 URL：`https://docs.example.com/path`
/// 预期结果：验证通过，返回原始 URL
#[test]
fn validate_url_accepts_public_allowed_host() {
    let allowed = vec!["example.com".to_string()];
    let blocked: Vec<String> = Vec::new();
    let got = validate_url("https://docs.example.com/path", &policy(&allowed, &blocked)).unwrap();
    assert_eq!(got, "https://docs.example.com/path");
}

/// 测试 URL 验证：拒绝禁止列表中的主机
///
/// 验证即使域名在允许列表中（全局通配符 `*`），
/// 如果同时在禁止列表中，请求仍会被拒绝。
///
/// # 示例
///
/// 允许列表：`["*"]`（允许所有）
/// 禁止列表：`["example.com"]`
/// 请求 URL：`https://example.com`
/// 预期结果：验证失败，错误信息包含 "blocked_domains"
#[test]
fn validate_url_rejects_blocked_host() {
    let allowed = vec!["*".to_string()];
    let blocked = vec!["example.com".to_string()];
    let err =
        validate_url("https://example.com", &policy(&allowed, &blocked)).unwrap_err().to_string();
    assert!(err.contains("blocked_domains"));
}

/// 测试 URL 验证：拒绝私有主机地址
///
/// 验证即使域名在允许列表中，私有/本地地址仍会被拒绝。
/// 这是 SSRF 防护的核心机制。
///
/// # 示例
///
/// 允许列表：`["*"]`（允许所有）
/// 请求 URL：`https://127.0.0.1`
/// 预期结果：验证失败，错误信息包含 "local/private"
#[test]
fn validate_url_rejects_private_host() {
    let allowed = vec!["*".to_string()];
    let blocked: Vec<String> = Vec::new();
    let err =
        validate_url("https://127.0.0.1", &policy(&allowed, &blocked)).unwrap_err().to_string();
    assert!(err.contains("local/private"));
}

/// 测试 URL 验证：拒绝不在允许列表中的主机
///
/// 验证默认拒绝策略：只允许明确在白名单中的域名。
///
/// # 示例
///
/// 允许列表：`["example.com"]`
/// 请求 URL：`https://google.com`
/// 预期结果：验证失败，错误信息包含 "allowed_domains"
#[test]
fn validate_url_rejects_allowlist_miss() {
    let allowed = vec!["example.com".to_string()];
    let blocked: Vec<String> = Vec::new();
    let err =
        validate_url("https://google.com", &policy(&allowed, &blocked)).unwrap_err().to_string();
    assert!(err.contains("allowed_domains"));
}

/// 测试 URL 验证：拒绝空允许列表
///
/// 验证当允许列表为空时，所有请求都会被拒绝。
/// 这是一个重要的安全默认值，强制管理员明确配置允许的域名。
///
/// # 示例
///
/// 允许列表：`[]`（空）
/// 请求 URL：`https://example.com`
/// 预期结果：验证失败，错误信息包含 "allowed domains must be configured"
#[test]
fn validate_url_rejects_empty_allowlist() {
    let allowed: Vec<String> = Vec::new();
    let blocked: Vec<String> = Vec::new();
    let err =
        validate_url("https://example.com", &policy(&allowed, &blocked)).unwrap_err().to_string();
    assert!(err.contains("allowed domains must be configured"));
}
