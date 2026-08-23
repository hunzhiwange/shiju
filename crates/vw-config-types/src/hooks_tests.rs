//! 验证运行时钩子及插件钩子的默认启用状态与显式关闭配置。
//! 顶层开关与插件子开关彼此独立，测试防止反序列化时相互覆盖。

// 空配置启用钩子基础设施，但高噪声的命令日志默认保持关闭。
#[test]
fn hooks_defaults_enable_runtime_hooks() {
    let config = super::HooksConfig::default();
    assert!(config.enabled);
    assert!(!config.builtin.command_logger);
    assert!(config.plugins.enabled);

    let parsed: super::HooksConfig = serde_json::from_str("{\"enabled\":false}").unwrap();
    assert!(!parsed.enabled);
}

// 关闭插件钩子不应连带关闭整个运行时钩子系统。
#[test]
fn hooks_plugin_hooks_can_be_disabled() {
    let parsed: super::HooksConfig =
        serde_json::from_str("{\"plugins\":{\"enabled\":false}}").unwrap();

    assert!(parsed.enabled);
    assert!(!parsed.plugins.enabled);
}
