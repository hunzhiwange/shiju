//! 验证代理配置的默认值、作用域匹配、地址归一化与选择器辅助逻辑。
//! 测试将启用判定、服务选择和字符串清理拆开，明确每层配置责任。

// 空配置不代理任何流量，但公开的支持范围常量必须保持稳定。
#[test]
fn proxy_config_default_values() {
    let config = super::ProxyConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.http_proxy, None);
    assert_eq!(config.https_proxy, None);
    assert_eq!(config.all_proxy, None);
    assert!(config.no_proxy.is_empty());
    assert_eq!(config.scope, super::ProxyScope::Vibewindow);
    assert!(config.services.is_empty());
}

#[test]
fn proxy_config_supported_helpers_return_constants() {
    assert_eq!(super::ProxyConfig::supported_service_keys(), super::SUPPORTED_PROXY_SERVICE_KEYS,);
    assert_eq!(
        super::ProxyConfig::supported_service_selectors(),
        super::SUPPORTED_PROXY_SERVICE_SELECTORS,
    );
}

// 归一化负责消除大小写和空白差异，作用域匹配只消费规范化结果。
#[test]
fn proxy_normalization_and_scope_matching_work() {
    let config = super::ProxyConfig {
        enabled: true,
        http_proxy: Some(" http://proxy ".into()),
        services: vec!["provider.*".into(), "tool.browser".into(), "provider.*".into()],
        no_proxy: vec![" localhost, 127.0.0.1 ".into()],
        scope: super::ProxyScope::Services,
        ..Default::default()
    };

    assert!(config.has_any_proxy_url());
    assert_eq!(config.normalized_services(), vec!["provider.*", "tool.browser"]);
    assert_eq!(config.normalized_no_proxy(), vec!["127.0.0.1", "localhost"]);
    assert!(config.should_apply_to_service("provider.openai"));
    assert!(config.should_apply_to_service("tool.browser"));
    assert!(!config.should_apply_to_service("channel.slack"));
}

// 是否存在代理地址与是否启用代理是两个独立判断，避免半配置被误用。
#[test]
fn proxy_has_any_proxy_url_variants() {
    let none_set = super::ProxyConfig::default();
    assert!(!none_set.has_any_proxy_url());

    let https_only =
        super::ProxyConfig { https_proxy: Some("https://proxy".into()), ..Default::default() };
    assert!(https_only.has_any_proxy_url());

    let all_only =
        super::ProxyConfig { all_proxy: Some("socks5://proxy".into()), ..Default::default() };
    assert!(all_only.has_any_proxy_url());
}

// 应用范围覆盖禁用、环境变量及 VibeWindow 自有请求三条互斥路径。
#[test]
fn proxy_should_apply_disabled() {
    let config = super::ProxyConfig {
        enabled: false,
        scope: super::ProxyScope::Vibewindow,
        ..Default::default()
    };
    assert!(!config.should_apply_to_service("provider.openai"));
}

#[test]
fn proxy_should_apply_environment_scope() {
    let config = super::ProxyConfig {
        enabled: true,
        scope: super::ProxyScope::Environment,
        ..Default::default()
    };
    assert!(!config.should_apply_to_service("provider.openai"));
}

#[test]
fn proxy_should_apply_vibewindow_scope() {
    let config = super::ProxyConfig {
        enabled: true,
        scope: super::ProxyScope::Vibewindow,
        ..Default::default()
    };
    assert!(config.should_apply_to_service("provider.openai"));
}

// 服务选择器支持显式键和通配符，空键集合保持不匹配语义。
#[test]
fn proxy_should_apply_services_empty_key() {
    let config = super::ProxyConfig {
        enabled: true,
        scope: super::ProxyScope::Services,
        services: vec!["provider.*".into()],
        ..Default::default()
    };
    assert!(!config.should_apply_to_service(""));
    assert!(!config.should_apply_to_service("   "));
}

#[test]
fn proxy_selector_helpers_cover_keys_and_wildcards() {
    assert!(super::is_supported_proxy_service_selector("provider.openai"));
    assert!(super::is_supported_proxy_service_selector("tool.*"));
    assert!(!super::is_supported_proxy_service_selector("unknown.service"));
    assert!(super::service_selector_matches("provider.*", "provider.openrouter"));
    assert!(!super::service_selector_matches("provider.*", "provider"));
    assert!(super::service_selector_matches("provider.openai", "provider.openai"));
    assert!(!super::service_selector_matches("provider.openai", "provider.anthropic"));
    assert!(!super::service_selector_matches("provider*", "provider.openai"));
    assert_eq!(super::normalize_proxy_url_option(Some("   ")), None);
}

#[test]
fn proxy_normalize_proxy_url_option_none() {
    assert_eq!(super::normalize_proxy_url_option(None), None);
    assert_eq!(
        super::normalize_proxy_url_option(Some("http://proxy:8080")),
        Some("http://proxy:8080".to_string()),
    );
}

// 逗号分隔字段需统一去空白、去空项和去重，供服务及 no_proxy 共用。
#[test]
fn proxy_normalize_comma_values_edge_cases() {
    assert!(super::normalize_comma_values(vec![] as Vec<String>).is_empty());
    assert_eq!(super::normalize_comma_values(vec!["value".into()]), vec!["value"]);
    assert_eq!(super::normalize_comma_values(vec!["a,b,c".into()]), vec!["a", "b", "c"]);
    assert!(super::normalize_comma_values(vec!["   ,  ,  ".into()]).is_empty());
    assert_eq!(super::normalize_comma_values(vec!["a,,b".into()]), vec!["a", "b"]);
    assert_eq!(super::normalize_comma_values(vec!["a".into(), "a".into()]), vec!["a"]);
    assert_eq!(
        super::normalize_comma_values(vec!["a,b".into(), "c,d".into()]),
        vec!["a", "b", "c", "d"],
    );
}

#[test]
fn proxy_normalize_service_list() {
    assert_eq!(
        super::normalize_service_list(vec!["Provider.OpenAI".into(), "provider.*".into()]),
        vec!["provider.*", "provider.openai"],
    );
    assert!(super::normalize_service_list(vec![] as Vec<String>).is_empty());
}

#[test]
fn proxy_normalize_no_proxy() {
    assert_eq!(
        super::normalize_no_proxy_list(vec![" localhost, 127.0.0.1 ".into()]),
        vec!["127.0.0.1", "localhost"],
    );
    assert!(super::normalize_no_proxy_list(vec![] as Vec<String>).is_empty());
}
