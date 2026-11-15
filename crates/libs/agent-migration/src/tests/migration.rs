//! OpenClaw 记忆迁移流程的边界行为测试。
//!
//! 本文件覆盖冲突 key 重命名、预览模式只读语义、缺失源目录处理，以及迁移目标后端约束。
//! 这些场景直接关系到用户数据安全，因此测试以最小真实 SQLite fixture 验证端到端行为。

use super::super::*;
use super::test_config;
use agent_memory::{MemoryCategory, SqliteMemory};
use rusqlite::params;
use tempfile::TempDir;

#[tokio::test]
async fn migration_renames_conflicting_key() {
    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();

    let target_mem = SqliteMemory::new(target.path()).unwrap();
    target_mem.store("k", "new value", MemoryCategory::Core, None).await.unwrap();

    let source_db_dir = source.path().join("memory");
    std::fs::create_dir_all(&source_db_dir).unwrap();
    let source_db = source_db_dir.join("brain.db");
    let conn = Connection::open(&source_db).unwrap();
    conn.execute_batch("CREATE TABLE memories (key TEXT, content TEXT, category TEXT);").unwrap();
    conn.execute(
        "INSERT INTO memories (key, content, category) VALUES (?1, ?2, ?3)",
        params!["k", "old value", "core"],
    )
    .unwrap();

    let config = test_config(target.path());
    migrate_openclaw_memory(&config, Some(source.path().to_path_buf()), false).await.unwrap();

    let all = target_mem.list(None, None).await.unwrap();
    // 目标已有数据优先保留，源端冲突 key 通过后缀改名以避免覆盖用户现有记忆。
    assert!(all.iter().any(|e| e.key == "k" && e.content == "new value"));
    assert!(all.iter().any(|e| e.key.starts_with("k__openclaw_") && e.content == "old value"));
}

#[tokio::test]
async fn dry_run_does_not_write() {
    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    let source_db_dir = source.path().join("memory");
    std::fs::create_dir_all(&source_db_dir).unwrap();

    let source_db = source_db_dir.join("brain.db");
    let conn = Connection::open(&source_db).unwrap();
    conn.execute_batch("CREATE TABLE memories (key TEXT, content TEXT, category TEXT);").unwrap();
    conn.execute(
        "INSERT INTO memories (key, content, category) VALUES (?1, ?2, ?3)",
        params!["dry", "run", "core"],
    )
    .unwrap();

    let config = test_config(target.path());
    migrate_openclaw_memory(&config, Some(source.path().to_path_buf()), true).await.unwrap();

    let target_mem = SqliteMemory::new(target.path()).unwrap();
    // dry-run 是预览入口，必须保持目标库完全不写入。
    assert_eq!(target_mem.count().await.unwrap(), 0);
}

#[test]
fn missing_default_source_is_tolerated_for_preview() {
    let target = TempDir::new().unwrap();
    let config = test_config(target.path());
    let source = SourceWorkspace {
        path: target.path().join("missing-openclaw-workspace"),
        kind: SourceWorkspaceKind::Default,
    };

    let result = handle_missing_source_workspace(&config, &source, true);
    assert!(result.is_ok(), "dry-run preview should not fail for missing default source");
}

#[test]
fn missing_default_source_is_tolerated_for_execution() {
    let target = TempDir::new().unwrap();
    let config = test_config(target.path());
    let source = SourceWorkspace {
        path: target.path().join("missing-openclaw-workspace"),
        kind: SourceWorkspaceKind::Default,
    };

    let result = handle_missing_source_workspace(&config, &source, false);
    assert!(result.is_ok(), "default source absence should become a no-op");
}

#[test]
fn missing_explicit_source_still_fails() {
    let target = TempDir::new().unwrap();
    let config = test_config(target.path());
    let source = SourceWorkspace {
        path: target.path().join("missing-explicit-source"),
        kind: SourceWorkspaceKind::Explicit,
    };

    let err = handle_missing_source_workspace(&config, &source, true)
        .expect_err("explicit source must still error when missing");
    assert!(err.to_string().contains("OpenClaw workspace not found"));
}

#[test]
fn migration_target_rejects_none_backend() {
    let target = TempDir::new().unwrap();
    let mut config = test_config(target.path());
    config.memory.backend = "none".to_string();

    let err = target_memory_backend(&config)
        .err()
        .expect("backend=none should be rejected for migration target");
    // noop 后端没有持久化能力，作为迁移目标会造成数据“导入成功但不可保留”的假象。
    assert!(err.to_string().contains("disables persistence"));
}
