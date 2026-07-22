use super::*;

#[test]
fn preview_and_output_cover_head_tail_byte_and_no_truncation() {
    let defaults = Options::default();
    assert_eq!(defaults.max_lines, MAX_LINES);
    assert_eq!(defaults.max_bytes, MAX_BYTES);
    assert_eq!(defaults.direction, Direction::Head);

    let head = compute_preview(
        "a\nb\nc",
        &Options { max_lines: 2, max_bytes: 99, direction: Direction::Head },
    );
    assert_eq!(head.text, "a\nb");
    assert!(!head.hit_bytes);

    let tail = compute_preview(
        "a\nb\nc",
        &Options { max_lines: 2, max_bytes: 99, direction: Direction::Tail },
    );
    assert_eq!(tail.text, "b\nc");

    let bytes = compute_preview(
        "abcd",
        &Options { max_lines: 9, max_bytes: 1, direction: Direction::Head },
    );
    assert!(bytes.hit_bytes);

    let original =
        output("short", Options { max_lines: 1, max_bytes: 99, direction: Direction::Head }, None);
    assert!(!original.truncated);
    assert_eq!(original.content, "short");

    let truncated = output(
        "a\nb\nc",
        Options { max_lines: 1, max_bytes: 99, direction: Direction::Head },
        None,
    );
    assert!(truncated.truncated);
    assert!(truncated.content.contains("已截断 2 lines"));
}

#[test]
fn cleanup_is_tolerant() {
    cleanup().unwrap();
}

#[test]
fn init_registers_cleanup_task_once_without_panicking() {
    init();
    init();
}

#[tokio::test]
async fn cleanup_task_runner_executes_cleanup_on_blocking_pool() {
    run_cleanup_task().await.unwrap();
}

#[test]
fn cleanup_dir_removes_only_expired_tool_output_files() {
    let dir = tempfile::tempdir().unwrap();
    let now = now_ms();
    let retention_ms = u64::try_from(RETENTION.as_millis()).unwrap();
    let expired_id =
        id::create(id::Prefix::Tool, false, Some(now.saturating_sub(retention_ms + 1))).unwrap();
    let fresh_id = id::create(id::Prefix::Tool, false, Some(now)).unwrap();
    let expired = dir.path().join(expired_id);
    let fresh = dir.path().join(fresh_id);
    let invalid_tool = dir.path().join("tool_not-a-valid-id");
    let unrelated = dir.path().join("other_output");
    let nested = dir.path().join("tool_nested_dir");

    std::fs::write(&expired, "old").unwrap();
    std::fs::write(&fresh, "fresh").unwrap();
    std::fs::write(&invalid_tool, "invalid").unwrap();
    std::fs::write(&unrelated, "other").unwrap();
    std::fs::create_dir(&nested).unwrap();

    cleanup_dir(dir.path(), now).unwrap();

    assert!(!expired.exists());
    assert!(fresh.exists());
    assert!(invalid_tool.exists());
    assert!(unrelated.exists());
    assert!(nested.exists());
}

#[test]
fn cleanup_dir_tolerates_unreadable_directory_target() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("tool-output-file");
    std::fs::write(&file_path, "not a directory").unwrap();

    cleanup_dir(&file_path, now_ms()).unwrap();

    assert!(file_path.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn cleanup_dir_skips_non_utf8_file_names() {
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let name = std::ffi::OsString::from_vec(b"tool_\xFF".to_vec());
    let path = dir.path().join(name);
    std::fs::write(&path, "invalid utf8").unwrap();

    cleanup_dir(dir.path(), now_ms()).unwrap();

    assert!(path.exists());
}

#[test]
fn output_reports_save_failure_when_output_directory_is_not_writable() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("tool-output-file");
    std::fs::write(&file_path, "not a directory").unwrap();

    let result = output_with_dir(
        "alpha\nbeta",
        Options { max_lines: 1, max_bytes: 99, direction: Direction::Head },
        &file_path,
    );

    assert!(result.truncated);
    assert_eq!(result.output_path, None);
    assert!(result.content.contains("完整输出保存失败"));
}

#[test]
fn tail_output_reports_removed_bytes_before_retained_suffix() {
    let truncated = output(
        "alpha\nbeta",
        Options { max_lines: 10, max_bytes: 4, direction: Direction::Tail },
        None,
    );

    assert!(truncated.truncated);
    assert!(truncated.content.starts_with("...已截断 6 bytes..."));
    assert!(truncated.content.ends_with("beta"));
}
