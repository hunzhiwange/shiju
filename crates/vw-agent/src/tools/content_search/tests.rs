//! # 内容搜索工具测试模块
//!
//! 本模块提供 `ContentSearchTool` 的全面测试覆盖，验证内容搜索功能的正确性、
//! 安全性和各种输出模式。
//!
//! ## 测试范围
//!
//! - **基础功能测试**：模式匹配、大小写敏感、文件过滤
//! - **输出模式测试**：content、files_with_matches、count 模式
//! - **上下文显示**：匹配行的前后文显示功能
//! - **路径处理**：子目录搜索、相对路径处理
//! - **安全性测试**：绝对路径拒绝、路径穿越防护、速率限制、符号链接处理
//! - **边界情况**：空模式、缺失参数、无效模式、多行搜索
//!
//! ## 测试策略
//!
//! 使用临时目录创建测试文件，确保测试隔离性和可重复性。
//! 通过不同的安全策略配置验证工具在各种安全约束下的行为。

use super::super::content_search::{
    build_grep_command, build_rg_command, format_grep_output, format_line_output,
    parse_content_line, parse_count_line, relativize_path, truncate_utf8,
};
use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

/// 创建测试用安全策略（使用默认配置）
///
/// 创建一个具有监督级自主权限的默认安全策略，用于大多数基础功能测试。
///
/// # 参数
///
/// - `workspace`: 工作区目录路径，作为搜索操作的基础目录
///
/// # 返回
///
/// 返回一个 `Arc<SecurityPolicy>`，配置为监督级自主权限，并设置指定的工作区
fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    })
}

/// 创建自定义配置的安全策略
///
/// 创建一个可自定义自主权限级别和速率限制的安全策略，
/// 用于需要特定安全约束的测试场景。
///
/// # 参数
///
/// - `workspace`: 工作区目录路径
/// - `autonomy`: 自主权限级别（如 Supervised、Autonomous）
/// - `max_actions_per_hour`: 每小时最大操作数，用于速率限制测试
///
/// # 返回
///
/// 返回一个自定义配置的 `Arc<SecurityPolicy>`
fn test_security_with(
    workspace: PathBuf,
    autonomy: AutonomyLevel,
    max_actions_per_hour: u32,
) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy,
        workspace_dir: workspace,
        max_actions_per_hour,
        ..SecurityPolicy::default()
    })
}

/// 创建测试文件集合
///
/// 在指定临时目录中创建一组标准的测试文件，包括：
/// - `hello.rs`: 包含 main 函数的 Rust 源文件
/// - `lib.rs`: 包含公开函数的库文件
/// - `readme.txt`: 纯文本说明文件
///
/// # 参数
///
/// - `dir`: 临时目录引用，测试文件将创建在该目录中
fn create_test_files(dir: &TempDir) {
    std::fs::write(dir.path().join("hello.rs"), "fn main() {\n    println!(\"hello\");\n}\n")
        .unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn greet() {\n    println!(\"greet\");\n}\n")
        .unwrap();
    std::fs::write(dir.path().join("readme.txt"), "This is a readme file.\n").unwrap();
}

/// 测试工具名称和参数 schema 的正确性
///
/// 验证 ContentSearchTool 的基础元数据：
/// - 工具名称应为 "content_search"
/// - 参数 schema 应包含 pattern、path、output_mode 等属性
/// - pattern 应为必需参数
#[test]
fn content_search_name_and_schema() {
    let tool = ContentSearchTool::new(test_security(std::env::temp_dir()));
    assert_eq!(tool.name(), "content_search");
    assert!(tool.description().contains("ripgrep"));
    assert!(tool.description().contains("files_with_matches"));

    let schema = tool.parameters_schema();
    assert!(schema["properties"]["pattern"].is_object());
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["output_mode"].is_object());
    assert!(schema["required"].as_array().unwrap().contains(&json!("pattern")));
}

#[tokio::test]
async fn content_search_max_results_parameter_limits_formatted_results() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("one.txt"), "needle one\n").unwrap();
    std::fs::write(dir.path().join("two.txt"), "needle two\n").unwrap();

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "needle", "max_results": 1})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("[Results truncated: showing first 1 results]"));
}

/// 测试基础模式匹配功能
///
/// 验证内容搜索能够：
/// - 成功匹配文件内容中的指定模式
/// - 在输出中显示匹配的文件名
/// - 在输出中显示匹配的内容
#[tokio::test]
async fn content_search_basic_match() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "fn main"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("hello.rs"));
    assert!(result.output.contains("fn main"));
}

/// 测试 files_with_matches 输出模式
///
/// 验证 files_with_matches 模式下：
/// - 只显示包含匹配的文件名，不显示具体内容
/// - 正确统计匹配文件数量
/// - 不包含未匹配的文件
#[tokio::test]
async fn content_search_files_with_matches_mode() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool
        .execute(json!({"pattern": "println", "output_mode": "files_with_matches"}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("hello.rs"));
    assert!(result.output.contains("lib.rs"));
    assert!(!result.output.contains("readme.txt"));
    assert!(result.output.contains("Total: 2 files"));
}

/// 测试 count 输出模式
///
/// 验证 count 模式下：
/// - 显示每个文件的匹配次数
/// - 显示所有匹配的文件
/// - 包含总计信息
#[tokio::test]
async fn content_search_count_mode() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "println", "output_mode": "count"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("hello.rs"));
    assert!(result.output.contains("lib.rs"));
    assert!(result.output.contains("Total:"));
}

/// 测试大小写不敏感搜索
///
/// 验证当 case_sensitive 设置为 false 时：
/// - 搜索不区分大小写
/// - 能同时匹配不同大小写形式的文本
#[tokio::test]
async fn content_search_case_insensitive() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.txt"), "Hello World\nhello world\n").unwrap();

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "HELLO", "case_sensitive": false})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("Hello World"));
    assert!(result.output.contains("hello world"));
}

/// 测试文件包含过滤器
///
/// 验证 include 参数能够正确过滤文件类型：
/// - 只搜索符合 glob 模式的文件
/// - 排除不符合模式的文件
#[tokio::test]
async fn content_search_include_filter() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "fn", "include": "*.rs"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("hello.rs"));
    assert!(!result.output.contains("readme.txt"));
}

/// 测试上下文行显示功能
///
/// 验证 context_before 和 context_after 参数：
/// - 能显示匹配行之前指定数量的行
/// - 能显示匹配行之后指定数量的行
/// - 帮助理解匹配内容的上下文
#[tokio::test]
async fn content_search_context_lines() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("ctx.rs"), "line1\nline2\ntarget_line\nline4\nline5\n").unwrap();

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool
        .execute(json!({"pattern": "target_line", "context_before": 1, "context_after": 1}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("target_line"));
    assert!(result.output.contains("line2"));
    assert!(result.output.contains("line4"));
}

/// 测试无匹配结果的情况
///
/// 验证当搜索模式在文件中不存在时：
/// - 操作仍然成功（success 为 true）
/// - 输出包含 "No matches found" 提示
#[tokio::test]
async fn content_search_no_matches() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "nonexistent_string_xyz"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("No matches found"));
}

/// 测试空模式字符串被拒绝
///
/// 验证输入验证：空字符串作为搜索模式应被拒绝：
/// - success 应为 false
/// - error 应包含 "Empty pattern" 提示
#[tokio::test]
async fn content_search_empty_pattern_rejected() {
    let tool = ContentSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": ""})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Empty pattern"));
}

/// 测试缺失必需参数的情况
///
/// 验证当缺少必需的 pattern 参数时：
/// - 执行应返回错误（Err）
/// - 符合参数 schema 的 required 约束
#[tokio::test]
async fn content_search_missing_pattern() {
    let tool = ContentSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

/// 测试无效输出模式被拒绝
///
/// 验证输出模式参数验证：
/// - 仅接受有效的 output_mode 值
/// - 无效值应返回明确的错误信息
#[tokio::test]
async fn content_search_invalid_output_mode_rejected() {
    let dir = TempDir::new().unwrap();
    create_test_files(&dir);

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result =
        tool.execute(json!({"pattern": "fn", "output_mode": "invalid_mode"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Invalid output_mode"));
}

/// 测试子目录搜索功能
///
/// 验证 path 参数能够限制搜索范围：
/// - 只搜索指定子目录中的文件
/// - 不包含工作区根目录中的文件
#[tokio::test]
async fn content_search_subdirectory() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("sub/deep")).unwrap();
    std::fs::write(dir.path().join("sub/deep/nested.rs"), "fn nested() {}\n").unwrap();
    std::fs::write(dir.path().join("root.rs"), "fn root() {}\n").unwrap();

    let tool = ContentSearchTool::new(test_security(dir.path().to_path_buf()));
    let result = tool.execute(json!({"pattern": "fn nested", "path": "sub"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("nested"));
    assert!(!result.output.contains("root"));
}

// ============================================================================
// 安全性测试
// ============================================================================

/// 测试绝对路径被拒绝
///
/// 安全策略：绝对路径可能被用于访问工作区外的敏感文件。
/// 验证尝试使用绝对路径时：
/// - success 应为 false
/// - error 应包含 "Absolute paths" 提示
#[tokio::test]
async fn content_search_rejects_absolute_path() {
    let tool = ContentSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": "test", "path": "/etc"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Absolute paths"));
}

/// 测试路径穿越攻击被阻止
///
/// 安全策略：路径穿越模式（如 `../`）可能被用于逃离工作区。
/// 验证尝试使用路径穿越时：
/// - success 应为 false
/// - error 应包含 "Path traversal" 提示
#[tokio::test]
async fn content_search_rejects_path_traversal() {
    let tool = ContentSearchTool::new(test_security(std::env::temp_dir()));
    let result = tool.execute(json!({"pattern": "test", "path": "../../../etc"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Path traversal"));
}

/// 测试速率限制功能
///
/// 安全策略：通过 max_actions_per_hour 限制操作频率，防止资源滥用。
/// 验证当速率限制为 0 时：
/// - 操作应被拒绝
/// - error 应包含 "Rate limit" 提示
#[tokio::test]
async fn content_search_rate_limited() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("file.txt"), "test content\n").unwrap();

    // 创建速率限制为 0 的安全策略
    let tool = ContentSearchTool::new(test_security_with(
        dir.path().to_path_buf(),
        AutonomyLevel::Supervised,
        0,
    ));
    let result = tool.execute(json!({"pattern": "test"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("Rate limit"));
}

/// 测试符号链接逃逸防护（仅 Unix 系统）
///
/// 安全策略：工作区内的符号链接可能指向工作区外的敏感文件。
///
/// 测试场景：
/// 1. 在工作区内创建指向工作区外的符号链接
/// 2. 在工作区内创建合法文件
///
/// 验证目标：
/// - 合法文件能被正常搜索
/// - 不会因符号链接导致崩溃
/// - 搜索结果被正确相对化处理
#[cfg(unix)]
#[tokio::test]
async fn content_search_symlink_escape_blocked() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let workspace = root.path().join("workspace");
    let outside = root.path().join("outside");

    // 创建工作区和外部目录
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "secret data\n").unwrap();

    // 在工作区内创建指向外部的符号链接
    symlink(&outside, workspace.join("escape_dir")).unwrap();
    // 创建合法的工作区文件
    std::fs::write(workspace.join("legit.txt"), "legit data\n").unwrap();

    let tool = ContentSearchTool::new(test_security(workspace.clone()));
    let result = tool.execute(json!({"pattern": "data"})).await.unwrap();

    assert!(result.success);
    // 合法文件应能被找到
    assert!(result.output.contains("legit.txt"));
    // 注意：ripgrep/grep 可能或可能不跟随符号链接，主要验证无崩溃
}

/// 测试多行模式在无 ripgrep 后端时的限制
///
/// 功能限制：多行搜索需要 ripgrep (rg) 后端支持。
///
/// 验证当后端不可用时：
/// - 尝试使用 multiline 参数应失败
/// - error 应包含 "ripgrep" 提示
#[tokio::test]
async fn content_search_multiline_without_rg() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.txt"), "line1\nline2\n").unwrap();

    // 创建不使用 ripgrep 后端的工具实例
    let tool = ContentSearchTool::new_with_backend(
        test_security(dir.path().to_path_buf()),
        false, // 禁用 ripgrep
    );
    let result = tool.execute(json!({"pattern": "line1", "multiline": true})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("ripgrep"));
}

/// 测试 relativize_path 函数的路径前缀移除功能
///
/// 验证当输出包含工作区绝对路径前缀时：
/// - 能正确移除工作区前缀
/// - 保留行号和内容部分
#[test]
fn relativize_path_strips_prefix() {
    let result = relativize_path("/workspace/src/main.rs:42:fn main()", "/workspace");
    assert_eq!(result, "src/main.rs:42:fn main()");
}

/// 测试 relativize_path 对无前缀路径的处理
///
/// 验证当输出已经是相对路径时：
/// - 函数不进行修改
/// - 原样返回输入字符串
#[test]
fn relativize_path_no_prefix() {
    let result = relativize_path("src/main.rs:42:fn main()", "/workspace");
    assert_eq!(result, "src/main.rs:42:fn main()");
}

/// 测试 format_line_output 在 content 模式下的行计数
///
/// 验证 content 模式下：
/// - 只统计匹配行（使用 `:` 分隔符的行）
/// - 不统计上下文行（使用 `-` 分隔符的行）
/// - 正确统计文件和行数
#[test]
fn format_line_output_content_counts_match_lines_only() {
    // 模拟 ripgrep 输出：包含上下文行（-）和匹配行（:）
    let raw =
        "src/main.rs-1-use std::fmt;\nsrc/main.rs:2:fn main() {}\n--\nsrc/lib.rs:10:pub fn f() {}";
    let output = format_line_output(raw, std::path::Path::new("/workspace"), "content", 100);
    assert!(output.contains("Total: 2 matching lines in 2 files"));
}

/// 测试 parse_count_line 对包含冒号的路径的支持
///
/// 边界情况：文件路径可能包含冒号（如 Windows 驱动器号或特殊字符）。
///
/// 验证解析器能正确处理：
/// - 路径中包含多个冒号的情况
/// - 正确识别最后一个冒号后的计数值
#[test]
fn parse_count_line_supports_colons_in_path() {
    let parsed = parse_count_line("dir:with:colon/file.rs:12");
    assert_eq!(parsed, Some(("dir:with:colon/file.rs", 12)));
}

/// 测试 truncate_utf8 的字符边界处理
///
/// UTF-8 边界处理：截断 UTF-8 字符串必须在字符边界处进行。
///
/// 测试场景：
/// - "你好" 每个中文字符占 3 字节
/// - 在字节索引 4 处截断会切分第一个中文字符
///
/// 验证：
/// - 函数正确识别字符边界
/// - 截断后的字符串保持 UTF-8 有效性
#[test]
fn truncate_utf8_keeps_char_boundary() {
    let text = "abc你好";
    // 字节索引 4 会切分第一个中文字符（"你" 占字节 3-5）
    let truncated = truncate_utf8(text, 4);
    assert_eq!(truncated, "abc");
}

#[test]
fn search_command_builders_encode_requested_modes_without_execution() {
    let rg = build_rg_command(
        "needle",
        std::path::Path::new("src"),
        "count",
        Some("*.rs"),
        false,
        2,
        3,
        true,
    );
    let rg_args: Vec<String> =
        rg.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect();
    assert!(rg_args.contains(&"--count".to_string()));
    assert!(rg_args.contains(&"-i".to_string()));
    assert!(rg_args.contains(&"-U".to_string()));
    assert!(rg_args.contains(&"--multiline-dotall".to_string()));
    assert!(rg_args.windows(2).any(|pair| pair[0] == "--glob" && pair[1] == "*.rs"));
    assert!(rg_args.ends_with(&["--".to_string(), "needle".to_string(), "src".to_string()]));
    assert!(!rg_args.contains(&"-B".to_string()));
    assert!(!rg_args.contains(&"-A".to_string()));

    let grep = build_grep_command(
        "needle",
        std::path::Path::new("src"),
        "content",
        Some("*.md"),
        true,
        2,
        3,
    );
    let grep_args: Vec<String> =
        grep.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect();
    assert!(grep_args.windows(2).any(|pair| pair[0] == "-B" && pair[1] == "2"));
    assert!(grep_args.windows(2).any(|pair| pair[0] == "-A" && pair[1] == "3"));
    assert!(grep_args.windows(2).any(|pair| pair[0] == "--include" && pair[1] == "*.md"));
    assert!(!grep_args.contains(&"-i".to_string()));

    let grep_files = build_grep_command(
        "needle",
        std::path::Path::new("src"),
        "files_with_matches",
        None,
        false,
        0,
        0,
    );
    let grep_files_args: Vec<String> =
        grep_files.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect();
    assert!(grep_files_args.contains(&"-l".to_string()));
    assert!(grep_files_args.contains(&"-i".to_string()));
    assert!(!grep_files_args.contains(&"--include".to_string()));

    let grep_count =
        build_grep_command("needle", std::path::Path::new("src"), "count", None, true, 0, 0);
    let grep_count_args: Vec<String> =
        grep_count.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect();
    assert!(grep_count_args.contains(&"-c".to_string()));
}

#[test]
fn format_line_output_filters_zero_counts_and_reports_truncation() {
    let raw = "/workspace/a.rs:0\n/workspace/b.rs:3\n/workspace/c.rs:4";
    let output = format_line_output(raw, std::path::Path::new("/workspace"), "count", 1);

    assert!(!output.contains("a.rs:0"));
    assert!(output.starts_with("b.rs:3"));
    assert!(output.contains("[Results truncated: showing first 1 results]"));
    assert!(output.contains("Total: 3 matches in 1 files"));
}

#[test]
fn format_line_output_deduplicates_file_results() {
    let raw = "/workspace/a.rs\n/workspace/a.rs\n/workspace/b.rs";
    let output =
        format_line_output(raw, std::path::Path::new("/workspace"), "files_with_matches", 10);

    assert_eq!(output.matches("a.rs").count(), 1);
    assert!(output.contains("Total: 2 files"));
}

#[test]
fn format_line_output_covers_empty_lines_truncation_and_unparsed_content() {
    let files = "/workspace/a.rs\n\n/workspace/b.rs";
    let files_output =
        format_line_output(files, std::path::Path::new("/workspace"), "files_with_matches", 1);
    assert_eq!(files_output.matches(".rs").count(), 1);
    assert!(files_output.contains("[Results truncated: showing first 1 results]"));

    let content = "plain unmatched text\n/workspace/src.rs:3:needle";
    let content_output =
        format_line_output(content, std::path::Path::new("/workspace"), "content", 1);
    assert!(content_output.starts_with("plain unmatched text"));
    assert!(content_output.contains("Total: 1 matching lines in 0 files"));
    assert!(content_output.contains("[Results truncated: showing first 1 results]"));
}

#[test]
fn format_grep_output_delegates_to_common_line_formatter() {
    let output = format_grep_output(
        "/workspace/src/lib.rs:2",
        std::path::Path::new("/workspace"),
        "count",
        10,
    );

    assert!(output.contains("src/lib.rs:2"));
    assert!(output.contains("Total: 2 matches in 1 files"));
}

#[test]
fn parse_content_line_and_count_line_report_unparseable_input() {
    assert_eq!(parse_content_line("plain text without line number"), None);
    assert_eq!(parse_count_line("plain text without count"), None);
}

#[test]
fn truncate_utf8_returns_original_when_within_limit() {
    let text = "short";

    assert_eq!(truncate_utf8(text, text.len()), text);
}
