//! `vw-cli` 库层输入解析辅助函数的测试。
//! 覆盖模型温度的有限边界、首个非空行的 Unicode 空白处理，以及项目目录的规范化与类型检查。
//! 同时确认 ACP 子命令不暴露额外的命令专用参数。

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{first_non_empty_line, parse_temperature, resolve_project_dir};
use clap::Parser;

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
fn parse_temperature_accepts_inclusive_model_bounds() {
    assert_eq!(parse_temperature("0"), Ok(0.0));
    assert_eq!(parse_temperature("2.0"), Ok(2.0));
    assert_eq!(parse_temperature("0.75"), Ok(0.75));
}

#[test]
fn parse_temperature_rejects_values_outside_supported_range() {
    assert_eq!(
        parse_temperature("-0.01"),
        Err("temperature must be between 0.0 and 2.0".to_string())
    );
    assert_eq!(
        parse_temperature("2.01"),
        Err("temperature must be between 0.0 and 2.0".to_string())
    );
}

#[test]
fn parse_temperature_rejects_non_finite_float_spellings() {
    for value in ["NaN", "inf", "-inf"] {
        assert_eq!(
            parse_temperature(value),
            Err("temperature must be between 0.0 and 2.0".to_string()),
            "{value} must not be accepted as a temperature"
        );
    }
}

#[test]
fn parse_temperature_keeps_parser_whitespace_rules_explicit() {
    assert!(parse_temperature(" 1.0 ").is_err());
    assert_eq!(parse_temperature("1e0"), Ok(1.0));
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

#[test]
fn acp_command_has_no_command_specific_flags() {
    let parsed = super::cli::Cli::try_parse_from(["vibewindow", "acp"]).expect("ACP command");
    assert!(matches!(parsed.command, super::cli::Commands::Acp));
    assert!(super::cli::Cli::try_parse_from(["vibewindow", "acp", "--listen"]).is_err());
}
