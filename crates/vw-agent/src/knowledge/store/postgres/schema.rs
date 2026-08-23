//! PostgreSQL 知识库表结构初始化与旧模式安全检查。

use anyhow::{Context, Result, bail};

use crate::global_database::GlobalDatabase;

const KNOWLEDGE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS knowledge_datasets (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    chunking_mode TEXT NOT NULL DEFAULT 'general',
    indexing_mode TEXT NOT NULL,
    retrieval_mode TEXT NOT NULL,
    keyword_count BIGINT NOT NULL DEFAULT 10,
    top_k BIGINT NOT NULL DEFAULT 10,
    score_threshold_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    score_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.15,
    rerank_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    embedding_model TEXT,
    rerank_model TEXT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, id)
);

CREATE TABLE IF NOT EXISTS knowledge_documents (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    status TEXT NOT NULL DEFAULT 'completed',
    error_message TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',
    char_count BIGINT NOT NULL DEFAULT 0,
    recall_count BIGINT NOT NULL DEFAULT 0,
    archived_at_ms BIGINT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY (platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_index_jobs (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT NOT NULL DEFAULT '',
    payload_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    started_at_ms BIGINT,
    attempt_count BIGINT NOT NULL DEFAULT 0,
    retry_after_ms BIGINT NOT NULL DEFAULT 0,
    lease_expires_at_ms BIGINT NOT NULL DEFAULT 0,
    finished_at_ms BIGINT,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY (platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, document_id)
        REFERENCES knowledge_documents(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_chunks (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    ordinal BIGINT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    chunk_type TEXT NOT NULL DEFAULT 'text',
    start_at BIGINT NOT NULL DEFAULT 0,
    end_at BIGINT NOT NULL DEFAULT 0,
    parent_chunk_id TEXT,
    prev_chunk_id TEXT,
    next_chunk_id TEXT,
    context_header TEXT NOT NULL DEFAULT '',
    index_content_hash TEXT NOT NULL DEFAULT '',
    embedding BYTEA,
    enabled BOOLEAN NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, id),
    FOREIGN KEY (platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, document_id)
        REFERENCES knowledge_documents(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_chunks_fts (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    search_vector TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('simple'::regconfig, COALESCE(title, '') || ' ' || COALESCE(content, ''))
    ) STORED,
    PRIMARY KEY (platform_id, company_id, chunk_id),
    FOREIGN KEY (platform_id, company_id, chunk_id)
        REFERENCES knowledge_chunks(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, dataset_id)
        REFERENCES knowledge_datasets(platform_id, company_id, id) ON DELETE CASCADE,
    FOREIGN KEY (platform_id, company_id, document_id)
        REFERENCES knowledge_documents(platform_id, company_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_embedding_cache (
    platform_id TEXT NOT NULL,
    company_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL,
    accessed_at_ms BIGINT NOT NULL,
    PRIMARY KEY (platform_id, company_id, content_hash)
);

CREATE INDEX IF NOT EXISTS knowledge_documents_dataset_idx
ON knowledge_documents(platform_id, company_id, dataset_id);
CREATE INDEX IF NOT EXISTS knowledge_index_jobs_dataset_idx
ON knowledge_index_jobs(platform_id, company_id, dataset_id);
CREATE INDEX IF NOT EXISTS knowledge_index_jobs_document_idx
ON knowledge_index_jobs(platform_id, company_id, document_id);
CREATE INDEX IF NOT EXISTS knowledge_index_jobs_status_idx
ON knowledge_index_jobs(platform_id, company_id, status, created_at_ms);
CREATE INDEX IF NOT EXISTS knowledge_chunks_dataset_idx
ON knowledge_chunks(platform_id, company_id, dataset_id);
CREATE INDEX IF NOT EXISTS knowledge_chunks_document_idx
ON knowledge_chunks(platform_id, company_id, document_id);
CREATE INDEX IF NOT EXISTS knowledge_chunks_fts_dataset_idx
ON knowledge_chunks_fts(platform_id, company_id, dataset_id);
CREATE INDEX IF NOT EXISTS knowledge_chunks_fts_document_idx
ON knowledge_chunks_fts(platform_id, company_id, document_id);
CREATE INDEX IF NOT EXISTS knowledge_chunks_fts_search_idx
ON knowledge_chunks_fts USING GIN(search_vector);
CREATE INDEX IF NOT EXISTS knowledge_embedding_cache_accessed_idx
ON knowledge_embedding_cache(platform_id, company_id, accessed_at_ms);
"#;

const KNOWLEDGE_TABLES: [&str; 6] = [
    "knowledge_datasets",
    "knowledge_documents",
    "knowledge_index_jobs",
    "knowledge_chunks",
    "knowledge_chunks_fts",
    "knowledge_embedding_cache",
];

/// 初始化租户化知识表，并拒绝可能导致跨租户读取的旧版无作用域表结构。
pub(super) fn initialize_schema(database: &GlobalDatabase) -> Result<()> {
    database.initialize_postgres_schema(|transaction| {
        reject_unscoped_schema(transaction)?;
        transaction
            .batch_execute(KNOWLEDGE_SCHEMA)
            .context("initialize PostgreSQL knowledge schema")?;
        Ok(())
    })
}

fn reject_unscoped_schema(connection: &mut impl ::postgres::GenericClient) -> Result<()> {
    // 旧表只要缺少任一租户键就拒绝启动，避免在原表上伪装完成安全迁移。
    for table in KNOWLEDGE_TABLES {
        let exists: bool = connection
            .query_one(
                "SELECT EXISTS(
                     SELECT 1 FROM information_schema.tables
                     WHERE table_schema = current_schema()
                       AND table_name = $1
                       AND table_type = 'BASE TABLE'
                 )",
                &[&table],
            )
            .context("inspect PostgreSQL knowledge table")?
            .get(0);
        if !exists {
            continue;
        }
        for column in ["platform_id", "company_id"] {
            let column_exists: bool = connection
                .query_one(
                    "SELECT EXISTS(
                         SELECT 1 FROM information_schema.columns
                         WHERE table_schema = current_schema()
                           AND table_name = $1
                           AND column_name = $2
                     )",
                    &[&table, &column],
                )
                .context("inspect PostgreSQL knowledge scope columns")?
                .get(0);
            if !column_exists {
                bail!(
                    "unsupported unscoped knowledge schema: table {table} must contain platform_id and company_id"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
