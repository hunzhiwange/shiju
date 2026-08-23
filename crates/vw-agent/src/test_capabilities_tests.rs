//! 验证测试能力探测结果与编译特性保持一致。
//! 环境变量读取由全局锁隔离，写权限探测必须清理探针且保留目录原有内容。

use super::{check_loopback_bind, check_writable_dir, home_dir_from_env};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct HomeEnvGuard {
    home: Option<OsString>,
    userprofile: Option<OsString>,
}

impl HomeEnvGuard {
    fn capture() -> Self {
        Self { home: std::env::var_os("HOME"), userprofile: std::env::var_os("USERPROFILE") }
    }

    fn set(key: &str, value: &OsStr) {
        // SAFETY: home environment mutations in this module are serialized by ENV_LOCK.
        unsafe { std::env::set_var(key, value) };
    }

    fn unset(key: &str) {
        // SAFETY: home environment mutations in this module are serialized by ENV_LOCK.
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        // SAFETY: the guard is dropped while its test still holds ENV_LOCK.
        unsafe {
            match &self.home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match &self.userprofile {
                Some(value) => std::env::set_var("USERPROFILE", value),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
    }
}

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn home_dir_from_env_resolves_priority_fallback_and_missing_values() {
    let _lock = lock_env();
    let _env = HomeEnvGuard::capture();

    HomeEnvGuard::set("HOME", OsStr::new("/tmp/vw-home"));
    HomeEnvGuard::set("USERPROFILE", OsStr::new("/tmp/vw-userprofile"));
    assert_eq!(home_dir_from_env().as_deref(), Some(Path::new("/tmp/vw-home")));

    HomeEnvGuard::unset("HOME");
    assert_eq!(home_dir_from_env().as_deref(), Some(Path::new("/tmp/vw-userprofile")));

    HomeEnvGuard::set("HOME", OsStr::new(""));
    assert_eq!(home_dir_from_env().as_deref(), Some(Path::new("/tmp/vw-userprofile")));

    HomeEnvGuard::set("USERPROFILE", OsStr::new(""));
    assert!(home_dir_from_env().is_none());

    HomeEnvGuard::unset("HOME");
    HomeEnvGuard::unset("USERPROFILE");
    assert!(home_dir_from_env().is_none());
}

#[cfg(unix)]
#[test]
fn home_dir_from_env_preserves_non_utf8_home_value() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = lock_env();
    let _env = HomeEnvGuard::capture();
    let home = OsString::from_vec(b"/tmp/vw-home-\xff".to_vec());

    HomeEnvGuard::set("HOME", &home);
    HomeEnvGuard::unset("USERPROFILE");

    assert_eq!(home_dir_from_env(), Some(PathBuf::from(home)));
}

#[test]
fn writable_dir_probe_creates_nested_directory_and_cleans_probe() {
    let temp = TempDir::new().expect("tempdir should create");
    let nested = temp.path().join("one").join("two");

    check_writable_dir(&nested).expect("temp directory should be writable");

    assert!(nested.is_dir());
    let leftovers =
        std::fs::read_dir(&nested).expect("nested directory should be readable").count();
    assert_eq!(leftovers, 0);
}

#[test]
// 写权限探测只能创建并删除自身探针，目录中的用户文件必须原样保留。
fn writable_dir_probe_preserves_existing_entries_and_cleans_probe() {
    let temp = TempDir::new().expect("tempdir should create");
    let sentinel = temp.path().join("sentinel.txt");
    std::fs::write(&sentinel, b"keep").expect("sentinel should write");

    check_writable_dir(temp.path()).expect("existing temp directory should be writable");

    assert_eq!(std::fs::read(&sentinel).expect("sentinel should remain readable"), b"keep");
    let entries =
        std::fs::read_dir(temp.path()).expect("temp directory should be readable").count();
    assert_eq!(entries, 1);
}

#[test]
fn writable_dir_reports_create_errors_for_file_path() {
    let temp = TempDir::new().expect("tempdir should create");
    let file_path = temp.path().join("not-a-dir");
    std::fs::write(&file_path, b"file").expect("file should write");

    let err = check_writable_dir(&file_path).expect_err("file path cannot become a directory");

    assert!(err.contains("无法为能力探测创建目录"));
    assert!(err.contains("not-a-dir"));
    assert_eq!(std::fs::read(file_path).expect("original file should remain"), b"file");
}

#[cfg(unix)]
#[test]
fn writable_dir_reports_probe_write_errors_for_read_only_directory() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("tempdir should create");
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o555))
        .expect("temp directory should become read-only");

    let result = check_writable_dir(temp.path());
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o755))
        .expect("temp directory permissions should restore");
    let err = result.expect_err("read-only directory should reject capability probe files");

    assert!(err.contains("无法写入探测文件"));
    assert!(err.contains(".vibewindow-capability-probe-"));
}

#[test]
fn loopback_bind_is_available_or_returns_contextual_error() {
    if let Err(err) = check_loopback_bind() {
        assert!(err.contains("回环地址绑定不可用"));
    }
}
