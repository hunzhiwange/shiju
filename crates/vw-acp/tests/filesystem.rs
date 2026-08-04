//! 验证 ACP 文件系统 handler 的读写行为和工作区边界。
//!
//! 测试覆盖读取行号/限制、写入确认回调，以及禁止写出 cwd 子树的安全约束。
//! 这些路径直接接触本地文件系统，因此使用真实临时目录验证路径规范化结果。

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use agent_client_protocol::{ReadTextFileRequest, WriteTextFileRequest};
use serde_json::json;
use vw_acp::{
    FileSystemHandlers, FileSystemHandlersOptions, NonInteractivePermissionPolicy, PermissionMode,
};

/// 生成唯一临时目录，避免异步测试或重复运行时共享同一文件系统状态。
fn unique_temp_dir() -> PathBuf {
    static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vw-acp-filesystem-{nanos}-{counter}"))
}

/// 验证读取文本文件时 `line` 和 `limit` 参数按 ACP 约定裁剪输出。
#[tokio::test]
async fn read_text_file_honors_line_and_limit() {
    let dir = unique_temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("notes.txt");
    std::fs::write(&file_path, "one\ntwo\nthree\nfour").unwrap();

    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: dir.clone(),
        permission_mode: PermissionMode::ApproveReads,
        full_access: false,
        non_interactive_permissions: None,
        on_operation: None,
        confirm_write: None,
    });

    let request: ReadTextFileRequest = serde_json::from_value(json!({
        "sessionId": "test-session",
        "path": file_path,
        "line": 2,
        "limit": 2
    }))
    .unwrap();

    let response = handlers.read_text_file(&request).await.unwrap();
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["content"], json!("two\nthree"));

    let _ = std::fs::remove_dir_all(dir);
}

/// 验证写入文件前会调用确认回调，并在确认通过后创建父目录和目标文件。
#[tokio::test]
async fn write_text_file_uses_confirmation_callback() {
    let dir = unique_temp_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("nested").join("result.txt");

    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: dir.clone(),
        permission_mode: PermissionMode::ApproveReads,
        full_access: false,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Deny),
        on_operation: None,
        confirm_write: Some(Arc::new(|_path, preview| {
            Box::pin(async move {
                // 确认回调收到 preview，调用方可以在非交互策略下审计即将写入的内容。
                assert!(preview.contains("hello world"));
                Ok(true)
            })
        })),
    });

    let request: WriteTextFileRequest = serde_json::from_value(json!({
        "sessionId": "test-session",
        "path": file_path,
        "content": "hello world"
    }))
    .unwrap();

    handlers.write_text_file(&request).await.unwrap();
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello world");

    let _ = std::fs::remove_dir_all(dir);
}

/// 验证即使权限模式允许写入，也不会允许目标路径逃逸出配置的工作目录。
#[tokio::test]
async fn write_text_file_rejects_path_outside_root() {
    let dir = unique_temp_dir();
    std::fs::create_dir_all(&dir).unwrap();

    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: dir.clone(),
        permission_mode: PermissionMode::ApproveAll,
        full_access: false,
        non_interactive_permissions: None,
        on_operation: None,
        confirm_write: None,
    });

    let outside = dir.parent().unwrap_or(&dir).join("outside.txt");
    let request: WriteTextFileRequest = serde_json::from_value(json!({
        "sessionId": "test-session",
        "path": outside,
        "content": "blocked"
    }))
    .unwrap();

    let error = handlers.write_text_file(&request).await.expect_err("path should be rejected");
    assert!(error.to_string().contains("outside allowed cwd subtree"));

    let _ = std::fs::remove_dir_all(dir);
}
