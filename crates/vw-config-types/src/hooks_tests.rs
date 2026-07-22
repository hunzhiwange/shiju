#[test]
fn hooks_defaults_enable_runtime_hooks() {
    let config = super::HooksConfig::default();
    assert!(config.enabled);
    assert!(!config.builtin.command_logger);
    assert!(config.plugins.enabled);

    let parsed: super::HooksConfig = serde_json::from_str("{\"enabled\":false}").unwrap();
    assert!(!parsed.enabled);
}

#[test]
fn hooks_plugin_hooks_can_be_disabled() {
    let parsed: super::HooksConfig =
        serde_json::from_str("{\"plugins\":{\"enabled\":false}}").unwrap();

    assert!(parsed.enabled);
    assert!(!parsed.plugins.enabled);
}
