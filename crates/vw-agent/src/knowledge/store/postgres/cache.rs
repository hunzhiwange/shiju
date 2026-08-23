//! PostgreSQL 知识向量缓存的读写与淘汰操作。

use anyhow::{Context, Result};
use vw_api_types::TenantScope;

use super::super::now_ms;
use super::PostgresKnowledgeStore;
use super::datasets::i64_from_u64;

impl PostgresKnowledgeStore {
    /// 读取租户内的向量缓存，并在命中时刷新最近访问时间。
    pub(in crate::knowledge::store) fn read_embedding_cache(
        &self,
        tenant: &TenantScope,
        content_hash: &str,
    ) -> Result<Option<Vec<u8>>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge embedding cache read")?;
        let embedding = transaction
            .query_opt(
                "SELECT embedding FROM knowledge_embedding_cache
                 WHERE platform_id = $1 AND company_id = $2 AND content_hash = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &content_hash],
            )
            .context("read tenant PostgreSQL knowledge embedding cache")?
            .map(|row| row.get::<_, Vec<u8>>(0));
        if embedding.is_some() {
            transaction
                .execute(
                    "UPDATE knowledge_embedding_cache SET accessed_at_ms = $1
                     WHERE platform_id = $2 AND company_id = $3 AND content_hash = $4",
                    &[
                        &i64_from_u64(now_ms()),
                        &tenant.platform_id().as_ref(),
                        &tenant.company_id().as_ref(),
                        &content_hash,
                    ],
                )
                .context("touch tenant PostgreSQL knowledge embedding cache")?;
        }
        transaction.commit().context("commit PostgreSQL knowledge embedding cache read")?;
        Ok(embedding)
    }

    /// 写入或更新租户向量缓存，并按容量上限淘汰最久未访问记录。
    pub(in crate::knowledge::store) fn write_embedding_cache(
        &self,
        tenant: &TenantScope,
        content_hash: &str,
        embedding: Vec<u8>,
        cache_size: usize,
    ) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge embedding cache write")?;
        let now = i64_from_u64(now_ms());
        transaction
            .execute(
                "INSERT INTO knowledge_embedding_cache
                 (platform_id, company_id, content_hash, embedding, created_at_ms, accessed_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (platform_id, company_id, content_hash) DO UPDATE
                 SET embedding = EXCLUDED.embedding, created_at_ms = EXCLUDED.created_at_ms,
                     accessed_at_ms = EXCLUDED.accessed_at_ms",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &content_hash,
                    &embedding,
                    &now,
                    &now,
                ],
            )
            .context("write tenant PostgreSQL knowledge embedding cache")?;
        let cache_size = i64::try_from(cache_size).unwrap_or(i64::MAX);
        transaction
            .execute(
                "DELETE FROM knowledge_embedding_cache
                 WHERE platform_id = $1 AND company_id = $2 AND content_hash IN (
                     SELECT content_hash FROM knowledge_embedding_cache
                     WHERE platform_id = $1 AND company_id = $2
                     ORDER BY accessed_at_ms DESC, content_hash ASC
                     OFFSET $3
                 )",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &cache_size],
            )
            .context("prune tenant PostgreSQL knowledge embedding cache")?;
        transaction.commit().context("commit PostgreSQL knowledge embedding cache write")
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
