//! 验证旧版 OpenSkills 发现与技能元数据转换行为。

use super::*;

#[test]
fn resolves_open_skills_dir_by_source_priority() {
    let home = std::path::Path::new("/home/example");
    assert_eq!(
        resolve_open_skills_dir_from_sources(Some(" /env/open "), Some("/config/open"), Some(home)),
        Some(PathBuf::from("/env/open"))
    );
    assert_eq!(
        resolve_open_skills_dir_from_sources(None, Some(" /config/open "), Some(home)),
        Some(PathBuf::from("/config/open"))
    );
    assert_eq!(
        resolve_open_skills_dir_from_sources(None, None, Some(home)),
        Some(PathBuf::from("/home/example/open-skills"))
    );
}

#[test]
fn empty_open_skills_dir_sources_are_ignored() {
    let home = std::path::Path::new("/home/example");

    assert_eq!(
        resolve_open_skills_dir_from_sources(Some("  "), Some(" /config/open "), None),
        Some(PathBuf::from("/config/open"))
    );
    assert_eq!(
        resolve_open_skills_dir_from_sources(Some("\t"), Some("\n"), Some(home)),
        Some(PathBuf::from("/home/example/open-skills"))
    );
    assert_eq!(resolve_open_skills_dir_from_sources(Some(""), Some(""), None), None);
}

#[test]
fn sync_marker_controls_should_sync_decision() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(should_sync_open_skills(dir.path()));
    mark_open_skills_synced(dir.path()).unwrap();
    assert!(!should_sync_open_skills(dir.path()));
}

#[test]
fn expired_sync_marker_requires_sync() {
    let dir = tempfile::tempdir().expect("temp dir");
    mark_open_skills_synced(dir.path()).unwrap();
    let marker = dir.path().join(OPEN_SKILLS_SYNC_MARKER);
    let file = std::fs::OpenOptions::new().write(true).open(marker).unwrap();
    let times = std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH);
    file.set_times(times).unwrap();

    assert!(should_sync_open_skills(dir.path()));
}

#[test]
fn future_sync_marker_requires_sync_for_clock_skew() {
    let dir = tempfile::tempdir().expect("temp dir");
    mark_open_skills_synced(dir.path()).unwrap();
    let marker = dir.path().join(OPEN_SKILLS_SYNC_MARKER);
    let file = std::fs::OpenOptions::new().write(true).open(marker).unwrap();
    let future = std::time::SystemTime::now()
        + std::time::Duration::from_secs(OPEN_SKILLS_SYNC_INTERVAL_SECS);
    let times = std::fs::FileTimes::new().set_modified(future);
    file.set_times(times).unwrap();

    assert!(should_sync_open_skills(dir.path()));
}

#[test]
fn pull_open_skills_repo_accepts_non_git_directories() {
    let dir = tempfile::tempdir().expect("temp dir");
    assert!(pull_open_skills_repo(dir.path()));
}

#[test]
fn ensure_repo_respects_disabled_flag_and_accepts_existing_directory() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(ensure_open_skills_repo(Some(false), Some(dir.path().to_str().unwrap())).is_none());

    let resolved = ensure_open_skills_repo(Some(true), Some(dir.path().to_str().unwrap()));
    assert_eq!(resolved.as_deref(), Some(dir.path()));
    assert!(dir.path().join(OPEN_SKILLS_SYNC_MARKER).is_file());
    assert!(!should_sync_open_skills(dir.path()));
}

#[test]
fn ensure_repo_preserves_fresh_sync_marker_without_pulling() {
    let dir = tempfile::tempdir().expect("temp dir");
    let marker = dir.path().join(OPEN_SKILLS_SYNC_MARKER);
    std::fs::write(&marker, b"fresh marker").unwrap();

    let resolved = ensure_open_skills_repo(Some(true), Some(dir.path().to_str().unwrap()));

    assert_eq!(resolved.as_deref(), Some(dir.path()));
    assert_eq!(std::fs::read(marker).unwrap(), b"fresh marker");
}

#[test]
fn clone_open_skills_repo_returns_false_when_parent_cannot_be_created() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file_parent = dir.path().join("not-a-directory");
    std::fs::write(&file_parent, "file").unwrap();

    assert!(!clone_open_skills_repo(&file_parent.join("open-skills")));
}

#[test]
fn mark_open_skills_synced_reports_missing_directory_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing");

    assert!(mark_open_skills_synced(&missing).is_err());
}

#[test]
fn mark_open_skills_synced_writes_and_replaces_marker_content() {
    let dir = tempfile::tempdir().expect("temp dir");
    let marker = dir.path().join(OPEN_SKILLS_SYNC_MARKER);
    std::fs::write(&marker, b"outdated marker content").unwrap();

    mark_open_skills_synced(dir.path()).unwrap();

    assert_eq!(std::fs::read(marker).unwrap(), b"synced");
}
