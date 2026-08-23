// 覆盖配置目录错误提示及 Codex 凭据文件的安全读取。

use super::config_helpers::config_dir_creation_error;
use super::config_helpers::read_codex_openai_api_key;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

struct HomeEnvGuard {
    old_home: Option<String>,
}

impl HomeEnvGuard {
    fn set(path: &Path) -> Self {
        let old_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", path);
        }

        Self { old_home }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(old_home) = &self.old_home {
                std::env::set_var("HOME", old_home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}

fn write_codex_auth(home: &Path, content: &str) {
    let codex_dir = home.join(".codex");
    fs::create_dir_all(&codex_dir).unwrap();
    fs::write(codex_dir.join("auth.json"), content).unwrap();
}

#[test]
fn config_dir_creation_error_mentions_path_and_openrc_hint() {
    let message = config_dir_creation_error(std::path::Path::new("/restricted/vw"));

    assert!(message.contains("/restricted/vw"));
    assert!(message.contains("OpenRC"));
}

#[test]
fn read_codex_openai_api_key_returns_none_when_auth_file_is_absent() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_none_when_auth_path_is_directory() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    fs::create_dir_all(tmp.path().join(".codex").join("auth.json")).unwrap();

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_trimmed_key_from_auth_file() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), r#"{ "OPENAI_API_KEY": "  sk-test-key  " }"#);

    assert_eq!(read_codex_openai_api_key(), Some("sk-test-key".to_string()));
}

#[test]
fn read_codex_openai_api_key_returns_none_for_blank_key() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), r#"{ "OPENAI_API_KEY": " \n\t " }"#);

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_none_for_invalid_json() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), "{");

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_none_when_json_root_is_not_object() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), r#"["sk-test-key"]"#);

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_none_when_key_is_missing() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), r#"{ "other": "sk-test-key" }"#);

    assert!(read_codex_openai_api_key().is_none());
}

#[test]
fn read_codex_openai_api_key_returns_none_when_key_is_not_a_string() {
    let _guard = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeEnvGuard::set(tmp.path());
    write_codex_auth(tmp.path(), r#"{ "OPENAI_API_KEY": 123 }"#);

    assert!(read_codex_openai_api_key().is_none());
}
