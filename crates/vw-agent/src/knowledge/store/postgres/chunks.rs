//! PostgreSQL 知识分块的查询、替换与行映射逻辑。

use anyhow::{Context, Result, bail};
use postgres::{GenericClient, Row};
use serde_json::Value;
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkDto, KnowledgeChunkType, KnowledgeDocumentCreateRequest, KnowledgeDocumentDto,
    KnowledgeDocumentStatus, KnowledgeMatchType,
};

use super::super::{
    PreparedKnowledgeChunk, ReindexKnowledgeDocument, StoredKnowledgeDocument, chunk_metadata,
    chunk_type_as_str, content_hash, document_status_as_str, final_document_status, next_link,
    now_ms, parse_chunk_type, previous_link,
};
use super::PostgresKnowledgeStore;
use super::datasets::i64_from_u64;

impl PostgresKnowledgeStore {
    /// 列出指定文档的有序分块；文档不存在时返回 `None`。
    pub(in crate::knowledge::store) fn list_chunks(
        &self,
        tenant: &TenantScope,
        document_id: &str,
    ) -> Result<Option<Vec<KnowledgeChunkDto>>> {
        if self.get_document(tenant, document_id)?.is_none() {
            return Ok(None);
        }
        let mut connection = self.database.connect_postgres()?;
        let rows = connection
            .query(
                "SELECT id, dataset_id, document_id, title, content, metadata_json, ordinal,
                        chunk_type, start_at, end_at, parent_chunk_id, prev_chunk_id, next_chunk_id,
                        context_header
                 FROM knowledge_chunks
                 WHERE platform_id = $1 AND company_id = $2 AND document_id = $3
                 ORDER BY ordinal ASC, id ASC",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
            )
            .context("list tenant PostgreSQL knowledge chunks")?;
        rows.iter().map(chunk_from_row).collect::<Result<Vec<_>>>().map(Some)
    }

    /// 在同一事务内创建文档并写入预先生成的全部分块。
    pub(in crate::knowledge::store) fn create_document_with_chunks(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
        body: KnowledgeDocumentCreateRequest,
        chunks: Vec<PreparedKnowledgeChunk>,
    ) -> Result<KnowledgeDocumentDto> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge document create")?;
        ensure_dataset_exists(&mut transaction, tenant, dataset_id)?;
        let document_id = Uuid::new_v4().to_string();
        let now = i64_from_u64(now_ms());
        let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
            .context("encode PostgreSQL knowledge document metadata")?;
        let document_hash = content_hash(&body.content);
        let char_count = i64::try_from(body.content.chars().count()).unwrap_or(i64::MAX);
        transaction
            .execute(
                "INSERT INTO knowledge_documents
                 (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled,
                  status, error_message, content_hash, char_count, recall_count, archived_at_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, '', $10, $11, 0, NULL, $12, $13)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id.as_str(),
                    &dataset_id,
                    &body.name.trim(),
                    &body.content,
                    &metadata_json,
                    &body.enabled,
                    &document_status_as_str(&KnowledgeDocumentStatus::Processing),
                    &document_hash,
                    &char_count,
                    &now,
                    &now,
                ],
            )
            .context("create tenant PostgreSQL knowledge document")?;
        replace_document_chunks(
            &mut transaction,
            tenant,
            dataset_id,
            &document_id,
            body.name.trim(),
            &body.metadata,
            body.enabled,
            chunks,
            now,
        )?;
        transaction
            .execute(
                "UPDATE knowledge_documents
                 SET status = $1, error_message = '', archived_at_ms = NULL, updated_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND id = $5",
                &[
                    &final_document_status(body.enabled),
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id.as_str(),
                ],
            )
            .context("complete tenant PostgreSQL knowledge document")?;
        transaction.commit().context("commit PostgreSQL knowledge document create")?;
        self.get_document(tenant, &document_id)?
            .context("PostgreSQL knowledge document was not saved")
    }

    /// 加载重新索引所需的文档正文与数据集配置。
    pub(in crate::knowledge::store) fn load_document_for_reindex(
        &self,
        tenant: &TenantScope,
        document_id: &str,
    ) -> Result<Option<ReindexKnowledgeDocument>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge document reindex load")?;
        let Some(document) = query_document_for_index(&mut transaction, tenant, document_id)?
        else {
            return Ok(None);
        };
        transaction
            .execute(
                "UPDATE knowledge_documents SET status = $1, error_message = '', updated_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND id = $5",
                &[
                    &document_status_as_str(&KnowledgeDocumentStatus::Processing),
                    &i64_from_u64(now_ms()),
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("mark tenant PostgreSQL knowledge document processing")?;
        transaction.commit().context("commit PostgreSQL knowledge document reindex load")?;
        let dataset = self
            .get_dataset(tenant, &document.dataset_id)?
            .with_context(|| format!("knowledge dataset not found: {}", document.dataset_id))?;
        Ok(Some(ReindexKnowledgeDocument { document, dataset }))
    }

    /// 原子替换文档分块，并将文档切换到最终可用状态。
    pub(in crate::knowledge::store) fn reindex_document(
        &self,
        tenant: &TenantScope,
        document_id: &str,
        chunks: Vec<PreparedKnowledgeChunk>,
    ) -> Result<Option<KnowledgeDocumentDto>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge document reindex")?;
        let Some(document) = query_document_for_index(&mut transaction, tenant, document_id)?
        else {
            return Ok(None);
        };
        let now = i64_from_u64(now_ms());
        replace_document_chunks(
            &mut transaction,
            tenant,
            &document.dataset_id,
            document_id,
            document.body.name.trim(),
            &document.body.metadata,
            document.body.enabled,
            chunks,
            now,
        )?;
        transaction
            .execute(
                "UPDATE knowledge_documents
                 SET status = $1, error_message = '', char_count = $2, archived_at_ms = NULL,
                     updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND id = $6",
                &[
                    &final_document_status(document.body.enabled),
                    &i64::try_from(document.body.content.chars().count()).unwrap_or(i64::MAX),
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("complete tenant PostgreSQL knowledge document reindex")?;
        transaction.commit().context("commit PostgreSQL knowledge document reindex")?;
        self.get_document(tenant, document_id)
    }

    /// 记录文档索引失败原因，供状态查询与后续重试展示。
    pub(in crate::knowledge::store) fn mark_document_failed(
        &self,
        tenant: &TenantScope,
        document_id: &str,
        error_message: &str,
    ) -> Result<Option<KnowledgeDocumentDto>> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "UPDATE knowledge_documents SET status = $1, error_message = $2, updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND id = $6",
                &[
                    &document_status_as_str(&KnowledgeDocumentStatus::Failed),
                    &error_message,
                    &i64_from_u64(now_ms()),
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("fail tenant PostgreSQL knowledge document")?;
        if changed == 0 {
            return Ok(None);
        }
        self.get_document(tenant, document_id)
    }
}

/// 删除旧分块并写入新分块，同时重建父子和前后链接。
pub(super) fn replace_document_chunks(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    dataset_id: &str,
    document_id: &str,
    document_name: &str,
    document_metadata: &Value,
    document_enabled: bool,
    chunks: Vec<PreparedKnowledgeChunk>,
    now: i64,
) -> Result<()> {
    connection
        .execute(
            "DELETE FROM knowledge_chunks_fts
             WHERE platform_id = $1 AND company_id = $2 AND document_id = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("delete tenant PostgreSQL knowledge full-text chunks")?;
    connection
        .execute(
            "DELETE FROM knowledge_chunks
             WHERE platform_id = $1 AND company_id = $2 AND document_id = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("delete tenant PostgreSQL knowledge chunks")?;

    let chunk_ids = (0..chunks.len()).map(|_| Uuid::new_v4().to_string()).collect::<Vec<_>>();
    let indexed_positions = chunks
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| chunk.indexed.then_some(index))
        .collect::<Vec<_>>();
    let parent_positions = chunks
        .iter()
        .enumerate()
        .filter_map(|(index, chunk)| {
            (chunk.chunk_type == KnowledgeChunkType::ParentText).then_some(index)
        })
        .collect::<Vec<_>>();

    for (index, chunk) in chunks.into_iter().enumerate() {
        let chunk_id = &chunk_ids[index];
        let parent_chunk_id =
            chunk.parent_position.and_then(|position| chunk_ids.get(position)).map(String::as_str);
        let prev_chunk_id = previous_link(&chunk_ids, &indexed_positions, index)
            .or_else(|| previous_link(&chunk_ids, &parent_positions, index));
        let next_chunk_id = next_link(&chunk_ids, &indexed_positions, index)
            .or_else(|| next_link(&chunk_ids, &parent_positions, index));
        let metadata =
            chunk_metadata(document_metadata, document_name, chunk.ordinal, &chunk.metadata);
        let metadata_json = serde_json::to_string(&metadata)
            .context("encode PostgreSQL knowledge chunk metadata")?;
        let index_content_hash = content_hash(&chunk.index_content);
        connection
            .execute(
                "INSERT INTO knowledge_chunks
                 (platform_id, company_id, id, dataset_id, document_id, ordinal, title, content,
                  metadata_json, chunk_type, start_at, end_at, parent_chunk_id, prev_chunk_id,
                  next_chunk_id, context_header, index_content_hash, embedding, enabled,
                  created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                         $15, $16, $17, $18, $19, $20, $21)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &chunk_id.as_str(),
                    &dataset_id,
                    &document_id,
                    &i64::try_from(chunk.ordinal).unwrap_or(i64::MAX),
                    &chunk.title,
                    &chunk.content,
                    &metadata_json,
                    &chunk_type_as_str(&chunk.chunk_type),
                    &i64::try_from(chunk.start_at).unwrap_or(i64::MAX),
                    &i64::try_from(chunk.end_at).unwrap_or(i64::MAX),
                    &parent_chunk_id,
                    &prev_chunk_id,
                    &next_chunk_id,
                    &chunk.context_header,
                    &index_content_hash,
                    &chunk.embedding,
                    &document_enabled,
                    &now,
                    &now,
                ],
            )
            .context("insert tenant PostgreSQL knowledge chunk")?;
        if chunk.indexed {
            connection
                .execute(
                    "INSERT INTO knowledge_chunks_fts
                     (platform_id, company_id, chunk_id, dataset_id, document_id, title, content)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &tenant.platform_id().as_ref(),
                        &tenant.company_id().as_ref(),
                        &chunk_id.as_str(),
                        &dataset_id,
                        &document_id,
                        &chunk.title,
                        &chunk.index_content,
                    ],
                )
                .context("insert tenant PostgreSQL knowledge full-text chunk")?;
        }
    }
    Ok(())
}

fn ensure_dataset_exists(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<()> {
    let exists: bool = connection
        .query_one(
            "SELECT EXISTS(
                 SELECT 1 FROM knowledge_datasets
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3
             )",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
        )
        .context("check tenant PostgreSQL knowledge dataset")?
        .get(0);
    if !exists {
        bail!("knowledge dataset not found: {dataset_id}");
    }
    Ok(())
}

fn query_document_for_index(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Option<StoredKnowledgeDocument>> {
    connection
        .query_opt(
            "SELECT id, dataset_id, name, content, metadata_json, enabled
             FROM knowledge_documents
             WHERE platform_id = $1 AND company_id = $2 AND id = $3
             FOR UPDATE",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("load tenant PostgreSQL knowledge document for indexing")?
        .map(|row| {
            let metadata_json: String =
                row.try_get(4).context("read knowledge document metadata")?;
            Ok(StoredKnowledgeDocument {
                id: row.try_get(0).context("read knowledge document id")?,
                dataset_id: row.try_get(1).context("read knowledge document dataset id")?,
                body: KnowledgeDocumentCreateRequest {
                    name: row.try_get(2).context("read knowledge document name")?,
                    content: row.try_get(3).context("read knowledge document content")?,
                    metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
                    enabled: row.try_get(5).context("read knowledge document enabled flag")?,
                },
            })
        })
        .transpose()
}

fn chunk_from_row(row: &Row) -> Result<KnowledgeChunkDto> {
    let metadata_json: String = row.try_get(5).context("read knowledge chunk metadata")?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or(Value::Null);
    let matched_content = super::super::matched_content_from_metadata(&metadata);
    Ok(KnowledgeChunkDto {
        id: row.try_get(0).context("read knowledge chunk id")?,
        dataset_id: row.try_get(1).context("read knowledge chunk dataset id")?,
        document_id: row.try_get(2).context("read knowledge chunk document id")?,
        title: row.try_get(3).context("read knowledge chunk title")?,
        content: row.try_get(4).context("read knowledge chunk content")?,
        metadata,
        score: None,
        ordinal: usize::try_from(row.try_get::<_, i64>(6).context("read knowledge chunk ordinal")?)
            .unwrap_or_default(),
        chunk_type: parse_chunk_type(
            &row.try_get::<_, String>(7).context("read knowledge chunk type")?,
        ),
        start_at: usize::try_from(row.try_get::<_, i64>(8).context("read knowledge chunk start")?)
            .unwrap_or_default(),
        end_at: usize::try_from(row.try_get::<_, i64>(9).context("read knowledge chunk end")?)
            .unwrap_or_default(),
        parent_chunk_id: row.try_get(10).context("read knowledge parent chunk id")?,
        prev_chunk_id: row.try_get(11).context("read knowledge previous chunk id")?,
        next_chunk_id: row.try_get(12).context("read knowledge next chunk id")?,
        matched_content,
        match_type: KnowledgeMatchType::FullText,
        context: None,
    })
}

fn metadata_or_object(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(Default::default()),
        other => other,
    }
}

#[cfg(test)]
#[path = "chunks_tests.rs"]
mod tests;
