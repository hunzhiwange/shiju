//! 数据目录与日志的既有行为测试。

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("vw-webview-main-tests-{}-{name}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn append_log_creates_parent_and_appends_lines() {
    let dir = TestDir::new("log");
    let log_path = dir.path().join("logs/vw-webview.log");

    append_log(&log_path, "one");
    append_log(&log_path, "two");

    assert_eq!(fs::read_to_string(log_path).unwrap(), "one\ntwo\n");
}

#[test]
fn webview_data_dir_uses_webview_data_leaf() {
    assert_eq!(webview_data_dir().file_name().and_then(|s| s.to_str()), Some("webview_data"));
}

#[test]
fn fallback_webview_data_dir_uses_webview_data_leaf() {
    assert_eq!(
        fallback_webview_data_dir().file_name().and_then(|s| s.to_str()),
        Some("webview_data")
    );
}

#[test]
fn fallback_webview_data_dir_from_uses_base_or_current_dir() {
    assert_eq!(
        fallback_webview_data_dir_from(Some(Path::new("/tmp/data"))),
        PathBuf::from("/tmp/data/vibe-window/webview_data")
    );
    assert_eq!(
        fallback_webview_data_dir_from(None),
        std::env::current_dir().unwrap().join("webview_data")
    );
}
