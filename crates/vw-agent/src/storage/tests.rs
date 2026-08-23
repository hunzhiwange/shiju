//! 验证通用存储键值操作、路径隔离与持久化行为。
//! 文件系统夹具覆盖序列化失败、原子更新和非法目标类型，错误时原 JSON 必须保持不变。

use super::*;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
struct AlwaysFailsToSerialize;

#[cfg(not(target_arch = "wasm32"))]
impl serde::Serialize for AlwaysFailsToSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("intentional serialization failure"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, serde::Deserialize)]
struct DeserializesButFailsToSerialize {
    count: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl serde::Serialize for DeserializesButFailsToSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("intentional serialization failure"))
    }
}

#[test]
fn not_found_error_display_uses_message_only() {
    let err = NotFoundError { message: "missing thing".to_string() };
    assert_eq!(err.to_string(), "missing thing");
}

#[test]
fn error_display_delegates_to_inner_error() {
    let err = Error::NotFound(NotFoundError { message: "gone".to_string() });
    assert_eq!(err.to_string(), "gone");

    let io = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "io boom"));
    assert_eq!(io.to_string(), "io boom");

    let json = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    assert!(Error::Json(json).to_string().contains("EOF"));
}

#[test]
fn error_from_conversions_preserve_source_variants() {
    let io = Error::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"));
    assert!(matches!(io, Error::Io(inner) if inner.kind() == std::io::ErrorKind::PermissionDenied));

    let json = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    assert!(matches!(Error::from(json), Error::Json(_)));
}

#[tokio::test]
async fn error_from_join_error_preserves_join_variant() {
    let task = tokio::spawn(std::future::pending::<()>());
    task.abort();
    let join = task.await.expect_err("aborted task should return a join error");
    let err = Error::from(join);

    assert!(err.to_string().contains("cancel"));
    assert!(matches!(err, Error::Join(inner) if inner.is_cancelled()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn now_ms_returns_epoch_milliseconds() {
    assert!(now_ms() > 1_000_000_000_000);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn storage_root_dir_uses_home_vibewindow_directory() {
    let home = Path::new("/tmp/vw-home");
    let path = storage_root_dir_for_home(home);

    assert_eq!(path, home.join(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("storage"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn target_path_joins_segments_and_sets_json_extension() {
    let path = target_path(Path::new("/tmp/storage"), &["project", "alpha"]);

    assert_eq!(path, Path::new("/tmp/storage").join("project").join("alpha.json"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn target_path_handles_empty_keys_and_replaces_existing_extensions() {
    assert_eq!(target_path(Path::new("/tmp/storage"), &[]), Path::new("/tmp/storage.json"));
    assert_eq!(
        target_path(Path::new("/tmp/storage"), &["record.backup"]),
        Path::new("/tmp/storage").join("record.json")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn map_io_error_maps_not_found_to_storage_error() {
    let target = Path::new("/tmp/missing.json");
    let err = map_io_error(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"), target);

    match err {
        Error::NotFound(not_found) => {
            assert!(not_found.message.contains("Resource not found"));
            assert!(not_found.message.contains("missing.json"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn map_io_error_preserves_non_not_found_errors() {
    let err = map_io_error(
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        Path::new("/tmp/inaccessible.json"),
    );

    assert!(
        matches!(err, Error::Io(inner) if inner.kind() == std::io::ErrorKind::PermissionDenied)
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn not_found_for_includes_the_exact_resource_path() {
    let path = Path::new("/tmp/vibe-window/missing resource.json");
    let error = not_found_for(path);

    assert!(matches!(&error, Error::NotFound(_)));
    assert_eq!(error.to_string(), format!("Resource not found: {}", path.display()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn list_returns_empty_for_missing_prefix() {
    let run_id = Uuid::new_v4().to_string();
    let prefix = ["test-storage-missing", run_id.as_str()];

    assert!(list(&prefix).await.expect("missing prefix should list").is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn write_read_update_list_and_remove_round_trip() {
    let run_id = Uuid::new_v4().to_string();
    let prefix = ["test-storage", run_id.as_str()];
    let first = ["test-storage", run_id.as_str(), "nested", "first"];
    let second = ["test-storage", run_id.as_str(), "second"];

    write(&first, &json!({"name": "Ada", "count": 1})).await.expect("first write should succeed");
    write(&second, &json!({"name": "Grace"})).await.expect("second write should succeed");

    let state_dir = state().await.dir.clone();
    let prefix_dir = state_dir.join("test-storage").join(&run_id);
    tokio::fs::write(prefix_dir.join("ignored.txt"), "not json")
        .await
        .expect("non-json fixture should write");

    let read_back: serde_json::Value = read(&first).await.expect("first value should read");
    assert_eq!(read_back["name"], "Ada");

    let updated: serde_json::Value = update(&first, |value: &mut serde_json::Value| {
        value["count"] = json!(2);
    })
    .await
    .expect("update should rewrite JSON");
    assert_eq!(updated["count"], 2);

    let keys = list(&prefix).await.expect("list should succeed");
    assert_eq!(
        keys,
        vec![
            vec![
                "test-storage".to_string(),
                run_id.clone(),
                "nested".to_string(),
                "first".to_string()
            ],
            vec!["test-storage".to_string(), run_id.clone(), "second".to_string()]
        ]
    );

    remove(&first).await.expect("remove existing should succeed");
    let err = read::<serde_json::Value>(&first).await.expect_err("removed key should be missing");
    assert!(matches!(err, Error::NotFound(_)));

    remove(&first).await.expect("remove missing should stay idempotent");
    remove(&second).await.expect("cleanup should succeed");
    assert!(list(&prefix).await.expect("empty prefix should list").is_empty());
    tokio::fs::remove_dir_all(prefix_dir).await.expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn write_reports_serialization_errors_without_creating_target() {
    let run_id = Uuid::new_v4().to_string();
    let prefix = ["test-storage", run_id.as_str()];
    let key = ["test-storage", run_id.as_str(), "serialization-failure"];
    let state_dir = state().await.dir.clone();
    let target = target_path(&state_dir, &key);

    let err = write(&key, &AlwaysFailsToSerialize)
        .await
        .expect_err("serialization failure should be reported");

    assert!(matches!(err, Error::Json(_)));
    assert!(!target.exists());
    tokio::fs::remove_dir_all(state_dir.join(prefix[0]).join(prefix[1]))
        .await
        .expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn write_reports_parent_path_file_as_io_error() {
    let run_id = Uuid::new_v4().to_string();
    let key = ["test-storage", run_id.as_str(), "blocking-file", "value"];
    let state_dir = state().await.dir.clone();
    let prefix_dir = state_dir.join("test-storage").join(&run_id);
    tokio::fs::create_dir_all(&prefix_dir).await.expect("fixture directory should create");
    tokio::fs::write(prefix_dir.join("blocking-file"), "not a directory")
        .await
        .expect("blocking file should write");

    let err = write(&key, &json!({"value": true}))
        .await
        .expect_err("a file used as the parent path should fail");

    assert!(matches!(err, Error::Io(_)));
    tokio::fs::remove_dir_all(prefix_dir).await.expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
// 更新失败后重新读取原值，证明序列化发生在替换文件之前。
async fn update_serialization_error_leaves_original_json_unchanged() {
    let run_id = Uuid::new_v4().to_string();
    let key = ["test-storage", run_id.as_str(), "update-serialization-failure"];
    let state_dir = state().await.dir.clone();
    write(&key, &json!({"count": 1})).await.expect("initial value should write");

    let err = update::<DeserializesButFailsToSerialize>(&key, |value| value.count = 2)
        .await
        .expect_err("serialization failure should be reported");
    assert!(matches!(err, Error::Json(_)));

    let unchanged: serde_json::Value = read(&key).await.expect("original value should remain");
    assert_eq!(unchanged, json!({"count": 1}));
    remove(&key).await.expect("fixture should clean up");
    tokio::fs::remove_dir_all(state_dir.join("test-storage").join(&run_id))
        .await
        .expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn read_and_remove_report_non_file_targets_as_io_errors() {
    let run_id = Uuid::new_v4().to_string();
    let key = ["test-storage", run_id.as_str(), "directory-target"];
    let state_dir = state().await.dir.clone();
    let target = target_path(&state_dir, &key);
    tokio::fs::create_dir_all(&target).await.expect("directory fixture should create");

    let read_err = read::<serde_json::Value>(&key)
        .await
        .expect_err("reading a directory should fail as an I/O error");
    assert!(matches!(read_err, Error::Io(_)));

    let remove_err =
        remove(&key).await.expect_err("removing a directory through remove_file should fail");
    assert!(matches!(remove_err, Error::Io(_)));

    tokio::fs::remove_dir_all(state_dir.join("test-storage").join(&run_id))
        .await
        .expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn read_and_update_report_missing_or_invalid_json() {
    let run_id = Uuid::new_v4().to_string();
    let missing = ["test-storage", run_id.as_str(), "missing"];
    let invalid = ["test-storage", run_id.as_str(), "invalid"];

    let err =
        read::<serde_json::Value>(&missing).await.expect_err("missing key should return NotFound");
    assert!(matches!(err, Error::NotFound(_)));

    write(&invalid, &json!({"ok": true})).await.expect("initial write should succeed");
    let state_dir = state().await.dir.clone();
    let path = target_path(&state_dir, &invalid);
    tokio::fs::write(&path, "{").await.expect("invalid JSON should write");

    let err =
        read::<serde_json::Value>(&invalid).await.expect_err("invalid JSON should be reported");
    assert!(matches!(err, Error::Json(_)));

    let err = update::<serde_json::Value>(&invalid, |_| {})
        .await
        .expect_err("updating invalid JSON should be reported");
    assert!(matches!(err, Error::Json(_)));

    let err = update::<serde_json::Value>(&missing, |_| {})
        .await
        .expect_err("missing update should return NotFound");
    assert!(matches!(err, Error::NotFound(_)));

    remove(&invalid).await.expect("cleanup should succeed");
    tokio::fs::remove_dir_all(state_dir.join("test-storage").join(&run_id))
        .await
        .expect("fixture directory should clean up");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
// 原生存储迁移只读取有效项目目录；无提交仓库没有可用的根提交标识。
fn migration_helpers_handle_absent_project_dir_and_root_commit() {
    let temp = TempDir::new().expect("tempdir should create");
    migration_0_blocking(temp.path()).expect("missing legacy project dir should be ignored");

    let repo_dir = temp.path().join("repo");
    std::fs::create_dir_all(&repo_dir).expect("repo dir should create");
    let repo = Repository::init(&repo_dir).expect("repo should init");
    let sig = git2::Signature::now("Vibe Window", "vw@example.com").expect("signature");
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree should load");
    let commit =
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).expect("commit should write");

    assert_eq!(root_commit_id(&repo), Some(commit.to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn root_commit_id_returns_none_for_repository_without_commits() {
    let temp = TempDir::new().expect("tempdir should create");
    let repo = Repository::init(temp.path()).expect("repo should init");

    assert_eq!(root_commit_id(&repo), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn root_commit_id_selects_lexicographically_first_root_across_refs() {
    let temp = TempDir::new().expect("tempdir should create");
    let repo = Repository::init(temp.path()).expect("repo should init");
    let sig = git2::Signature::now("Vibe Window", "vw@example.com").expect("signature");
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree should load");
    let first = repo
        .commit(Some("refs/heads/first"), &sig, &sig, "first root", &tree, &[])
        .expect("first root should commit");
    let second = repo
        .commit(Some("refs/heads/second"), &sig, &sig, "second root", &tree, &[])
        .expect("second root should commit");
    let expected = [first.to_string(), second.to_string()]
        .into_iter()
        .min()
        .expect("two roots should have a minimum");

    assert_eq!(root_commit_id(&repo), Some(expected));
}
