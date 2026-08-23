//! 验证版本解析、环境变量匹配与向上查找软件包版本的规则。

use super::*;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard};

static VERSION_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn env_lock() -> MutexGuard<'static, ()> {
    VERSION_ENV_TEST_LOCK.lock().expect("version env test lock should acquire")
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vwacp-version-{}-{name}", std::process::id()))
}

fn env_with(package_name: &str, version: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("npm_package_name".to_string(), package_name.to_string());
    env.insert("npm_package_version".to_string(), version.to_string());
    env
}

#[test]
fn parse_version_trims_non_empty_values() {
    assert_eq!(parse_version(Some("  1.2.3  ")), Some("1.2.3".to_string()));
    assert_eq!(parse_version(Some("\t\n")), None);
    assert_eq!(parse_version(None), None);
}

#[test]
fn env_version_accepts_vwacp_package_only() {
    let mut env = env_with("vwacp", "1.2.3");

    assert_eq!(
        resolve_vwacp_version(Some(ResolveAcpxVersionParams {
            env: Some(&env),
            package_json_path: None
        })),
        "1.2.3"
    );

    env.insert("npm_package_name".to_string(), "other".to_string());
    assert_eq!(env_version(Some(&env)), None);
}

#[test]
fn env_version_accepts_cargo_package_name_alias() {
    let env = env_with(env!("CARGO_PKG_NAME"), "2.3.4");

    assert_eq!(env_version(Some(&env)), Some("2.3.4".to_string()));
}

#[test]
fn env_version_rejects_missing_or_blank_fields() {
    let env = HashMap::new();
    assert_eq!(env_version(Some(&env)), None);

    let env = env_with("vwacp", "   ");
    assert_eq!(env_version(Some(&env)), None);

    let env = env_with("   ", "1.0.0");
    assert_eq!(env_version(Some(&env)), None);
}

#[test]
fn env_version_reads_process_environment() {
    let _lock = env_lock();
    let _name = EnvGuard::set("npm_package_name", "vwacp");
    let _version = EnvGuard::set("npm_package_version", "7.8.9");

    assert_eq!(env_version(None), Some("7.8.9".to_string()));
}

#[test]
fn env_version_rejects_process_environment_other_package() {
    let _lock = env_lock();
    let _name = EnvGuard::set("npm_package_name", "other");
    let _version = EnvGuard::set("npm_package_version", "7.8.9");

    assert_eq!(env_version(None), None);
}

#[test]
fn read_package_version_rejects_blank_or_missing_version() {
    let path = temp_path("package.json");
    fs::write(&path, r#"{ "version": "  2.0.0-beta  " }"#).unwrap();
    assert_eq!(read_package_version(&path), Some("2.0.0-beta".to_string()));

    fs::write(&path, r#"{ "version": "   " }"#).unwrap();
    assert_eq!(read_package_version(&path), None);

    fs::write(&path, r#"{ "version": 7 }"#).unwrap();
    assert_eq!(read_package_version(&path), None);

    fs::write(&path, r#"{ "name": "vwacp" }"#).unwrap();
    assert_eq!(read_package_version(&path), None);

    fs::write(&path, r#"{ "version": "#).unwrap();
    assert_eq!(read_package_version(&path), None);

    let _ = fs::remove_file(path);
}

#[test]
fn read_package_version_returns_none_for_missing_file() {
    assert_eq!(read_package_version(&temp_path("missing-package.json")), None);
}

#[test]
fn resolve_version_from_ancestors_finds_nearest_valid_package() {
    let root = temp_path("ancestors");
    let child = root.join("child");
    let grandchild = child.join("grandchild");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&grandchild).unwrap();

    fs::write(root.join("package.json"), r#"{ "version": "3.4.5" }"#).unwrap();
    assert_eq!(resolve_version_from_ancestors(&grandchild), Some("3.4.5".to_string()));

    fs::write(child.join("package.json"), r#"{ "version": "4.5.6" }"#).unwrap();
    assert_eq!(resolve_version_from_ancestors(&grandchild), Some("4.5.6".to_string()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_version_from_ancestors_returns_none_without_valid_package() {
    let root = temp_path("no-ancestors");
    let child = root.join("child");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&child).unwrap();
    fs::write(root.join("package.json"), r#"{ "version": " " }"#).unwrap();

    assert_eq!(resolve_version_from_ancestors(&child), None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn resolve_vwacp_version_prefers_env_over_package_path() {
    let path = temp_path("env-wins-package.json");
    let env = env_with("vwacp", "5.6.7");
    fs::write(&path, r#"{ "version": "9.9.9" }"#).unwrap();

    assert_eq!(
        resolve_vwacp_version(Some(ResolveAcpxVersionParams {
            env: Some(&env),
            package_json_path: Some(&path),
        })),
        "5.6.7"
    );

    let _ = fs::remove_file(path);
}

#[test]
fn resolve_vwacp_version_uses_package_path_or_unknown() {
    let path = temp_path("explicit-package.json");
    let env = env_with("other", "5.6.7");
    fs::write(&path, r#"{ "version": "6.7.8" }"#).unwrap();

    assert_eq!(
        resolve_vwacp_version(Some(ResolveAcpxVersionParams {
            env: Some(&env),
            package_json_path: Some(&path),
        })),
        "6.7.8"
    );

    fs::write(&path, r#"{ "version": " " }"#).unwrap();
    assert_eq!(
        resolve_vwacp_version(Some(ResolveAcpxVersionParams {
            env: Some(&env),
            package_json_path: Some(&path),
        })),
        UNKNOWN_VERSION
    );

    let _ = fs::remove_file(path);
}

#[test]
fn resolve_vwacp_version_without_params_uses_ancestor_package_or_unknown() {
    let _lock = env_lock();
    let _name = EnvGuard::unset("npm_package_name");
    let _version = EnvGuard::unset("npm_package_version");
    let expected = resolve_version_from_ancestors(&PathBuf::from(MODULE_DIR))
        .unwrap_or_else(|| UNKNOWN_VERSION.to_string());

    assert_eq!(resolve_vwacp_version(None), expected);
}

#[test]
fn get_vwacp_version_returns_cached_resolved_value() {
    let first = get_vwacp_version();
    let second = get_vwacp_version();

    assert_eq!(first, second);
    assert!(parse_version(Some(&first)).is_some());
}
