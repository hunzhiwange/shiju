use super::{daily_memory_list::list_daily_memory_files, workspace_paths::PathError};
use std::{fs, os::unix::fs::symlink, time::UNIX_EPOCH};

#[test]
fn daily_list_preserves_metadata_and_unicode_preview_in_descending_order() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let memory = root.join("memory");
    fs::create_dir_all(&memory).unwrap();
    let content = format!("{}尾", "中🙂".repeat(100));
    fs::write(memory.join("2024-02-29.md"), &content).unwrap();
    fs::write(memory.join("2026-11-14.md"), "").unwrap();
    let files = list_daily_memory_files("host", "host", home.path(), &root).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].filename.as_str(), "2026-11-14.md");
    assert_eq!(files[0].preview, "");
    assert_eq!(files[0].size_bytes, 0);
    assert_eq!(files[1].date, "2024-02-29");
    assert_eq!(files[1].size_bytes, content.len() as u64);
    assert_eq!(files[1].preview, "中🙂".repeat(100));
    let metadata = fs::metadata(memory.join("2024-02-29.md")).unwrap();
    assert_eq!(
        files[1].modified_at,
        metadata.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs()
    );
    assert_eq!(list_daily_memory_files("host", "host", home.path(), &root).unwrap().len(), 2);
}

#[test]
fn daily_list_preserves_metadata_skips_invalid_names_links_and_directories() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let memory = root.join("memory");
    fs::create_dir_all(&memory).unwrap();
    for name in ["2025-02-29.md", "1900-02-29.md", "0000-01-01.md", "notes.md", "2026-1-01.md"] {
        // Invalid UTF-8 must never be read for rejected names.
        fs::write(memory.join(name), [0xff]).unwrap();
    }
    fs::create_dir(memory.join("2026-01-01.md")).unwrap();
    let outside = home.path().join("private");
    fs::write(&outside, [0xff]).unwrap();
    symlink(&outside, memory.join("2026-01-02.md")).unwrap();
    symlink(home.path().join("absent"), memory.join("2026-01-03.md")).unwrap();
    fs::hard_link(&outside, memory.join("2026-01-04.md")).unwrap();
    fs::write(memory.join("2000-02-29.md"), "合法").unwrap();
    let files = list_daily_memory_files("host", "host", home.path(), &root).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].preview, "合法");
}

#[test]
fn daily_list_preserves_metadata_reports_errors_and_missing_without_creation() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    assert!(list_daily_memory_files("host", "host", home.path(), &root).unwrap().is_empty());
    assert!(!root.exists());
    assert!(matches!(
        list_daily_memory_files("other", "host", home.path(), &root),
        Err(PathError::UnauthorizedHost)
    ));
    fs::create_dir_all(root.join("memory")).unwrap();
    fs::write(root.join("memory/2026-11-14.md"), [0xff]).unwrap();
    assert!(matches!(
        list_daily_memory_files("host", "host", home.path(), &root),
        Err(PathError::Io)
    ));
}
