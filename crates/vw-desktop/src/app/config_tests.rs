// Tests for plan6 task 844.
const SOURCE: &str = include_str!("config.rs");

#[test]
fn config_tests_keeps_config_module_exports_explicit() {
    for expected in ["mod agent;", "mod desktop;", "mod gateway;", "mod system_settings;"] {
        assert!(SOURCE.contains(expected), "expected config source to keep declaration");
    }
}
