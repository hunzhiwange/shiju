//! 隔离模型恢复测试配置，不读取生产凭据。
use std::ffi::OsString;

pub(crate) struct RecoveryConfig {
    _home: tempfile::TempDir,
    previous: Vec<(&'static str, Option<OsString>)>,
}
impl RecoveryConfig {
    pub(crate) async fn new(api: &str, key: Option<&str>) -> Self {
        let home = tempfile::tempdir().unwrap();
        let previous =
            ["HOME", "VIBEWINDOW_CONFIG_DIR", "VIBEWINDOW_WORKSPACE", "VW_MODEL_RECOVERY_TEST_KEY"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect();
        unsafe {
            std::env::set_var("HOME", home.path());
            match key {
                Some(key) => std::env::set_var("VW_MODEL_RECOVERY_TEST_KEY", key),
                None => std::env::remove_var("VW_MODEL_RECOVERY_TEST_KEY"),
            }
            std::env::set_var("VIBEWINDOW_CONFIG_DIR", home.path().join("config"));
            std::env::remove_var("VIBEWINDOW_WORKSPACE");
        }
        let fixture = Self { _home: home, previous };
        crate::config::update(serde_json::json!({"providers": {"model-recovery": {
            "api": api, "env": ["VW_MODEL_RECOVERY_TEST_KEY"], "models": {"original": {"status": "active"}, "disabled": {"status": "inactive"}}
        }}})).await.unwrap();
        crate::provider::provider::invalidate_cache().await;
        fixture
    }
}
impl Drop for RecoveryConfig {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
