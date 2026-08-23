//! 验证路径形态识别与用户主目录展开的环境边界。

use super::*;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().expect("HOME environment lock")
}

struct HomeEnvGuard(Option<OsString>);

impl HomeEnvGuard {
    fn unset() -> Self {
        let original = std::env::var_os("HOME");
        unsafe { std::env::remove_var("HOME") };
        Self(original)
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(home) => unsafe { std::env::set_var("HOME", home) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

#[test]
fn looks_like_path_detects_absolute_relative_and_home_paths() {
    assert!(looks_like_path("/tmp/file"));
    assert!(looks_like_path("./file"));
    assert!(looks_like_path("~/file"));
    assert!(looks_like_path("../file"));
    assert!(looks_like_path("."));
    assert!(looks_like_path(".."));
    assert!(looks_like_path("src/main.rs"));
    assert!(!looks_like_path("not-a-path"));
}

#[test]
fn home_dir_and_expand_user_path_follow_home_environment() {
    let _lock = env_lock();
    let home = home_dir().expect("HOME should be available");

    assert_eq!(expand_user_path("~"), home);
    assert_eq!(expand_user_path("~/project"), home.join("project"));
    assert_eq!(expand_user_path("~other/project"), std::path::PathBuf::from("~other/project"));
    assert_eq!(expand_user_path("/tmp/project"), std::path::PathBuf::from("/tmp/project"));
}

#[test]
fn home_paths_remain_literal_when_home_is_unset() {
    let _lock = env_lock();
    let _home = HomeEnvGuard::unset();

    assert_eq!(home_dir(), None);
    assert_eq!(expand_user_path("~"), PathBuf::from("~"));
    assert_eq!(expand_user_path("~/project"), PathBuf::from("~/project"));
}
