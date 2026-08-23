//! 验证插件来源的空默认值以及显式路径的去空白与去重行为。
//! 去重保持首次出现顺序，确保发现优先级不因清理过程改变。

use crate::config::Config;

// 空配置不限制插件名，也不额外扫描配置目录之外的来源。
#[test]
fn plugins_config_defaults_are_empty() {
    let config = super::PluginsConfig::default();
    assert!(config.enabled.is_empty());
    assert!(config.sources.is_empty());

    let default_config = Config::default();
    assert!(default_config.plugins.enabled.is_empty());
    assert!(default_config.plugins.sources.is_empty());

    let parsed: super::PluginsConfig = serde_json::from_str("{}").unwrap();
    assert!(parsed.enabled.is_empty());
    assert!(parsed.sources.is_empty());
}

// 输入两端空白不属于路径，重复来源只保留最早出现的一项。
#[test]
fn plugins_config_deduplicates_explicit_sources() {
    let parsed: super::PluginsConfig = serde_json::from_value(serde_json::json!({
        "enabled": ["alpha"],
        "sources": [
            " /tmp/vw/plugins ",
            "/opt/vw/plugins",
            "/tmp/vw/plugins"
        ]
    }))
    .unwrap();

    println!("deduped plugin sources: {:?}", parsed.sources);
    assert_eq!(parsed.sources, vec!["/tmp/vw/plugins".to_string(), "/opt/vw/plugins".to_string()]);
    assert_eq!(parsed.enabled, vec!["alpha".to_string()]);
}
