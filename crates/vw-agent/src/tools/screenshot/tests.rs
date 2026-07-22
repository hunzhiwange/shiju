//! 截图工具测试模块
//!
//! 本模块包含对 `ScreenshotTool` 的全面测试用例，验证以下方面：
//! - 工具基本属性（名称、描述、参数 schema）
//! - 跨平台截图命令生成
//! - 文件名安全清理（防止路径遍历攻击）
//! - Shell 注入防护
//! - 符号链接攻击防护
//!
//! 这些测试确保截图工具在各种边界情况下都能安全可靠地运行。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use std::path::Path;

/// 创建跨平台符号链接
///
/// 在 Unix 系统上使用标准符号链接，在 Windows 上使用文件符号链接。
/// 这是一个辅助函数，用于测试符号链接相关的安全防护。
///
/// # 参数
///
/// - `src` - 源文件路径
/// - `dst` - 目标符号链接路径
///
/// # Panics
///
/// 如果符号链接创建失败，将会 panic（仅在测试环境中使用）
#[cfg(unix)]
fn symlink_file(src: &Path, dst: &Path) {
    std::os::unix::fs::symlink(src, dst).expect("symlink should be created");
}

/// 创建跨平台符号链接（Windows 版本）
///
/// 在 Windows 上使用 `symlink_file` 创建文件符号链接。
/// 这是一个辅助函数，用于测试符号链接相关的安全防护。
///
/// # 参数
///
/// - `src` - 源文件路径
/// - `dst` - 目标符号链接路径
///
/// # Panics
///
/// 如果符号链接创建失败，将会 panic（仅在测试环境中使用）
#[cfg(windows)]
fn symlink_file(src: &Path, dst: &Path) {
    std::os::windows::fs::symlink_file(src, dst).expect("symlink should be created");
}

/// 创建测试用的安全策略
///
/// 生成一个具有完全自主权限的默认安全策略，用于测试环境。
/// 工作目录设置为系统临时目录，确保测试不会影响实际工作区。
///
/// # 返回值
///
/// 返回一个 `Arc<SecurityPolicy>` 实例，可以直接用于创建工具实例
fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

/// 测试截图工具的名称是否正确
///
/// 验证 `ScreenshotTool` 实例返回的工具名称为 "screenshot"，
/// 这是工具在系统中注册和调用的标识符。
#[test]
fn screenshot_tool_name() {
    let tool = ScreenshotTool::new(test_security());
    assert_eq!(tool.name(), "screenshot");
}

/// 测试截图工具的描述是否有效
///
/// 验证工具描述满足以下条件：
/// - 描述不为空
/// - 描述中包含 "screenshot" 关键字，确保描述与功能相关
#[test]
fn screenshot_tool_description() {
    let tool = ScreenshotTool::new(test_security());
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("screenshot"));
}

/// 测试截图工具的参数 schema 是否正确
///
/// 验证工具的 JSON Schema 包含必要的参数定义：
/// - `filename` 参数：指定输出文件名
/// - `region` 参数：指定截图区域
///
/// 这些参数定义用于工具的参数验证和文档生成
#[test]
fn screenshot_tool_schema() {
    let tool = ScreenshotTool::new(test_security());
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["filename"].is_object());
    assert!(schema["properties"]["region"].is_object());
}

/// 测试截图工具的完整规范
///
/// 验证 `spec()` 方法返回的 `ToolSpec` 包含：
/// - 正确的工具名称 "screenshot"
/// - 有效的参数 schema（JSON 对象格式）
///
/// 这个规范用于工具注册和发现
#[test]
fn screenshot_tool_spec() {
    let tool = ScreenshotTool::new(test_security());
    let spec = tool.spec();
    assert_eq!(spec.name, "screenshot");
    assert!(spec.parameters.is_object());
}

/// 测试截图命令是否在 macOS/Linux 上可用
///
/// 验证 `screenshot_commands()` 方法能够生成有效的截图命令列表：
/// - 命令列表不为空（至少有一个可用命令）
/// - 每个命令都是非空字符串
///
/// 该测试仅在 macOS 和 Linux 平台上运行
#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn screenshot_command_exists() {
    let commands = ScreenshotTool::screenshot_commands("/tmp/test.png");
    assert!(!commands.is_empty());
    assert!(commands.iter().all(|cmd| !cmd.is_empty()));
}

/// 测试文件名清理功能对路径遍历攻击的防护
///
/// 验证 `sanitize_output_filename()` 方法能够正确清理包含危险路径段的文件名：
/// - `../outside.png` 应被清理为 `outside.png`（移除上级目录引用）
/// - `..` 应回退到默认文件名（单个点无效）
/// - `.` 应回退到默认文件名（当前目录引用无效）
///
/// 这防止了通过文件名进行路径遍历攻击
#[test]
fn screenshot_filename_sanitizes_dot_segments() {
    let fallback = "fallback.png";
    assert_eq!(ScreenshotTool::sanitize_output_filename("../outside.png", fallback), "outside.png");
    assert_eq!(ScreenshotTool::sanitize_output_filename("..", fallback), fallback);
    assert_eq!(ScreenshotTool::sanitize_output_filename(".", fallback), fallback);
}

#[test]
fn screenshot_filename_trims_basename_and_falls_back_for_invalid_values() {
    let fallback = "fallback.png";

    assert_eq!(
        ScreenshotTool::sanitize_output_filename("nested/  capture.png  ", fallback),
        "capture.png"
    );
    assert_eq!(ScreenshotTool::sanitize_output_filename("   ", fallback), fallback);
    assert_eq!(ScreenshotTool::sanitize_output_filename("bad\0name.png", fallback), fallback);
    assert_eq!(ScreenshotTool::sanitize_output_filename("/", fallback), fallback);
}

/// 测试拒绝包含 Shell 注入字符的文件名
///
/// 验证工具在执行时能够检测并拒绝包含危险字符的文件名。
/// 测试使用包含单引号的文件名 `test'injection.png`，这可能导致 Shell 命令注入。
///
/// 期望结果：
/// - 执行返回失败状态（`success = false`）
/// - 错误消息中包含 "unsafe for shell execution" 警告
///
/// 这是防止 Shell 注入攻击的关键安全测试
#[tokio::test]
async fn screenshot_rejects_shell_injection_filename() {
    let tool = ScreenshotTool::new(test_security());
    let result = tool.execute(json!({"filename": "test'injection.png"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("unsafe for shell execution"));
}

#[tokio::test]
async fn screenshot_rejects_other_shell_control_characters() {
    let tool = ScreenshotTool::new(test_security());

    for filename in [
        "bad\"name.png",
        "bad`name.png",
        "bad$name.png",
        "bad;name.png",
        "bad|name.png",
        "bad&name.png",
        "bad\nname.png",
        "bad(name.png",
        "bad)name.png",
    ] {
        let result = tool
            .execute(json!({"filename": filename}))
            .await
            .expect("unsafe filename should return a tool result");

        assert!(!result.success, "{filename:?} must be rejected");
        assert_eq!(
            result.error.as_deref(),
            Some("Filename contains characters unsafe for shell execution"),
            "{filename:?} must report the shell safety error"
        );
    }
}

/// 测试截图命令中包含指定的输出路径
///
/// 验证生成的截图命令中确实包含了用户指定的输出文件路径。
/// 这确保命令构造正确，输出文件会被保存到预期位置。
///
/// 测试使用 `/tmp/my_screenshot.png` 作为输出路径，
/// 验证该路径出现在生成的命令中
#[test]
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn screenshot_command_contains_output_path() {
    let commands = ScreenshotTool::screenshot_commands("/tmp/my_screenshot.png");
    assert!(!commands.is_empty());
    let joined = commands[0].join(" ");
    assert!(joined.contains("/tmp/my_screenshot.png"), "Command should contain the output path");
}

#[test]
fn screenshot_commands_match_platform_contract() {
    let output_path = "/tmp/contract.png";
    let commands = ScreenshotTool::screenshot_commands(output_path);

    #[cfg(target_os = "macos")]
    assert_eq!(
        commands,
        vec![vec!["screencapture".to_string(), "-x".to_string(), output_path.to_string()]]
    );

    #[cfg(target_os = "linux")]
    assert_eq!(
        commands,
        vec![
            vec!["gnome-screenshot".to_string(), "-f".to_string(), output_path.to_string(),],
            vec!["scrot".to_string(), output_path.to_string()],
            vec![
                "import".to_string(),
                "-window".to_string(),
                "root".to_string(),
                output_path.to_string(),
            ],
        ]
    );

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    assert!(commands.is_empty());
}

/// 测试阻止符号链接输出目标攻击
///
/// 这是一个重要的安全测试，验证工具能够检测并拒绝写入到符号链接目标。
/// 符号链接攻击场景：攻击者创建一个指向工作区外敏感文件的符号链接，
/// 试图让工具覆盖该敏感文件。
///
/// 测试步骤：
/// 1. 创建临时工作区
/// 2. 在工作区外创建一个文件（模拟敏感文件）
/// 3. 在工作区内创建指向该外部文件的符号链接
/// 4. 尝试解析符号链接的输出路径
///
/// 期望结果：路径解析应返回错误，拒绝写入到符号链接目标
///
/// 这防止了通过符号链接逃逸工作区或覆盖外部敏感文件
#[tokio::test]
async fn screenshot_blocks_symlink_output_target() {
    // 创建临时目录和工作区
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    tokio::fs::create_dir_all(&workspace).await.expect("workspace should exist");

    // 在工作区外创建一个"敏感"文件
    let outside = temp.path().join("outside.png");
    tokio::fs::write(&outside, b"secret").await.expect("outside fixture should be written");

    // 在工作区内创建指向外部文件的符号链接
    symlink_file(&outside, &workspace.join("screen.png"));

    // 使用工作区路径创建安全策略
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    }));

    // 尝试解析符号链接的输出路径，应该被拒绝
    let result = tool.resolve_output_path_for_write("screen.png").await;
    assert!(result.is_err(), "symlink output target must be rejected");
}

#[tokio::test]
async fn screenshot_output_path_creates_workspace_for_new_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace.clone(),
        ..SecurityPolicy::default()
    }));

    let output_path = tool
        .resolve_output_path_for_write("new.png")
        .await
        .expect("new output path should resolve");
    let canonical_workspace =
        tokio::fs::canonicalize(&workspace).await.expect("workspace should be created");

    assert_eq!(output_path, canonical_workspace.join("new.png"));
    assert!(!output_path.exists());
}

#[tokio::test]
async fn screenshot_output_path_accepts_existing_regular_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    tokio::fs::create_dir_all(&workspace).await.expect("workspace should be created");
    tokio::fs::write(workspace.join("existing.png"), b"fixture")
        .await
        .expect("fixture should be written");
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    }));

    let output_path = tool
        .resolve_output_path_for_write("existing.png")
        .await
        .expect("regular file output should resolve");

    assert!(output_path.is_file());
}

#[tokio::test]
async fn screenshot_output_path_rejects_existing_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    tokio::fs::create_dir_all(workspace.join("directory.png"))
        .await
        .expect("directory fixture should be created");
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace,
        ..SecurityPolicy::default()
    }));

    let error = tool
        .resolve_output_path_for_write("directory.png")
        .await
        .expect_err("directory output must be rejected");

    assert!(error.to_string().contains("is not a regular file"));
}

#[tokio::test]
async fn screenshot_output_path_reports_workspace_creation_failure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_file = temp.path().join("workspace-file");
    tokio::fs::write(&workspace_file, b"not a directory")
        .await
        .expect("workspace fixture should be written");
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: workspace_file,
        ..SecurityPolicy::default()
    }));

    assert!(
        tool.resolve_output_path_for_write("screen.png").await.is_err(),
        "a regular file cannot serve as the screenshot workspace"
    );
}

#[tokio::test]
async fn screenshot_execute_blocks_read_only_before_creating_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("not-created");
    let tool = ScreenshotTool::new(Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        workspace_dir: workspace.clone(),
        ..SecurityPolicy::default()
    }));

    let result = tool.execute(json!({"filename": "blocked.png"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: autonomy is read-only"));
    assert!(!workspace.exists(), "permission denial must happen before filesystem changes");
}

#[tokio::test]
async fn screenshot_read_and_encode_returns_exact_binary_data_uri() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("screen.png");
    tokio::fs::write(&output_path, [0_u8, 255_u8]).await.expect("fixture should be written");

    let result = ScreenshotTool::read_and_encode(&output_path)
        .await
        .expect("read_and_encode should succeed");

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(
        result.output,
        format!(
            "Screenshot saved to: {}\nSize: 2 bytes\nBase64 length: 4\ndata:image/png;base64,AP8=",
            output_path.display()
        )
    );
}

#[tokio::test]
async fn screenshot_read_and_encode_supports_empty_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("empty.PNG");
    tokio::fs::write(&output_path, b"").await.expect("fixture should be written");

    let result = ScreenshotTool::read_and_encode(&output_path)
        .await
        .expect("empty screenshot should still be encoded");

    assert!(result.success);
    assert_eq!(result.error, None);
    assert!(result.output.contains("Size: 0 bytes\nBase64 length: 0"));
    assert!(result.output.ends_with("data:image/png;base64,"));
}

#[tokio::test]
#[cfg(unix)]
async fn screenshot_rejects_backslash_in_output_filename() {
    let tool = ScreenshotTool::new(test_security());

    let result = tool
        .execute(json!({"filename": "bad\\name.png"}))
        .await
        .expect("unsafe filename should return a tool result");

    assert!(!result.success);
    assert_eq!(
        result.error.as_deref(),
        Some("Filename contains characters unsafe for shell execution")
    );
}

#[tokio::test]
async fn screenshot_read_and_encode_selects_mime_type_from_extension() {
    let temp = tempfile::tempdir().expect("tempdir");

    for (extension, mime) in [
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("bmp", "image/bmp"),
        ("gif", "image/gif"),
        ("webp", "image/webp"),
        ("unknown", "image/png"),
    ] {
        let output_path = temp.path().join(format!("screen.{extension}"));
        tokio::fs::write(&output_path, b"x").await.expect("fixture should be written");

        let result = ScreenshotTool::read_and_encode(&output_path)
            .await
            .expect("read_and_encode should succeed");

        assert!(
            result.output.contains(&format!("data:{mime};base64,eA==")),
            "extension {extension} should use {mime}"
        );
    }
}

#[tokio::test]
async fn screenshot_read_and_encode_reports_missing_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("missing.png");

    let result = ScreenshotTool::read_and_encode(&output_path)
        .await
        .expect("read failure should return a tool result");

    assert!(!result.success);
    assert_eq!(result.output, format!("Screenshot saved to: {}", output_path.display()));
    assert!(matches!(
        result.error.as_deref(),
        Some(error) if error.contains("Failed to read screenshot file")
    ));
}

#[tokio::test]
async fn screenshot_read_and_encode_skips_inline_data_above_raw_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("large.png");
    let file = tokio::fs::File::create(&output_path).await.expect("fixture should be created");
    file.set_len(super::MAX_RAW_BYTES + 1).await.expect("fixture size should be set");

    let result = ScreenshotTool::read_and_encode(&output_path)
        .await
        .expect("oversized screenshot should return a tool result");

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(
        result.output,
        format!(
            "Screenshot saved to: {}\nSize: {} bytes (too large to base64-encode inline)",
            output_path.display(),
            super::MAX_RAW_BYTES + 1
        )
    );
    assert!(!result.output.contains("data:image/"));
}

#[tokio::test]
async fn screenshot_read_and_encode_keeps_medium_image_inline() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_path = temp.path().join("screen.png");
    let bytes = vec![0_u8; 2 * 1024 * 1024];
    tokio::fs::write(&output_path, bytes).await.expect("fixture should be written");

    let result = ScreenshotTool::read_and_encode(&output_path)
        .await
        .expect("read_and_encode should succeed");

    assert!(result.success);
    assert!(result.output.contains(&format!("Screenshot saved to: {}", output_path.display())));
    assert!(result.output.contains("Base64 length:"));
    assert!(result.output.contains("data:image/png;base64,"));
    assert!(!result.output.contains("too large to base64-encode inline"));
    assert!(!result.output.contains("truncated"));
}
