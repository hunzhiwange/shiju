use super::{
    apps::lifecycle::operations::Operation,
    daily_memory_search::{SearchError, search_daily_memory_files},
    workspace_paths::PathError,
};
use std::{fs, os::unix::fs::symlink};

#[test]
fn daily_search_handles_unicode_offsets_and_counts() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    fs::create_dir_all(root.join("memory")).unwrap();
    let content = format!("{}目标🙂AbC abc尾", "İ".repeat(100));
    fs::write(root.join("memory/2026-11-14.md"), &content).unwrap();
    let op = Operation::new();
    let search =
        |query| search_daily_memory_files("host", "host", home.path(), &root, query, &op).unwrap();
    let found = search("目标");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].snippet, format!("...{}目标🙂AbC abc尾", "İ".repeat(50)));
    assert_eq!(found[0].match_count, 1);
    assert_eq!(found[0].size_bytes, content.len() as u64);
    assert_eq!(search("ABC")[0].match_count, 2);
    assert_eq!(search("\u{307}")[0].match_count, 100);
    assert!(search("").is_empty());
    assert!(search("不存在").is_empty());
}

#[test]
fn daily_search_handles_unicode_offsets_date_order_and_safe_entries() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let memory = root.join("memory");
    fs::create_dir_all(&memory).unwrap();
    fs::write(memory.join("2026-11-14.md"), "中文🙂".repeat(60)).unwrap();
    fs::write(memory.join("2026-01-01.md"), "").unwrap();
    fs::write(memory.join("2026-02-30.md"), [0xff]).unwrap();
    fs::create_dir(memory.join("2026-01-02.md")).unwrap();
    let outside = home.path().join("private");
    fs::write(&outside, [0xff]).unwrap();
    symlink(&outside, memory.join("2026-01-03.md")).unwrap();
    fs::hard_link(&outside, memory.join("2026-01-04.md")).unwrap();
    let found =
        search_daily_memory_files("host", "host", home.path(), &root, "2026", &Operation::new())
            .unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].filename.as_str(), "2026-11-14.md");
    assert_eq!(found[0].match_count, 0);
    assert_eq!(found[0].snippet, format!("{}...", "中文🙂".repeat(40)));
    assert_eq!(found[1].snippet, "");
}

#[test]
fn daily_search_handles_unicode_offsets_errors_cancellation_and_missing() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let op = Operation::new();
    assert!(
        search_daily_memory_files("host", "host", home.path(), &root, "x", &op).unwrap().is_empty()
    );
    assert!(!root.exists());
    assert!(matches!(
        search_daily_memory_files("other", "host", home.path(), &root, "", &op),
        Err(SearchError::Path(PathError::UnauthorizedHost))
    ));
    fs::create_dir_all(root.join("memory")).unwrap();
    fs::write(root.join("memory/2026-11-14.md"), [0xff]).unwrap();
    assert!(matches!(
        search_daily_memory_files("host", "host", home.path(), &root, "x", &op),
        Err(SearchError::Path(PathError::Io))
    ));
    op.cancel();
    assert!(matches!(
        search_daily_memory_files("host", "host", home.path(), &root, "x", &op),
        Err(SearchError::Cancelled)
    ));
}
