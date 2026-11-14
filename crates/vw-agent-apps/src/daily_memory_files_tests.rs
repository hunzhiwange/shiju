use super::daily_memory_files::{read_daily_memory_file, write_daily_memory_file};
use super::workspace_paths::PathError;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use vw_api_types::workspace_memory::DailyMemoryFilename;

#[test]
fn daily_roundtrip_valid_dates_preserve_unicode_empty_and_missing() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    for date in ["0001-01-01.md", "2000-02-29.md", "2024-02-29.md", "9999-12-31.md"] {
        let name: DailyMemoryFilename = date.parse().unwrap();
        assert_eq!(
            read_daily_memory_file("host", "host", home.path(), &root, &name).unwrap(),
            None
        );
        for content in ["中文记忆\n🙂\n", "", "再次保存"] {
            write_daily_memory_file("host", "host", home.path(), &root, &name, content).unwrap();
            assert_eq!(
                read_daily_memory_file("host", "host", home.path(), &root, &name)
                    .unwrap()
                    .as_deref(),
                Some(content)
            );
            assert_eq!(
                fs::metadata(root.join("memory").join(date)).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn daily_roundtrip_valid_dates_reject_invalid_calendar_and_paths() {
    for name in [
        "0000-01-01.md",
        "1900-02-29.md",
        "2025-02-29.md",
        "2026-04-31.md",
        "2026-00-01.md",
        "2026-13-01.md",
        "2026-01-00.md",
        "2026-1-01.md",
        "../2026-01-01.md",
        "2026-01-01.MD",
        "２０２６-01-01.md",
        "2026-01-01.md\n",
    ] {
        assert!(name.parse::<DailyMemoryFilename>().is_err(), "accepted {name:?}");
    }
}

#[test]
fn daily_roundtrip_valid_dates_missing_read_and_host_denial_create_nothing() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let name = "2026-11-14.md".parse().unwrap();
    assert_eq!(read_daily_memory_file("host", "host", home.path(), &root, &name).unwrap(), None);
    assert!(!root.exists());
    assert_eq!(
        write_daily_memory_file("other", "host", home.path(), &root, &name, "data"),
        Err(PathError::UnauthorizedHost)
    );
    assert!(!root.exists());
}

#[test]
fn daily_roundtrip_valid_dates_unsafe_write_preserves_original() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let name: DailyMemoryFilename = "2026-11-14.md".parse().unwrap();
    write_daily_memory_file("host", "host", home.path(), &root, &name, "原文").unwrap();
    let path = root.join("memory").join(name.as_str());
    let alias = home.path().join("alias");
    fs::hard_link(&path, &alias).unwrap();
    assert_eq!(
        write_daily_memory_file("host", "host", home.path(), &root, &name, "覆盖"),
        Err(PathError::UnsafeEntry)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "原文");
    fs::remove_file(&path).unwrap();
    symlink(&alias, &path).unwrap();
    assert_eq!(
        read_daily_memory_file("host", "host", home.path(), &root, &name),
        Err(PathError::UnsafeEntry)
    );
    assert_eq!(
        write_daily_memory_file("host", "host", home.path(), &root, &name, "覆盖"),
        Err(PathError::UnsafeEntry)
    );
    assert_eq!(fs::read_to_string(alias).unwrap(), "原文");
}

#[test]
fn daily_delete_is_idempotent_preserves_other_files_and_missing_directories() {
    use super::daily_memory_files::delete_daily_memory_file;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let name = "2026-11-14.md".parse().unwrap();
    delete_daily_memory_file("host", "host", home.path(), &root, &name).unwrap();
    assert!(!root.exists());
    write_daily_memory_file("host", "host", home.path(), &root, &name, "中文").unwrap();
    let other = root.join("memory/2026-11-13.md");
    fs::write(&other, "保留").unwrap();
    for _ in 0..2 {
        delete_daily_memory_file("host", "host", home.path(), &root, &name).unwrap();
        assert!(!root.join("memory/2026-11-14.md").exists());
        assert_eq!(fs::read_to_string(&other).unwrap(), "保留");
    }
}

#[test]
fn daily_delete_is_idempotent_rejects_unsafe_entries_and_wrong_host() {
    use super::daily_memory_files::delete_daily_memory_file;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("workspace");
    let name = "2026-11-14.md".parse().unwrap();
    assert_eq!(
        delete_daily_memory_file("other", "host", home.path(), &root, &name),
        Err(PathError::UnauthorizedHost)
    );
    assert!(!root.exists());
    fs::create_dir_all(root.join("memory/2026-11-14.md")).unwrap();
    let path = root.join("memory/2026-11-14.md");
    assert_eq!(
        delete_daily_memory_file("host", "host", home.path(), &root, &name),
        Err(PathError::UnsafeEntry)
    );
    assert!(path.is_dir());
    fs::remove_dir(&path).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("private.md");
    fs::write(&target, "不可删除").unwrap();
    symlink(&target, &path).unwrap();
    assert_eq!(
        delete_daily_memory_file("host", "host", home.path(), &root, &name),
        Err(PathError::UnsafeEntry)
    );
    assert!(fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&target).unwrap(), "不可删除");
}
