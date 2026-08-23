use super::*;

#[test]
fn lark_and_feishu_base_urls_use_https() {
    assert!(FEISHU_BASE_URL.starts_with("https://"));
    assert!(LARK_BASE_URL.starts_with("https://"));
    assert!(FEISHU_WS_BASE_URL.starts_with("https://"));
    assert!(LARK_WS_BASE_URL.starts_with("https://"));
}

#[test]
fn token_refresh_skew_is_shorter_than_default_ttl() {
    assert!(LARK_TOKEN_REFRESH_SKEW < LARK_DEFAULT_TOKEN_TTL);
}
// 本文件验证 Lark 与飞书端点及令牌刷新常量的安全约束。
