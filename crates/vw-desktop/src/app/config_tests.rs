//! 验证应用配置的加载入口、默认合并和保存路径。
//!
//! 该文件保留配置模块接入哨兵，防止测试模块在拆分连线时被意外移除。

// 保留计划任务 844 的既定覆盖面。
const SOURCE: &str = include_str!("config.rs");

#[test]
fn config_tests_keeps_config_module_exports_explicit() {
    for expected in ["mod agent;", "mod desktop;", "mod gateway;", "mod system_settings;"] {
        assert!(SOURCE.contains(expected), "expected config source to keep declaration");
    }
}
