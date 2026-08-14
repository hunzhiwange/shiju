use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::{ReadTextFileRequest, WriteTextFileRequest};
use parking_lot::Mutex;

use super::errors::ErrorSource;
use super::filesystem::{FileSystemHandlers, FileSystemHandlersOptions};
use super::types::{
    ClientOperation, ClientOperationMethod, ClientOperationStatus, NonInteractivePermissionPolicy,
    PermissionMode,
};

fn temp_root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("vw-acp-filesystem-tests-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

#[test]
fn new_accepts_current_directory_cwd() {
    let _handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: PathBuf::from("."),
        ..FileSystemHandlersOptions::default()
    });
}

#[tokio::test]
async fn read_text_file_slices_requested_window_and_emits_completion() {
    let root = temp_root("read");
    let file_path = root.join("nested").join("sample.txt");
    std::fs::create_dir_all(file_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&file_path, "one\ntwo\nthree\nfour").expect("write sample");

    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).line(2_u32).limit(2_u32))
        .await
        .expect("read text file");

    assert_eq!(response.content, "two\nthree");
    let operations = operations.lock();
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[1].status, ClientOperationStatus::Completed);
    assert_eq!(operations[1].details.as_deref(), Some("line=2, limit=2"));
}

#[tokio::test]
async fn write_text_file_denies_paths_outside_root() {
    let root = temp_root("outside");
    let outside = root.parent().expect("parent").join("outside-vw-acp-test.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveAll,
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", outside, "secret"))
        .await
        .expect_err("outside path must fail");

    assert!(err.to_string().contains("outside allowed cwd subtree"));
}

#[tokio::test]
async fn full_access_allows_file_access_outside_root() {
    let root = temp_root("full-access-root");
    let outside = temp_root("full-access-outside").join("shared.txt");
    std::fs::write(&outside, "before").expect("write outside fixture");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        full_access: true,
        ..FileSystemHandlersOptions::default()
    });

    let read_result =
        handlers.read_text_file(&ReadTextFileRequest::new("session-1", &outside)).await;
    let write_result =
        handlers.write_text_file(&WriteTextFileRequest::new("session-1", &outside, "after")).await;
    let written_content = std::fs::read_to_string(&outside);

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside.parent().expect("outside parent"));

    assert_eq!(read_result.expect("full access read outside root").content, "before");
    write_result.expect("full access write outside root");
    assert_eq!(written_content.expect("read written outside file"), "after");
}

#[tokio::test]
async fn standard_access_still_denies_file_access_outside_root() {
    let root = temp_root("standard-access-root");
    let outside = temp_root("standard-access-outside").join("shared.txt");
    std::fs::write(&outside, "before").expect("write outside fixture");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        ..FileSystemHandlersOptions::default()
    });

    let read_err = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &outside))
        .await
        .expect_err("standard access read outside root must fail");
    let write_err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &outside, "after"))
        .await
        .expect_err("standard access write outside root must fail");
    let written_content = std::fs::read_to_string(&outside).expect("read outside fixture");

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside.parent().expect("outside parent"));

    assert!(read_err.to_string().contains("outside allowed cwd subtree"));
    assert!(write_err.to_string().contains("outside allowed cwd subtree"));
    assert_eq!(written_content, "before");
}

#[tokio::test]
async fn approve_reads_auto_approves_default_write_inside_root() {
    let root = temp_root("auto-write");
    let target = root.join("auto.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveReads,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "content"))
        .await
        .expect("workspace write should not prompt");

    assert_eq!(std::fs::read_to_string(target).expect("read file"), "content");
}

#[tokio::test]
async fn read_text_file_without_window_returns_full_content_without_details() {
    let root = temp_root("read-full");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo\nthree").expect("write sample");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path))
        .await
        .expect("read full file");

    assert_eq!(response.content, "one\ntwo\nthree");
    let operations = operations.lock();
    assert_eq!(operations[0].details, None);
    assert_eq!(operations[1].details, None);
}

#[tokio::test]
async fn read_text_file_limit_zero_returns_empty_window() {
    let root = temp_root("read-limit-zero");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo").expect("write sample");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).limit(0_u32))
        .await
        .expect("read empty window");

    assert_eq!(response.content, "");
}

#[tokio::test]
async fn read_text_file_out_of_range_window_returns_empty_content() {
    let root = temp_root("read-out-of-range");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo").expect("write sample");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).line(99_u32))
        .await
        .expect("read out of range window");

    assert_eq!(response.content, "");
}

#[tokio::test]
async fn read_text_file_zero_line_starts_from_first_line() {
    let root = temp_root("read-zero-line");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo").expect("write sample");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).line(0_u32).limit(1_u32))
        .await
        .expect("read from normalized first line");

    assert_eq!(response.content, "one");
}

#[tokio::test]
async fn read_text_file_limit_without_line_defaults_details_to_first_line() {
    let root = temp_root("read-limit-only");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo\nthree").expect("write sample");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).limit(2_u32))
        .await
        .expect("read limited window");

    assert_eq!(response.content, "one\ntwo");
    let operations = operations.lock();
    assert_eq!(operations[0].details.as_deref(), Some("line=1, limit=2"));
    assert_eq!(operations[1].details.as_deref(), Some("line=1, limit=2"));
}

#[tokio::test]
async fn read_text_file_line_without_limit_reads_remaining_content() {
    let root = temp_root("read-line-only");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "one\ntwo\nthree").expect("write sample");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path).line(2_u32))
        .await
        .expect("read remaining lines");

    assert_eq!(response.content, "two\nthree");
    let operations = operations.lock();
    assert_eq!(operations[0].details.as_deref(), Some("line=2, limit=all"));
    assert_eq!(operations[1].details.as_deref(), Some("line=2, limit=all"));
}

#[tokio::test]
async fn read_text_file_denied_mode_emits_failed_completion() {
    let root = temp_root("read-denied");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "secret").expect("write sample");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::DenyAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path))
        .await
        .expect_err("deny-all read must fail");

    assert!(err.to_string().contains("Permission denied"));
    let operations = operations.lock();
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].method, ClientOperationMethod::FsReadTextFile);
    assert_eq!(operations[0].status, ClientOperationStatus::Running);
    assert_eq!(operations[1].status, ClientOperationStatus::Failed);
    assert!(operations[1].details.as_deref().unwrap_or_default().contains("Permission denied"));
}

#[tokio::test]
async fn read_text_file_missing_file_reports_io_failure() {
    let root = temp_root("read-missing");
    let file_path = root.join("missing.txt");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path))
        .await
        .expect_err("missing file must fail");

    assert!(err.downcast_ref::<std::io::Error>().is_some());
    let operations = operations.lock();
    assert_eq!(operations[1].status, ClientOperationStatus::Failed);
    assert!(!operations[1].details.as_deref().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn read_text_file_rejects_relative_path_before_emitting_operation() {
    let root = temp_root("read-relative");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", "sample.txt"))
        .await
        .expect_err("relative path must fail");

    assert!(err.to_string().contains("Path must be absolute"));
    assert!(operations.lock().is_empty());
}

#[tokio::test]
async fn update_permission_policy_changes_read_decision() {
    let root = temp_root("update-policy");
    let file_path = root.join("sample.txt");
    std::fs::write(&file_path, "content").expect("write sample");
    let mut handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::DenyAll,
        ..FileSystemHandlersOptions::default()
    });

    handlers.update_permission_policy(PermissionMode::ApproveAll, None);
    let response = handlers
        .read_text_file(&ReadTextFileRequest::new("session-1", &file_path))
        .await
        .expect("updated policy should allow read");

    assert_eq!(response.content, "content");
}

#[tokio::test]
async fn write_text_file_normalizes_parent_components_inside_root() {
    let root = temp_root("write-parent-inside");
    let target = root.join("nested").join("..").join("normalized.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", target, "content"))
        .await
        .expect("normalized path inside root should be writable");

    assert_eq!(
        std::fs::read_to_string(root.join("normalized.txt")).expect("read normalized file"),
        "content"
    );
}

#[tokio::test]
async fn write_text_file_rejects_parent_escape_before_emitting_operation() {
    let root = temp_root("write-parent-escape");
    let target = root.join("..").join("escaped-vw-acp-test.txt");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", target, "secret"))
        .await
        .expect_err("parent escape must fail");

    assert!(err.to_string().contains("outside allowed cwd subtree"));
    assert!(operations.lock().is_empty());
}

#[tokio::test]
async fn write_text_file_deny_all_emits_failed_completion_and_skips_file() {
    let root = temp_root("write-deny-all");
    let target = root.join("denied.txt");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::DenyAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "secret"))
        .await
        .expect_err("deny-all write must fail");

    assert!(err.to_string().contains("Permission denied"));
    assert!(!target.exists());
    let operations = operations.lock();
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].method, ClientOperationMethod::FsWriteTextFile);
    assert_eq!(operations[1].status, ClientOperationStatus::Failed);
    assert!(operations[1].details.as_deref().unwrap_or_default().contains("Permission denied"));
}

#[tokio::test]
async fn write_text_file_root_path_reports_io_failure_without_parent() {
    let root = temp_root("write-directory-target");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &root, "content"))
        .await
        .expect_err("writing a directory path must fail");

    assert!(!err.to_string().is_empty());
    let operations = operations.lock();
    assert_eq!(operations[1].status, ClientOperationStatus::Failed);
    assert!(!operations[1].details.as_deref().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn approve_reads_denied_confirmation_skips_write() {
    let root = temp_root("confirm-denied");
    let target = root.join("denied.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveReads,
        confirm_write: Some(Arc::new(|_, _| Box::pin(async { Ok(false) }))),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "content"))
        .await
        .expect_err("denied confirmation must fail");

    assert!(err.to_string().contains("Permission denied"));
    assert!(!target.exists());
}

#[tokio::test]
async fn approve_reads_confirmation_error_is_reported_as_failed_operation() {
    let root = temp_root("confirm-error");
    let target = root.join("error.txt");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveReads,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        confirm_write: Some(Arc::new(|_, _| {
            Box::pin(async {
                let error: ErrorSource = Box::new(std::io::Error::other("callback unavailable"));
                Err(error)
            })
        })),
        ..FileSystemHandlersOptions::default()
    });

    let err = handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "content"))
        .await
        .expect_err("confirmation error must fail");

    assert!(err.to_string().contains("callback unavailable"));
    assert!(!target.exists());
    let operations = operations.lock();
    assert_eq!(operations[1].status, ClientOperationStatus::Failed);
    assert!(operations[1].details.as_deref().unwrap_or_default().contains("callback unavailable"));
}

#[tokio::test]
async fn write_text_file_creates_parent_directories() {
    let root = temp_root("write-create-parent");
    let target = root.join("a").join("b").join("created.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveAll,
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "content"))
        .await
        .expect("write nested file");

    assert_eq!(std::fs::read_to_string(target).expect("read nested file"), "content");
}

#[tokio::test]
async fn write_text_file_normalizes_curdir_components_inside_root() {
    let root = temp_root("write-curdir-inside");
    let target = root.join(".").join("nested").join(".").join("curdir.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", target, "content"))
        .await
        .expect("curdir path inside root should be writable");

    assert_eq!(
        std::fs::read_to_string(root.join("nested").join("curdir.txt"))
            .expect("read normalized file"),
        "content"
    );
}

#[tokio::test]
async fn write_text_file_preview_normalizes_crlf_and_reports_extra_lines() {
    let root = temp_root("preview-lines");
    let target = root.join("preview.txt");
    let content = (1..=18).map(|line| format!("line-{line:02}")).collect::<Vec<_>>().join("\r\n");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", target, content))
        .await
        .expect("write file");

    let details = operations.lock()[0].details.clone().expect("preview details");
    assert!(details.contains("line-01\nline-02"));
    assert!(details.contains("... (2 more lines)"));
    assert!(!details.contains('\r'));
}

#[tokio::test]
async fn write_text_file_preview_truncates_long_content() {
    let root = temp_root("preview-chars");
    let target = root.join("preview.txt");
    let operations = Arc::new(Mutex::new(Vec::<ClientOperation>::new()));
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some({
            let operations = Arc::clone(&operations);
            Arc::new(move |operation| operations.lock().push(operation))
        }),
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", target, "a".repeat(1_300)))
        .await
        .expect("write file");

    let details = operations.lock()[0].details.clone().expect("preview details");
    assert_eq!(details.chars().count(), 1_200);
    assert!(details.ends_with("..."));
}

#[tokio::test]
async fn approve_reads_uses_confirmation_for_writes() {
    let root = temp_root("confirm");
    let target = root.join("approved.txt");
    let handlers = FileSystemHandlers::new(FileSystemHandlersOptions {
        cwd: root,
        permission_mode: PermissionMode::ApproveReads,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Deny),
        confirm_write: Some(Arc::new(|path, preview| {
            Box::pin(async move {
                assert!(path.ends_with("approved.txt"));
                assert_eq!(preview, "content");
                Ok(true)
            })
        })),
        ..FileSystemHandlersOptions::default()
    });

    handlers
        .write_text_file(&WriteTextFileRequest::new("session-1", &target, "content"))
        .await
        .expect("write approved file");

    assert_eq!(std::fs::read_to_string(target).expect("read file"), "content");
}
