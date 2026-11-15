//! 本地任务输入解析辅助函数的测试。
//! 覆盖首个非空行的 Unicode 空白处理，以及项目目录的规范化与类型检查。

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{first_non_empty_line, resolve_project_dir};

struct TestDir {
    path: std::path::PathBuf,
}

impl TestDir {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vw-cli-lib-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create lib test directory");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn first_non_empty_line_handles_crlf_and_unicode_whitespace() {
    assert_eq!(first_non_empty_line("\r\n\u{2003}\r\n  crab \r\nnext"), "crab");
}

#[test]
fn first_non_empty_line_returns_empty_when_content_has_no_text() {
    assert_eq!(first_non_empty_line(" \n\t\r\n"), "");
}

#[test]
fn resolve_project_dir_rejects_blank_input_before_filesystem_lookup() {
    let err = resolve_project_dir(" \n\t ").expect_err("blank project dir must fail");

    assert!(err.to_string().contains("--project-dir cannot be empty"));
}

#[test]
fn resolve_project_dir_returns_canonical_directory_path() {
    let test_dir = TestDir::new();
    let nested = test_dir.path.join("nested").join("project");
    fs::create_dir_all(&nested).expect("create nested project");
    let input = nested.join("..").join("project");

    let resolved = resolve_project_dir(&input.to_string_lossy()).expect("resolve project dir");

    assert_eq!(resolved, nested.canonicalize().unwrap().to_string_lossy());
}

#[test]
fn resolve_project_dir_rejects_existing_file() {
    let test_dir = TestDir::new();
    let file = test_dir.path.join("Cargo.toml");
    fs::write(&file, b"[package]\n").expect("create file");

    let err = resolve_project_dir(&file.to_string_lossy()).expect_err("file must fail");

    assert!(err.to_string().contains("project directory is not a folder"));
}
