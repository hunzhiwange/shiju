// HTTP 请求默认允许公网域名，显式白名单和其他覆盖项必须独立生效。
#[test]
fn http_request_config_defaults_and_overrides_deserialize() {
    let config = super::HttpRequestConfig::default();
    assert!(config.enabled);
    assert_eq!(config.allowed_domains, ["*"]);
    assert_eq!(config.max_response_size, 1_000_000);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::HttpRequestConfig = serde_json::from_str("{}").unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.allowed_domains, ["*"]);
    assert_eq!(parsed.max_response_size, 1_000_000);
    assert_eq!(parsed.timeout_secs, 30);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::HttpRequestConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "allowed_domains": ["api.example.com"],
        "max_response_size": 2048,
        "timeout_secs": 7,
        "user_agent": "TestAgent/2.0"
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.allowed_domains, vec!["api.example.com"]);
    assert_eq!(custom.max_response_size, 2048);
    assert_eq!(custom.timeout_secs, 7);
    assert_eq!(custom.user_agent, "TestAgent/2.0");

    let disabled: super::HttpRequestConfig =
        serde_json::from_str(r#"{"enabled":false,"allowed_domains":[]}"#).unwrap();
    assert!(!disabled.enabled);
    assert!(disabled.allowed_domains.is_empty());
}

// 历史默认用户代理可升级到当前值，用户自定义字符串则必须保留。
#[test]
fn user_agent_resolution_upgrades_legacy_default_and_preserves_override() {
    assert_eq!(super::resolve_user_agent(""), super::DEFAULT_USER_AGENT);
    assert_eq!(super::resolve_user_agent(" VibeWindow/1.0 "), super::DEFAULT_USER_AGENT);
    assert_eq!(super::resolve_user_agent(" CustomAgent/2.0 "), "CustomAgent/2.0");
}

// `Default` 与字段级反序列化默认刻意不同，此用例防止二者被误合并。
#[test]
fn web_fetch_config_default_impl_differs_from_field_level_deserialize_defaults() {
    let config = super::WebFetchConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.provider, "fast_html2md");
    assert_eq!(config.api_key, None);
    assert_eq!(config.api_url, None);
    assert_eq!(config.allowed_domains, vec!["*"]);
    assert!(config.blocked_domains.is_empty());
    assert_eq!(config.max_response_size, 500_000);
    assert_eq!(config.timeout_secs, 30);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::WebFetchConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.enabled);
    assert_eq!(parsed.provider, "fast_html2md");
    assert!(parsed.allowed_domains.is_empty());
    assert!(parsed.blocked_domains.is_empty());
    assert_eq!(parsed.max_response_size, 500_000);
    assert_eq!(parsed.timeout_secs, 30);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::WebFetchConfig = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "provider": "firecrawl",
        "api_key": "k1,k2",
        "api_url": "https://crawler.example.com",
        "allowed_domains": ["example.com"],
        "blocked_domains": ["private.example.com"],
        "max_response_size": 1024,
        "timeout_secs": 9,
        "user_agent": "Crawler/9.9"
    }))
    .unwrap();
    assert!(custom.enabled);
    assert_eq!(custom.provider, "firecrawl");
    assert_eq!(custom.api_key.as_deref(), Some("k1,k2"));
    assert_eq!(custom.api_url.as_deref(), Some("https://crawler.example.com"));
    assert_eq!(custom.allowed_domains, vec!["example.com"]);
    assert_eq!(custom.blocked_domains, vec!["private.example.com"]);
    assert_eq!(custom.max_response_size, 1024);
    assert_eq!(custom.timeout_secs, 9);
    assert_eq!(custom.user_agent, "Crawler/9.9");
}

// 搜索配置锁定提供方、结果上限与网络参数的缺省和覆盖行为。
#[test]
fn web_search_config_defaults_and_overrides_deserialize() {
    let config = super::WebSearchConfig::default();
    assert!(config.enabled);
    assert_eq!(config.provider, "duckduckgo");
    assert_eq!(config.api_key, None);
    assert_eq!(config.api_url, None);
    assert_eq!(config.brave_api_key, None);
    assert_eq!(config.max_results, 5);
    assert_eq!(config.timeout_secs, 15);
    assert_eq!(config.user_agent, super::DEFAULT_USER_AGENT);

    let parsed: super::WebSearchConfig = serde_json::from_str("{}").unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.provider, "duckduckgo");
    assert_eq!(parsed.max_results, 5);
    assert_eq!(parsed.timeout_secs, 15);
    assert_eq!(parsed.user_agent, super::DEFAULT_USER_AGENT);

    let custom: super::WebSearchConfig = serde_json::from_value(serde_json::json!({
        "enabled": false,
        "provider": "brave",
        "api_key": "shared-key",
        "api_url": "https://search.example.com",
        "brave_api_key": "brave-key",
        "max_results": 10,
        "timeout_secs": 3,
        "user_agent": "SearchBot/1.2"
    }))
    .unwrap();
    assert!(!custom.enabled);
    assert_eq!(custom.provider, "brave");
    assert_eq!(custom.api_key.as_deref(), Some("shared-key"));
    assert_eq!(custom.api_url.as_deref(), Some("https://search.example.com"));
    assert_eq!(custom.brave_api_key.as_deref(), Some("brave-key"));
    assert_eq!(custom.max_results, 10);
    assert_eq!(custom.timeout_secs, 3);
    assert_eq!(custom.user_agent, "SearchBot/1.2");
}
