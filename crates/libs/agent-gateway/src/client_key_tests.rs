//! 客户端地址提取的安全边界测试。

use super::*;
use axum::http::HeaderValue;

/// 测试非受信代理模式下客户端密钥的默认行为
///
/// ## 测试目的
/// 验证当 `trusted_proxy_mode` 为 `false` 时，即使请求中包含
/// `X-Forwarded-For` 头，系统也应忽略该头，直接使用 TCP 连接的
/// 对端地址作为客户端标识。
///
/// ## 场景描述
/// - 对端地址: `10.0.0.5:42617`（内网地址）
/// - 转发头内容: `198.51.100.10, 203.0.113.11`（两个公网 IP）
/// - 代理模式: 非受信（`false`）
///
/// ## 预期结果
/// 返回对端 IP `10.0.0.5`，完全忽略转发头中的地址。
///
/// ## 安全考虑
/// 这是默认安全的行为：防止客户端通过伪造转发头绕过基于 IP 的限制。
#[test]
fn client_key_defaults_to_peer_addr_when_untrusted_proxy_mode() {
    // 构造测试用的对端地址（模拟内网客户端）
    let peer = SocketAddr::from(([10, 0, 0, 5], 42617));

    // 构造包含多个转发地址的 HTTP 头
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-For", HeaderValue::from_static("198.51.100.10, 203.0.113.11"));

    // 在非受信代理模式下提取客户端密钥
    let key = client_key_from_request(Some(peer), &headers, false);

    // 断言：应使用对端地址，忽略转发头
    assert_eq!(key, "10.0.0.5");
}

/// 测试受信代理模式下客户端密钥的提取行为
///
/// ## 测试目的
/// 验证当 `trusted_proxy_mode` 为 `true` 时，系统应解析
/// `X-Forwarded-For` 头并使用第一个 IP 地址作为客户端标识。
///
/// ## 场景描述
/// - 对端地址: `10.0.0.5:42617`（代理服务器地址）
/// - 转发头内容: `198.51.100.10, 203.0.113.11`（原始客户端 + 中间代理）
/// - 代理模式: 受信（`true`）
///
/// ## 预期结果
/// 返回转发头中的第一个 IP `198.51.100.10`，即原始客户端地址。
///
/// ## 使用前提
/// 此模式仅在网关部署于可信的反向代理（如 Nginx、Cloudflare）之后时启用。
/// 代理负责将真实客户端 IP 注入 `X-Forwarded-For` 头的起始位置。
#[test]
fn client_key_uses_forwarded_ip_only_in_trusted_proxy_mode() {
    // 构造测试用的对端地址（模拟代理服务器）
    let peer = SocketAddr::from(([10, 0, 0, 5], 42617));

    // 构造包含多个转发地址的 HTTP 头
    // 格式: 客户端IP, 代理1, 代理2, ...
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-For", HeaderValue::from_static("198.51.100.10, 203.0.113.11"));

    // 在受信代理模式下提取客户端密钥
    let key = client_key_from_request(Some(peer), &headers, true);

    // 断言：应使用转发头中的第一个 IP（原始客户端）
    assert_eq!(key, "198.51.100.10");
}

/// 测试转发头无效时的回退行为
///
/// ## 测试目的
/// 验证当 `X-Forwarded-For` 头包含无效内容时，即使在受信代理模式下，
/// 系统也应安全回退到使用对端地址作为客户端标识。
///
/// ## 场景描述
/// - 对端地址: `10.0.0.5:42617`
/// - 转发头内容: `garbage-value`（非有效 IP 格式）
/// - 代理模式: 受信（`true`）
///
/// ## 预期结果
/// 由于无法解析转发头，回退到对端 IP `10.0.0.5`。
///
/// ## 健壮性保障
/// 此测试确保系统对畸形输入具有防御性，不会因解析失败而崩溃或
/// 返回空值。回退策略确保速率限制仍然有效。
#[test]
fn client_key_falls_back_to_peer_when_forwarded_header_invalid() {
    // 构造测试用的对端地址
    let peer = SocketAddr::from(([10, 0, 0, 5], 42617));

    // 构造包含无效内容的转发头
    let mut headers = HeaderMap::new();
    headers.insert("X-Forwarded-For", HeaderValue::from_static("garbage-value"));

    // 在受信代理模式下尝试提取（但头内容无效）
    let key = client_key_from_request(Some(peer), &headers, true);

    // 断言：应回退到对端地址
    assert_eq!(key, "10.0.0.5");
}
