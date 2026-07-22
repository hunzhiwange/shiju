use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::patch::UpdateFileChunk;

use super::{Error, Hunk, apply_hunks_to_files, apply_patch, preview_changes};

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn enter(path: &std::path::Path) -> Self {
        let original = std::env::current_dir().expect("current dir should be readable");
        std::env::set_current_dir(path).expect("current dir should be changed");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).expect("current dir should be restored");
    }
}

fn replace_chunk(old_lines: &[&str], new_lines: &[&str]) -> UpdateFileChunk {
    UpdateFileChunk {
        old_lines: old_lines.iter().map(|line| line.to_string()).collect(),
        new_lines: new_lines.iter().map(|line| line.to_string()).collect(),
        change_context: None,
        is_end_of_file: None,
    }
}

#[test]
fn apply_hunks_to_files_rejects_empty_hunks() {
    let err = apply_hunks_to_files(&[], None).expect_err("empty hunks should be rejected");

    assert!(matches!(err, Error::Parse(message) if message == "No files were modified."));
}

#[test]
fn apply_hunks_to_files_add_plain_relative_file_without_parent() {
    let _lock = CWD_LOCK.lock().expect("cwd lock should be acquired");
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let _guard = CurrentDirGuard::enter(temp_dir.path());
    let hunks = vec![Hunk::Add { path: "plain.txt".to_string(), contents: "plain".to_string() }];

    let affected = apply_hunks_to_files(&hunks, None).expect("add hunk should be applied");

    assert_eq!(fs::read_to_string(temp_dir.path().join("plain.txt")).unwrap(), "plain");
    assert_eq!(affected.added, vec!["plain.txt"]);
}

#[test]
fn apply_hunks_to_files_add_creates_parent_directories() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let hunks = vec![Hunk::Add {
        path: "nested/example.txt".to_string(),
        contents: "hello\nworld".to_string(),
    }];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("add hunk should be applied");
    let file_path = temp_dir.path().join("nested/example.txt");

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "hello\nworld");
    assert_eq!(affected.added, vec![file_path.to_string_lossy().to_string()]);
    assert!(affected.modified.is_empty());
    assert!(affected.deleted.is_empty());
}

#[test]
fn apply_hunks_to_files_delete_removes_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let file_path = temp_dir.path().join("old.txt");
    fs::write(&file_path, "remove me").unwrap();
    let hunks = vec![Hunk::Delete { path: "old.txt".to_string() }];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("delete hunk should apply");

    assert!(!file_path.exists());
    assert_eq!(affected.deleted, vec![file_path.to_string_lossy().to_string()]);
    assert!(affected.added.is_empty());
    assert!(affected.modified.is_empty());
}

#[test]
fn apply_hunks_to_files_delete_missing_file_returns_io_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let hunks = vec![Hunk::Delete { path: "missing.txt".to_string() }];

    let err = apply_hunks_to_files(&hunks, Some(temp_dir.path()))
        .expect_err("missing delete target should fail");

    assert!(matches!(err, Error::Io(_)));
}

#[test]
fn apply_hunks_to_files_update_rewrites_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let file_path = temp_dir.path().join("note.txt");
    fs::write(&file_path, "alpha\nbeta\n").unwrap();
    let hunks = vec![Hunk::Update {
        path: "note.txt".to_string(),
        move_path: None,
        chunks: vec![replace_chunk(&["beta"], &["gamma"])],
    }];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("update hunk should apply");

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "alpha\ngamma\n");
    assert_eq!(affected.modified, vec![file_path.to_string_lossy().to_string()]);
    assert!(affected.added.is_empty());
    assert!(affected.deleted.is_empty());
}

#[test]
fn apply_hunks_to_files_update_move_writes_target_and_removes_source() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("moved/target.txt");
    fs::write(&source_path, "one\ntwo\n").unwrap();
    let hunks = vec![Hunk::Update {
        path: "source.txt".to_string(),
        move_path: Some("moved/target.txt".to_string()),
        chunks: vec![replace_chunk(&["two"], &["three"])],
    }];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("move update should apply");

    assert!(!source_path.exists());
    assert_eq!(fs::read_to_string(&target_path).unwrap(), "one\nthree\n");
    assert_eq!(affected.modified, vec![target_path.to_string_lossy().to_string()]);
    assert!(affected.added.is_empty());
    assert!(affected.deleted.is_empty());
}

#[test]
fn apply_hunks_to_files_update_move_to_plain_relative_target_without_parent() {
    let _lock = CWD_LOCK.lock().expect("cwd lock should be acquired");
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let _guard = CurrentDirGuard::enter(temp_dir.path());
    fs::write("source.txt", "one\ntwo\n").unwrap();
    let hunks = vec![Hunk::Update {
        path: "source.txt".to_string(),
        move_path: Some("target.txt".to_string()),
        chunks: vec![replace_chunk(&["two"], &["three"])],
    }];

    let affected = apply_hunks_to_files(&hunks, None).expect("move update should apply");

    assert!(!temp_dir.path().join("source.txt").exists());
    assert_eq!(fs::read_to_string(temp_dir.path().join("target.txt")).unwrap(), "one\nthree\n");
    assert_eq!(affected.modified, vec!["target.txt"]);
}

#[test]
fn apply_hunks_to_files_update_missing_expected_lines_returns_compute_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("note.txt"), "alpha\nbeta\n").unwrap();
    let hunks = vec![Hunk::Update {
        path: "note.txt".to_string(),
        move_path: None,
        chunks: vec![replace_chunk(&["delta"], &["gamma"])],
    }];

    let err = apply_hunks_to_files(&hunks, Some(temp_dir.path()))
        .expect_err("unmatched update hunk should fail");

    assert!(
        matches!(err, Error::ComputeReplacements(message) if message.contains("Failed to find expected lines"))
    );
}

#[test]
fn apply_patch_parses_and_applies_add_file_patch() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let patch = r#"
*** Begin Patch
*** Add File: created.txt
+created
*** End Patch
"#;

    let affected = apply_patch(patch, Some(temp_dir.path())).expect("patch should apply");
    let file_path = temp_dir.path().join("created.txt");

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "created");
    assert_eq!(affected.added, vec![file_path.to_string_lossy().to_string()]);
}

#[test]
fn apply_patch_invalid_input_returns_parse_error() {
    let err = apply_patch("not a patch", None).expect_err("invalid patch should fail");

    assert!(matches!(err, Error::Parse(message) if message.contains("missing Begin/End")));
}

#[test]
fn preview_changes_reports_absolute_add_without_root_joining() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let file_path = temp_dir.path().join("absolute.txt");
    let path_text = file_path.to_string_lossy();
    let patch = format!("*** Begin Patch\n*** Add File: {}\n+absolute\n*** End Patch\n", path_text);

    let preview = preview_changes(&patch, Some(temp_dir.path())).expect("preview should be built");

    assert_eq!(preview["changes"][path_text.as_ref()]["type"], "add");
    assert_eq!(preview["changes"][path_text.as_ref()]["content"], "absolute");
}

#[test]
fn preview_changes_reports_relative_add_without_root() {
    let patch = r#"
*** Begin Patch
*** Add File: relative.txt
+preview
*** End Patch
"#;

    let preview = preview_changes(patch, None).expect("preview should be built");

    assert_eq!(preview["cwd"], serde_json::Value::Null);
    assert_eq!(preview["changes"]["relative.txt"]["type"], "add");
    assert_eq!(preview["changes"]["relative.txt"]["content"], "preview");
    assert_eq!(preview["patch"], patch);
}

#[test]
fn preview_changes_reports_delete_content() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("stale.txt"), "old content").unwrap();
    let patch = r#"
*** Begin Patch
*** Delete File: stale.txt
*** End Patch
"#;

    let preview = preview_changes(patch, Some(temp_dir.path())).expect("preview should be built");
    let file_path = temp_dir.path().join("stale.txt").to_string_lossy().to_string();

    assert_eq!(preview["cwd"], temp_dir.path().to_string_lossy().to_string());
    assert_eq!(preview["changes"][&file_path]["type"], "delete");
    assert_eq!(preview["changes"][&file_path]["content"], "old content");
}

#[test]
fn preview_changes_reports_update_diff_and_new_content() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("note.txt"), "alpha\nbeta\n").unwrap();
    let patch = r#"
*** Begin Patch
*** Update File: note.txt
@@
-beta
+gamma
*** End Patch
"#;

    let preview = preview_changes(patch, Some(temp_dir.path())).expect("preview should be built");
    let file_path = temp_dir.path().join("note.txt").to_string_lossy().to_string();

    assert_eq!(preview["changes"][&file_path]["type"], "update");
    assert_eq!(preview["changes"][&file_path]["move_path"], serde_json::Value::Null);
    assert_eq!(preview["changes"][&file_path]["new_content"], "alpha\ngamma\n");
    assert!(preview["changes"][&file_path]["unified_diff"].as_str().unwrap().contains("+gamma"));
}

#[test]
fn preview_changes_reports_move_target() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("source.txt"), "one\ntwo\n").unwrap();
    let patch = r#"
*** Begin Patch
*** Update File: source.txt
*** Move to: moved/target.txt
@@
-two
+three
*** End Patch
"#;

    let preview = preview_changes(patch, Some(temp_dir.path())).expect("preview should be built");
    let target_path = temp_dir.path().join("moved/target.txt").to_string_lossy().to_string();

    assert_eq!(preview["changes"][&target_path]["type"], "update");
    assert_eq!(preview["changes"][&target_path]["move_path"], target_path);
    assert_eq!(preview["changes"][&target_path]["new_content"], "one\nthree\n");
}

#[test]
fn apply_hunks_to_files_add_absolute_path_ignores_root() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let other_dir = tempfile::tempdir().expect("temp dir should be created");
    let file_path = other_dir.path().join("absolute.txt");
    let hunks = vec![Hunk::Add {
        path: file_path.to_string_lossy().to_string(),
        contents: "abs".to_string(),
    }];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("absolute add should apply");

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "abs");
    assert_eq!(affected.added, vec![file_path.to_string_lossy().to_string()]);
}

#[test]
fn apply_hunks_to_files_handles_mixed_add_update_delete() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("note.txt"), "alpha\nbeta\n").unwrap();
    fs::write(temp_dir.path().join("gone.txt"), "remove").unwrap();
    let hunks = vec![
        Hunk::Add { path: "fresh.txt".to_string(), contents: "fresh".to_string() },
        Hunk::Update {
            path: "note.txt".to_string(),
            move_path: None,
            chunks: vec![replace_chunk(&["beta"], &["gamma"])],
        },
        Hunk::Delete { path: "gone.txt".to_string() },
    ];

    let affected =
        apply_hunks_to_files(&hunks, Some(temp_dir.path())).expect("mixed hunks should apply");

    assert_eq!(
        affected.added,
        vec![temp_dir.path().join("fresh.txt").to_string_lossy().to_string()]
    );
    assert_eq!(
        affected.modified,
        vec![temp_dir.path().join("note.txt").to_string_lossy().to_string()]
    );
    assert_eq!(
        affected.deleted,
        vec![temp_dir.path().join("gone.txt").to_string_lossy().to_string()]
    );
}

#[test]
fn apply_patch_applies_relative_without_root() {
    let _lock = CWD_LOCK.lock().expect("cwd lock should be acquired");
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let _guard = CurrentDirGuard::enter(temp_dir.path());
    let patch = r#"
*** Begin Patch
*** Add File: rel.txt
+rel
*** End Patch
"#;

    let affected = apply_patch(patch, None).expect("patch should apply");

    assert_eq!(fs::read_to_string(temp_dir.path().join("rel.txt")).unwrap(), "rel");
    assert_eq!(affected.added, vec!["rel.txt"]);
}

#[test]
fn preview_changes_invalid_input_returns_parse_error() {
    let err = preview_changes("not a patch", None).expect_err("invalid patch should fail");

    assert!(matches!(err, Error::Parse(_)));
}

#[test]
fn preview_changes_delete_missing_file_returns_io_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let patch = r#"
*** Begin Patch
*** Delete File: missing.txt
*** End Patch
"#;

    let err = preview_changes(patch, Some(temp_dir.path()))
        .expect_err("missing delete target should fail");

    assert!(matches!(err, Error::Io(_)));
}

#[test]
fn preview_changes_update_missing_lines_returns_compute_error() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(temp_dir.path().join("note.txt"), "alpha\nbeta\n").unwrap();
    let patch = r#"
*** Begin Patch
*** Update File: note.txt
@@
-delta
+gamma
*** End Patch
"#;

    let err = preview_changes(patch, Some(temp_dir.path()))
        .expect_err("unmatched update hunk should fail");

    assert!(matches!(err, Error::ComputeReplacements(_)));
}
