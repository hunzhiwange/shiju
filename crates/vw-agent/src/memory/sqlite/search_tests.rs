use super::*;
use crate::memory::embeddings::EmbeddingProvider;
use async_trait::async_trait;
use std::sync::Arc;

fn memory_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    SqliteMemory::init_schema(&conn).unwrap();
    conn
}

fn insert_memory(
    conn: &Connection,
    id: &str,
    content: &str,
    category: &str,
    session_id: Option<&str>,
    embedding: Option<&[f32]>,
) {
    let embedding = embedding.map(vector::vec_to_bytes);
    conn.execute(
        "INSERT INTO memories
         (id, key, content, category, embedding, created_at, updated_at, session_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            id,
            content,
            category,
            embedding,
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            session_id
        ],
    )
    .unwrap();
}

struct FixedEmbedding;

#[async_trait]
impl EmbeddingProvider for FixedEmbedding {
    fn name(&self) -> &str {
        "fixed"
    }

    fn dimensions(&self) -> usize {
        2
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.25, 0.75]).collect())
    }
}

struct FailingEmbedding;

#[async_trait]
impl EmbeddingProvider for FailingEmbedding {
    fn name(&self) -> &str {
        "failing"
    }

    fn dimensions(&self) -> usize {
        2
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        anyhow::bail!("embedding rejected")
    }
}

#[test]
fn fts5_search_whitespace_returns_empty_without_accessing_schema() {
    let conn = Connection::open_in_memory().unwrap();

    let results = SqliteMemory::fts5_search(&conn, " \n\t ", 10).unwrap();

    assert!(results.is_empty());
}

#[test]
fn fts5_search_uses_or_terms_and_returns_positive_scores() {
    let conn = memory_connection();
    insert_memory(&conn, "rust", "rust only", "core", None, None);
    insert_memory(&conn, "sqlite", "sqlite only", "core", None, None);
    insert_memory(&conn, "both", "rust and sqlite", "core", None, None);

    let results = SqliteMemory::fts5_search(&conn, "rust sqlite", 10).unwrap();
    let mut ids = results.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>();
    ids.sort_unstable();

    assert_eq!(ids, vec!["both", "rust", "sqlite"]);
    assert!(results.iter().all(|(_, score)| *score > 0.0));
}

#[test]
fn fts5_search_respects_limit() {
    let conn = memory_connection();
    for id in ["one", "two", "three"] {
        insert_memory(&conn, id, "shared term", "core", None, None);
    }

    let results = SqliteMemory::fts5_search(&conn, "shared", 2).unwrap();

    assert_eq!(results.len(), 2);
}

#[test]
fn fts5_search_missing_schema_returns_error() {
    let conn = Connection::open_in_memory().unwrap();

    let result = SqliteMemory::fts5_search(&conn, "missing", 10);

    assert!(result.is_err());
}

#[test]
fn fts5_search_rejects_non_text_result_ids() {
    let conn = memory_connection();
    conn.execute(
        "INSERT INTO memories
         (id, key, content, category, created_at, updated_at, session_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            vec![0xff_u8],
            "invalid-id",
            "searchable needle",
            "core",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            Option::<String>::None,
        ],
    )
    .unwrap();

    assert!(SqliteMemory::fts5_search(&conn, "needle", 10).is_err());
}

#[test]
fn vector_search_sorts_positive_matches_and_discards_non_positive_scores() {
    let conn = memory_connection();
    insert_memory(&conn, "exact", "exact", "core", None, Some(&[1.0, 0.0]));
    insert_memory(&conn, "angled", "angled", "core", None, Some(&[1.0, 1.0]));
    insert_memory(&conn, "opposite", "opposite", "core", None, Some(&[-1.0, 0.0]));
    insert_memory(&conn, "wrong-size", "wrong-size", "core", None, Some(&[1.0]));

    let results = SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, None, None).unwrap();

    assert_eq!(
        results.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["exact", "angled"]
    );
    assert!(results[0].1 > results[1].1);
    assert!(results.iter().all(|(_, score)| *score > 0.0));
}

#[test]
fn vector_search_applies_category_and_session_filters_independently_and_together() {
    let conn = memory_connection();
    for (id, category, session_id) in [
        ("core-a", "core", "a"),
        ("core-b", "core", "b"),
        ("project-a", "project", "a"),
        ("project-b", "project", "b"),
    ] {
        insert_memory(&conn, id, id, category, Some(session_id), Some(&[1.0, 0.0]));
    }

    let category = SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, Some("core"), None).unwrap();
    let session = SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, None, Some("a")).unwrap();
    let combined =
        SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, Some("project"), Some("b")).unwrap();

    let mut category_ids = category.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>();
    category_ids.sort_unstable();
    let mut session_ids = session.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>();
    session_ids.sort_unstable();
    assert_eq!(category_ids, vec!["core-a", "core-b"]);
    assert_eq!(session_ids, vec!["core-a", "project-a"]);
    assert_eq!(combined[0].0, "project-b");
}

#[test]
fn vector_search_zero_limit_discards_scored_matches() {
    let conn = memory_connection();
    insert_memory(&conn, "match", "match", "core", None, Some(&[1.0, 0.0]));

    let results = SqliteMemory::vector_search(&conn, &[1.0, 0.0], 0, None, None).unwrap();

    assert!(results.is_empty());
}

#[test]
fn vector_search_missing_schema_returns_error() {
    let conn = Connection::open_in_memory().unwrap();

    let result = SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, None, None);

    assert!(result.is_err());
}

#[test]
fn vector_search_rejects_non_blob_embeddings() {
    let conn = memory_connection();
    conn.execute(
        "INSERT INTO memories
         (id, key, content, category, embedding, created_at, updated_at, session_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            "invalid-embedding",
            "invalid-embedding",
            "content",
            "core",
            42_i64,
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00Z",
            Option::<String>::None,
        ],
    )
    .unwrap();

    assert!(SqliteMemory::vector_search(&conn, &[1.0, 0.0], 10, None, None).is_err());
}

#[tokio::test]
async fn reindex_zero_dimension_embedder_rebuilds_fts_without_embeddings() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::new(workspace.path()).unwrap();
    {
        let conn = memory.conn.lock();
        conn.execute("DROP TRIGGER memories_ai", []).unwrap();
        insert_memory(&conn, "unindexed", "rebuild needle", "core", None, None);
        assert!(SqliteMemory::fts5_search(&conn, "needle", 10).unwrap().is_empty());
    }

    let count = memory.reindex().await.unwrap();
    let results = {
        let conn = memory.conn.lock();
        SqliteMemory::fts5_search(&conn, "needle", 10).unwrap()
    };

    assert_eq!(count, 0);
    assert_eq!(results[0].0, "unindexed");
}

#[tokio::test]
async fn reindex_returns_fts_rebuild_errors() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::new(workspace.path()).unwrap();
    memory.conn.lock().execute("DROP TABLE memories_fts", []).unwrap();

    let error = memory.reindex().await.unwrap_err().to_string();

    assert!(error.contains("memories_fts"));
}

#[tokio::test]
async fn reindex_fills_only_missing_embeddings_and_reports_count() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory =
        SqliteMemory::with_embedder(workspace.path(), Arc::new(FixedEmbedding), 0.7, 0.3, 10, None)
            .unwrap();
    {
        let conn = memory.conn.lock();
        insert_memory(&conn, "missing", "needs embedding", "core", None, None);
        insert_memory(&conn, "existing", "already embedded", "core", None, Some(&[1.0, 0.0]));
    }

    let count = memory.reindex().await.unwrap();
    let (missing, existing) = {
        let conn = memory.conn.lock();
        let missing = conn
            .query_row("SELECT embedding FROM memories WHERE id = 'missing'", [], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .unwrap();
        let existing = conn
            .query_row("SELECT embedding FROM memories WHERE id = 'existing'", [], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .unwrap();
        (vector::bytes_to_vec(&missing), vector::bytes_to_vec(&existing))
    };

    assert_eq!(count, 1);
    assert_eq!(missing, vec![0.25, 0.75]);
    assert_eq!(existing, vec![1.0, 0.0]);
}

#[tokio::test]
async fn reindex_skips_entries_whose_embedding_fails() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::with_embedder(
        workspace.path(),
        Arc::new(FailingEmbedding),
        0.7,
        0.3,
        10,
        None,
    )
    .unwrap();
    {
        let conn = memory.conn.lock();
        insert_memory(&conn, "missing", "cannot embed", "core", None, None);
    }

    let count = memory.reindex().await.unwrap();
    let missing_count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM memories WHERE embedding IS NULL", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 0);
    assert_eq!(missing_count, 1);
}

#[tokio::test]
async fn reindex_returns_embedding_update_errors() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory =
        SqliteMemory::with_embedder(workspace.path(), Arc::new(FixedEmbedding), 0.7, 0.3, 10, None)
            .unwrap();
    {
        let conn = memory.conn.lock();
        insert_memory(&conn, "blocked", "cannot update", "core", None, None);
        conn.execute_batch(
            "CREATE TRIGGER reject_embedding_update
             BEFORE UPDATE OF embedding ON memories
             BEGIN SELECT RAISE(FAIL, 'embedding update rejected'); END;",
        )
        .unwrap();
    }

    let error = memory.reindex().await.unwrap_err().to_string();

    assert!(error.contains("embedding update rejected"));
}
