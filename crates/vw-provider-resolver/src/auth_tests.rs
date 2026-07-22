use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::Info;

static ENV_LOCK: Mutex<()> = Mutex::new(());
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn set_test_home(home: &Path) -> Self {
        let lock = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let saved = vec![("VIBEWINDOW_TEST_HOME", std::env::var_os("VIBEWINDOW_TEST_HOME"))];
        unsafe { std::env::set_var("VIBEWINDOW_TEST_HOME", home) };
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn unique_dir(prefix: &str) -> PathBuf {
    let root = TEST_ROOT
        .get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            std::env::temp_dir()
                .join(format!("vw-provider-auth-tests-{}-{nanos}", std::process::id()))
        })
        .clone();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    root.join(format!("{prefix}-{nanos}"))
}

fn assert_api_key(info: Option<Info>, expected: &str) {
    match info {
        Some(Info::Api(api)) => assert_eq!(api.key, expected),
        other => panic!("expected api info, got {other:?}"),
    }
}

#[test]
fn filepath_uses_global_paths_and_legacy_auth_location() {
    let home = unique_dir("home");
    let _guard = EnvGuard::set_test_home(&home);

    let path = super::filepath();

    assert!(path.ends_with(".vibewindow/auth.json") || path.ends_with("auth.json"));
}

/// 在当前进程解析出的鉴权文件路径上写入内容并在结束后恢复，避免污染本机文件。
struct AuthFileGuard {
    _env: EnvGuard,
    path: PathBuf,
    previous: Option<String>,
}

impl AuthFileGuard {
    fn write(contents: &str) -> Self {
        // `EnvGuard` 持有进程级 env 锁，保证读写 `filepath()` 期间无并发干扰。
        let home = unique_dir("records-home");
        let env = EnvGuard::set_test_home(&home);
        let path = super::filepath();
        let previous = std::fs::read_to_string(&path).ok();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("auth parent should be created");
        }
        std::fs::write(&path, contents).expect("auth file should be written");
        Self { _env: env, path, previous }
    }
}

impl Drop for AuthFileGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(prev) => {
                let _ = std::fs::write(&self.path, prev);
            }
            None => {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}

#[test]
fn get_and_all_read_auth_records_from_resolved_file() {
    let _guard = AuthFileGuard::write(
        &json!({
            "openai": {"type": "api", "key": "openai-key"},
            "bad": {"type": "api"}
        })
        .to_string(),
    );

    // `bad` 缺少 key 字段，反序列化失败被过滤，因此只剩一条记录。
    assert_api_key(super::get("openai"), "openai-key");
    assert!(super::get("missing").is_none());
    let all = super::all();
    assert_eq!(all.len(), 1);
    assert_api_key(all.get("openai").cloned(), "openai-key");
}

#[test]
fn get_and_all_return_empty_when_auth_file_invalid() {
    let _guard = AuthFileGuard::write("not-json");

    assert!(super::get("openai").is_none());
    assert!(super::all().is_empty());
}
