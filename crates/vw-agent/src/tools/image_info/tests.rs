//! 图片信息工具的格式解析、尺寸提取和路径安全测试。
//!
//! 测试使用最小二进制头部构造图片样本，避免依赖外部 fixture，同时覆盖符号链接
//! 逃逸工作区的拒绝路径。

use super::super::*;
use super::MAX_IMAGE_BYTES;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn symlink_file(src: &Path, dst: &Path) {
    std::os::unix::fs::symlink(src, dst).expect("symlink should be created");
}

#[cfg(windows)]
fn symlink_file(src: &Path, dst: &Path) {
    std::os::windows::fs::symlink_file(src, dst).expect("symlink should be created");
}

fn test_security() -> Arc<SecurityPolicy> {
    // 默认测试策略不限制工作区，路径隔离场景在专门用例中单独构造。
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: std::env::temp_dir(),
        workspace_only: false,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    })
}

#[test]
fn image_info_tool_name() {
    let tool = ImageInfoTool::new(test_security());
    assert_eq!(tool.name(), "image_info");
}

#[test]
fn image_info_tool_description() {
    let tool = ImageInfoTool::new(test_security());
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("image"));
}

#[test]
fn image_info_tool_schema() {
    let tool = ImageInfoTool::new(test_security());
    let schema = tool.parameters_schema();
    assert!(schema["properties"]["path"].is_object());
    assert!(schema["properties"]["include_base64"].is_object());
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("path")));
}

#[test]
fn image_info_tool_spec() {
    let tool = ImageInfoTool::new(test_security());
    let spec = tool.spec();
    assert_eq!(spec.name, "image_info");
    assert!(spec.parameters.is_object());
}

#[test]
fn detect_png() {
    let bytes = b"\x89PNG\r\n\x1a\n";
    assert_eq!(ImageInfoTool::detect_format(bytes), "png");
}

#[test]
fn detect_jpeg() {
    let bytes = b"\xFF\xD8\xFF\xE0";
    assert_eq!(ImageInfoTool::detect_format(bytes), "jpeg");
}

#[test]
fn detect_gif() {
    let bytes = b"GIF89a";
    assert_eq!(ImageInfoTool::detect_format(bytes), "gif");
}

#[test]
fn detect_webp() {
    let bytes = b"RIFF\x00\x00\x00\x00WEBP";
    assert_eq!(ImageInfoTool::detect_format(bytes), "webp");
}

#[test]
fn detect_bmp() {
    let bytes = b"BM\x00\x00";
    assert_eq!(ImageInfoTool::detect_format(bytes), "bmp");
}

#[test]
fn detect_unknown_short() {
    let bytes = b"\x00\x01";
    assert_eq!(ImageInfoTool::detect_format(bytes), "unknown");
}

#[test]
fn detect_unknown_garbage() {
    let bytes = b"this is not an image";
    assert_eq!(ImageInfoTool::detect_format(bytes), "unknown");
}

#[test]
fn png_dimensions() {
    let mut bytes = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x03, 0x20, 0x00, 0x00, 0x02, 0x58,
    ];
    bytes.extend_from_slice(&[0u8; 10]);
    let dims = ImageInfoTool::extract_dimensions(&bytes, "png");
    assert_eq!(dims, Some((800, 600)));
}

#[test]
fn gif_dimensions() {
    let bytes = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x40, 0x01, 0xF0, 0x00];
    let dims = ImageInfoTool::extract_dimensions(&bytes, "gif");
    assert_eq!(dims, Some((320, 240)));
}

#[test]
fn bmp_dimensions() {
    let mut bytes = vec![0u8; 26];
    bytes[0] = b'B';
    bytes[1] = b'M';
    bytes[18] = 0x00;
    bytes[19] = 0x04;
    bytes[20] = 0x00;
    bytes[21] = 0x00;
    bytes[22] = 0x00;
    bytes[23] = 0x03;
    bytes[24] = 0x00;
    bytes[25] = 0x00;
    let dims = ImageInfoTool::extract_dimensions(&bytes, "bmp");
    assert_eq!(dims, Some((1024, 768)));
}

#[test]
fn jpeg_dimensions() {
    let mut bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    bytes.extend_from_slice(&[0u8; 14]);
    bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01, 0xE0, 0x02, 0x80]);
    let dims = ImageInfoTool::extract_dimensions(&bytes, "jpeg");
    assert_eq!(dims, Some((640, 480)));
}

#[test]
fn jpeg_malformed_zero_length_segment() {
    let bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00];
    let dims = ImageInfoTool::extract_dimensions(&bytes, "jpeg");
    assert!(dims.is_none());
}

#[test]
fn unknown_format_no_dimensions() {
    let bytes = b"random data here";
    let dims = ImageInfoTool::extract_dimensions(bytes, "unknown");
    assert!(dims.is_none());
}

#[tokio::test]
async fn execute_missing_path() {
    let tool = ImageInfoTool::new(test_security());
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn execute_nonexistent_file() {
    let tool = ImageInfoTool::new(test_security());
    let result = tool.execute(json!({"path": "/tmp/nonexistent_image_xyz.png"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("not found"));
}

#[tokio::test]
async fn execute_real_file() {
    let dir = std::env::temp_dir().join("vibewindow_image_info_test");
    let _ = tokio::fs::create_dir_all(&dir).await;
    let png_path = dir.join("test.png");

    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    tokio::fs::write(&png_path, &png_bytes).await.unwrap();

    let tool = ImageInfoTool::new(test_security());
    let result = tool.execute(json!({"path": png_path.to_string_lossy()})).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Format: png"));
    assert!(result.output.contains("Dimensions: 1x1"));
    assert!(!result.output.contains("data:"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn execute_with_base64() {
    let dir = std::env::temp_dir().join("vibewindow_image_info_b64");
    let _ = tokio::fs::create_dir_all(&dir).await;
    let png_path = dir.join("test_b64.png");

    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    tokio::fs::write(&png_path, &png_bytes).await.unwrap();

    let tool = ImageInfoTool::new(test_security());
    let result = tool
        .execute(json!({"path": png_path.to_string_lossy(), "include_base64": true}))
        .await
        .unwrap();
    assert!(result.success);
    assert!(result.output.contains("data:image/png;base64,"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn execute_blocks_symlink_escape_outside_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should exist");

    let outside = temp.path().join("secret.png");
    std::fs::write(&outside, b"not-an-image").expect("fixture should be written");

    let link = workspace.join("link.png");
    symlink_file(&outside, &link);

    let policy = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: PathBuf::from(&workspace),
        workspace_only: true,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tool = ImageInfoTool::new(policy);

    let result = tool.execute(json!({"path": "link.png"})).await.unwrap();
    assert!(!result.success, "symlink escape must be blocked");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("escapes workspace allowlist")
            || err.contains("Path not allowed")
            || err.contains("outside"),
        "unexpected error message: {err}"
    );
}

#[test]
fn dimension_extractors_reject_truncated_headers() {
    assert_eq!(ImageInfoTool::extract_dimensions(&[0; 23], "png"), None);
    assert_eq!(ImageInfoTool::extract_dimensions(&[0; 9], "gif"), None);
    assert_eq!(ImageInfoTool::extract_dimensions(&[0; 25], "bmp"), None);
}

#[test]
fn jpeg_dimensions_rejects_invalid_marker_and_truncated_segments() {
    assert_eq!(ImageInfoTool::jpeg_dimensions(&[0xFF, 0xD8, 0x00, 0xE0]), None);
    assert_eq!(ImageInfoTool::jpeg_dimensions(&[0xFF, 0xD8, 0xFF]), None);
    assert_eq!(ImageInfoTool::jpeg_dimensions(&[0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11]), None);
}

#[tokio::test]
async fn execute_rejects_directory_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: dir.path().to_path_buf(),
        workspace_only: true,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tool = ImageInfoTool::new(policy);

    let result = tool.execute(json!({"path": "."})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("Not a file"));
}

#[tokio::test]
async fn execute_rejects_image_over_size_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let image = dir.path().join("large.png");
    let file = std::fs::File::create(&image).expect("fixture should be created");
    file.set_len(MAX_IMAGE_BYTES + 1).expect("fixture size should be set");
    let policy = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_dir: dir.path().to_path_buf(),
        workspace_only: true,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    });
    let tool = ImageInfoTool::new(policy);

    let result = tool.execute(json!({"path": "large.png"})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.as_deref().unwrap().contains("Image too large"));
}
