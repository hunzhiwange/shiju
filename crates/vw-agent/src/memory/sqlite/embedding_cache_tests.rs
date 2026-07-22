use super::*;
use crate::app::agent::memory::embeddings::EmbeddingProvider;
use crate::memory::vector;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn embedding_cache_module_is_linked() {
    assert!(std::any::type_name::<SqliteMemory>().contains("SqliteMemory"));
}

struct CountingEmbedding {
    calls: AtomicUsize,
}

#[async_trait]
impl EmbeddingProvider for CountingEmbedding {
    fn name(&self) -> &str {
        "counting"
    }

    fn dimensions(&self) -> usize {
        2
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(texts.iter().map(|text| vec![text.len() as f32, 1.0]).collect())
    }
}

struct FailingEmbedding {
    calls: AtomicUsize,
}

#[async_trait]
impl EmbeddingProvider for FailingEmbedding {
    fn name(&self) -> &str {
        "failing"
    }

    fn dimensions(&self) -> usize {
        2
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("embedding failed")
    }
}

struct EmptyEmbedding;

#[async_trait]
impl EmbeddingProvider for EmptyEmbedding {
    fn name(&self) -> &str {
        "empty"
    }

    fn dimensions(&self) -> usize {
        2
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn zero_dimension_embedder_skips_cache_and_embedding_call() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::new(workspace.path()).unwrap();
    memory.conn.lock().execute("DROP TABLE embedding_cache", []).unwrap();

    let result = memory.get_or_compute_embedding("hello").await.unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn get_or_compute_embedding_caches_and_reuses_result() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();

    let first = memory.get_or_compute_embedding("hello").await.unwrap().unwrap();
    let second = memory.get_or_compute_embedding("hello").await.unwrap().unwrap();

    assert_eq!(first, vec![5.0, 1.0]);
    assert_eq!(second, first);
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn empty_text_is_cached_and_reused() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();

    let first = memory.get_or_compute_embedding("").await.unwrap().unwrap();
    let second = memory.get_or_compute_embedding("").await.unwrap().unwrap();

    assert_eq!(first, vec![0.0, 1.0]);
    assert_eq!(second, first);
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn get_or_compute_embedding_caches_different_texts_independently() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();

    let short = memory.get_or_compute_embedding("a").await.unwrap().unwrap();
    let long = memory.get_or_compute_embedding("longer").await.unwrap().unwrap();

    assert_eq!(short, vec![1.0, 1.0]);
    assert_eq!(long, vec![6.0, 1.0]);
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 2);
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn cached_embedding_is_decoded_and_refreshes_access_time() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();
    let hash = SqliteMemory::content_hash("cached");
    memory
        .conn
        .lock()
        .execute(
            "INSERT INTO embedding_cache (content_hash, embedding, created_at, accessed_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                hash,
                vector::vec_to_bytes(&[3.5, -2.0]),
                "2000-01-01T00:00:00Z",
                "2000-01-01T00:00:00Z"
            ],
        )
        .unwrap();

    let result = memory.get_or_compute_embedding("cached").await.unwrap().unwrap();

    assert_eq!(result, vec![3.5, -2.0]);
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 0);
    let accessed_at: String = memory
        .conn
        .lock()
        .query_row(
            "SELECT accessed_at FROM embedding_cache WHERE content_hash = ?1",
            rusqlite::params![SqliteMemory::content_hash("cached")],
            |row| row.get(0),
        )
        .unwrap();
    assert_ne!(accessed_at, "2000-01-01T00:00:00Z");
}

#[tokio::test]
async fn cache_query_error_is_returned_before_embedding_is_called() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();
    memory.conn.lock().execute("DROP TABLE embedding_cache", []).unwrap();

    let error = memory.get_or_compute_embedding("hello").await.unwrap_err().to_string();

    assert!(error.contains("no such table"));
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cache_access_time_update_error_is_returned_without_recomputing() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();
    memory
        .conn
        .lock()
        .execute(
            "INSERT INTO embedding_cache (content_hash, embedding, created_at, accessed_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                SqliteMemory::content_hash("cached"),
                vector::vec_to_bytes(&[1.0, 2.0]),
                "2000-01-01T00:00:00Z",
                "2000-01-01T00:00:00Z"
            ],
        )
        .unwrap();
    memory
        .conn
        .lock()
        .execute_batch(
            "CREATE TRIGGER reject_embedding_cache_update
             BEFORE UPDATE ON embedding_cache
             BEGIN SELECT RAISE(FAIL, 'cache update rejected'); END;",
        )
        .unwrap();

    let error = memory.get_or_compute_embedding("cached").await.unwrap_err().to_string();

    assert!(error.contains("cache update rejected"));
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn embedding_provider_error_is_returned_without_cache_entry() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(FailingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();

    let error = memory.get_or_compute_embedding("hello").await.unwrap_err().to_string();

    assert_eq!(error, "embedding failed");
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn empty_embedding_result_is_returned_without_cache_entry() {
    let workspace = tempfile::TempDir::new().unwrap();
    let memory = SqliteMemory::with_embedder(
        workspace.path(),
        Arc::new(EmptyEmbedding),
        0.7,
        0.3,
        100,
        None,
    )
    .unwrap();

    let error = memory.get_or_compute_embedding("hello").await.unwrap_err().to_string();

    assert_eq!(error, "Empty embedding result");
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn cache_insert_error_is_returned_after_embedding_is_computed() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 100, None)
            .unwrap();
    memory
        .conn
        .lock()
        .execute_batch(
            "CREATE TRIGGER reject_embedding_cache_insert
             BEFORE INSERT ON embedding_cache
             BEGIN SELECT RAISE(FAIL, 'cache insert rejected'); END;",
        )
        .unwrap();

    let error = memory.get_or_compute_embedding("hello").await.unwrap_err().to_string();

    assert!(error.contains("cache insert rejected"));
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn zero_capacity_cache_returns_embedding_without_retaining_it() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 0, None).unwrap();

    let result = memory.get_or_compute_embedding("hello").await.unwrap().unwrap();

    assert_eq!(result, vec![5.0, 1.0]);
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn cache_eviction_error_is_returned_after_inserting_embedding() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 0, None).unwrap();
    memory
        .conn
        .lock()
        .execute_batch(
            "CREATE TRIGGER reject_embedding_cache_delete
             BEFORE DELETE ON embedding_cache
             BEGIN SELECT RAISE(FAIL, 'cache delete rejected'); END;",
        )
        .unwrap();

    let error = memory.get_or_compute_embedding("hello").await.unwrap_err().to_string();

    assert!(error.contains("cache delete rejected"));
    assert_eq!(embedder.calls.load(Ordering::SeqCst), 1);
    let count: i64 = memory
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn get_or_compute_embedding_evicts_old_entries_when_cache_is_full() {
    let workspace = tempfile::TempDir::new().unwrap();
    let embedder = Arc::new(CountingEmbedding { calls: AtomicUsize::new(0) });
    let memory =
        SqliteMemory::with_embedder(workspace.path(), embedder.clone(), 0.7, 0.3, 1, None).unwrap();

    memory.get_or_compute_embedding("one").await.unwrap();
    memory
        .conn
        .lock()
        .execute("UPDATE embedding_cache SET accessed_at = '2000-01-01T00:00:00Z'", [])
        .unwrap();
    memory.get_or_compute_embedding("two").await.unwrap();

    let conn = memory.conn.lock();
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM embedding_cache", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
    let remaining_hash: String =
        conn.query_row("SELECT content_hash FROM embedding_cache", [], |row| row.get(0)).unwrap();
    assert_eq!(remaining_hash, SqliteMemory::content_hash("two"));
}
