//! Provider 聚合测试覆盖内置模型、用户配置、环境认证与远端发现合并后的最终查询视图。
//!
//! 用例分别检查设置态与运行态过滤、默认值和能力继承、别名建议、配置指纹失效及小模型选择；
//! 临时配置目录和环境守卫将进程缓存、凭据变量与文件变更隔离在单个串行测试场景中。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::sync::{Mutex, MutexGuard};

use crate::invalidate_cache;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

// 聚合查询依赖进程环境和两层缓存，fixture 持锁期间统一建立并在退出时恢复测试边界。
pub(crate) struct ProviderTestEnv {
    _lock: MutexGuard<'static, ()>,
    config_dir: PathBuf,
    saved_env: Vec<(String, Option<OsString>)>,
}

impl ProviderTestEnv {
    pub(crate) async fn new() -> Self {
        let lock = TEST_LOCK.lock().await;
        let root = test_root();
        let config_dir = fresh_dir(&root, "config");

        std::fs::create_dir_all(&config_dir).expect("config dir should be created");

        let mut saved_env = save_env(&[
            "VIBEWINDOW_TEST_HOME",
            "VIBEWINDOW_WORKSPACE",
            "TEST_PROVIDER_KEY",
            "SECOND_PROVIDER_KEY",
            "VIBEWINDOW_PROVIDER_KEY",
        ]);

        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", &config_dir);
            std::env::set_var("VIBEWINDOW_WORKSPACE", &config_dir);
            std::env::remove_var("TEST_PROVIDER_KEY");
            std::env::remove_var("SECOND_PROVIDER_KEY");
            std::env::remove_var("VIBEWINDOW_PROVIDER_KEY");
        }

        provider_model_cache::invalidate_cache();
        for name in provider_model_cache::get().await.values().flat_map(|provider| provider.env.iter()) {
            if saved_env.iter().any(|(saved, _)| saved == name) {
                continue;
            }
            saved_env.push((name.clone(), std::env::var_os(name)));
            unsafe { std::env::remove_var(name) };
        }
        invalidate_cache().await;

        Self { _lock: lock, config_dir, saved_env }
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        self.config_dir.join(".vibewindow.json")
    }

    pub(crate) fn set_env(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    pub(crate) async fn write_config(&self, value: Value) {
        write_json(&self.config_path(), &value);
        invalidate_cache().await;
    }
}

impl Drop for ProviderTestEnv {
    fn drop(&mut self) {
        for (key, value) in &self.saved_env {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn save_env(keys: &[&str]) -> Vec<(String, Option<OsString>)> {
    keys.iter().map(|key| ((*key).to_string(), std::env::var_os(key))).collect()
}

fn test_root() -> PathBuf {
    TEST_ROOT
        .get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            std::env::temp_dir()
                .join(format!("vw-provider-resolver-tests-{}-{nanos}", std::process::id()))
        })
        .clone()
}

fn fresh_dir(root: &Path, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    root.join(format!("{prefix}-{nanos}"))
}

pub(crate) fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("json parent dir should be created");
    }
    std::fs::write(path, serde_json::to_string_pretty(value).expect("json value should serialize"))
        .expect("json file should be written");
}

pub(crate) fn config_with_default(default_model: Value, providers: Value) -> Value {
    json!({
        "default_model": default_model,
        "default_temperature": 0.7,
        "providers": providers
    })
}

pub(crate) fn active_test_provider_config() -> Value {
    json!({
        "zz-test-provider": {
            "name": "Configured Provider",
            "api": "https://config.example/test",
            "env": ["TEST_PROVIDER_KEY"],
            "adapter": "acp",
            "options": {"timeout_seconds": 30},
            "models": {
                "alpha": {
                    "id": "alpha-api",
                    "name": "Alpha Config",
                    "family": "config-family",
                    "release_date": "2026-01-01",
                    "status": "active",
                    "headers": {"x-config": "config"},
                    "options": {"configured": true},
                    "limit": {"context": 32768, "input": 16384, "output": 8192},
                    "provider": {"adapter": "custom-adapter"}
                },
                "alias-a": {
                    "id": "shared-api",
                    "name": "Alias A",
                    "status": "active",
                    "limit": {"context": 1024, "output": 256}
                },
                "alias-b": {
                    "id": "shared-api",
                    "name": "Alias B",
                    "status": "active",
                    "limit": {"context": 1024, "output": 256}
                }
            }
        }
    })
}
