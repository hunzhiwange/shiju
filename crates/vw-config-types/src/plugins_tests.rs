use crate::config::Config;

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
