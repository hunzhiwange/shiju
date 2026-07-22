//! 用户文件附件工具测试。
//!
//! 测试确保工作区相对路径会解析为真实文件，并返回模型可引用的附件标记。

use super::SendUserFileTool;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn send_user_file_returns_attachment_marker() {
    let tmp = TempDir::new().expect("tempdir should create");
    let file_path = tmp.path().join("report.txt");
    fs::write(&file_path, "hello").expect("test file should write");

    let mut policy = SecurityPolicy::default();
    policy.workspace_dir = tmp.path().to_path_buf();
    let tool = SendUserFileTool::new(Arc::new(policy));
    let result = tool
        .call(json!({
            "path": "report.txt"
        }))
        .await
        .expect("send user file should succeed");

    assert!(result.is_success());
    let expected = format!("[DOCUMENT:{}]", file_path.display());
    assert_eq!(result.model_result.as_str(), Some(expected.as_str()));
}

#[test]
fn infer_kind_honors_supported_override_and_normalizes_case() {
    let path = Path::new("archive.bin");

    assert_eq!(SendUserFileTool::infer_kind(path, Some(" IMAGE ")), "IMAGE");
    assert_eq!(SendUserFileTool::infer_kind(path, Some("video")), "VIDEO");
    assert_eq!(SendUserFileTool::infer_kind(path, Some("Audio")), "AUDIO");
    assert_eq!(SendUserFileTool::infer_kind(path, Some("voice")), "VOICE");
}

#[test]
fn infer_kind_falls_back_to_extension_for_unknown_override() {
    assert_eq!(SendUserFileTool::infer_kind(Path::new("photo.JPEG"), None), "IMAGE");
    assert_eq!(SendUserFileTool::infer_kind(Path::new("clip.MOV"), Some("unknown")), "VIDEO");
    assert_eq!(SendUserFileTool::infer_kind(Path::new("sound.ogg"), None), "AUDIO");
    assert_eq!(SendUserFileTool::infer_kind(Path::new("README"), None), "DOCUMENT");
}

#[tokio::test]
async fn send_user_file_rejects_directory() {
    let tmp = TempDir::new().expect("tempdir should create");
    let policy =
        SecurityPolicy { workspace_dir: tmp.path().to_path_buf(), ..SecurityPolicy::default() };
    let tool = SendUserFileTool::new(Arc::new(policy));

    let error = tool
        .call(json!({"path": "."}))
        .await
        .expect_err("directories must not be returned as attachments");

    assert_eq!(error.to_string(), "path does not point to a regular file");
}

#[tokio::test]
async fn send_user_file_reports_missing_file_metadata_error() {
    let tmp = TempDir::new().expect("tempdir should create");
    let policy =
        SecurityPolicy { workspace_dir: tmp.path().to_path_buf(), ..SecurityPolicy::default() };
    let tool = SendUserFileTool::new(Arc::new(policy));

    let error =
        tool.call(json!({"path": "missing.txt"})).await.expect_err("missing file should fail");

    assert!(error.to_string().starts_with("failed to read file metadata:"));
}

#[test]
fn send_user_file_spec_declares_safe_read_only_contract() {
    let tool = SendUserFileTool::new(Arc::new(SecurityPolicy::default()));
    let spec = tool.spec();

    assert_eq!(tool.name(), "SendUserFile");
    assert_eq!(spec.name, "SendUserFile");
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.strict);
    assert_eq!(tool.parameters_schema()["required"], json!(["path"]));
}
