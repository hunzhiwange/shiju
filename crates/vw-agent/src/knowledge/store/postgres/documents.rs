//! PostgreSQL 知识文档的持久化、筛选与状态映射。

use anyhow::{Context, Result, bail};
use postgres::{GenericClient, Row};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeDocumentCreateRequest, KnowledgeDocumentDto, KnowledgeDocumentListQuery,
    KnowledgeDocumentStatus,
};

use super::super::{document_status_as_str, now_ms, parse_document_status};
use super::PostgresKnowledgeStore;
use super::datasets::{i64_from_u64, u64_from_i64};

const DOCUMENT_SELECT: &str =
    "SELECT d.id, d.dataset_id, d.name, d.metadata_json, d.enabled, d.created_at_ms,
            d.updated_at_ms, d.status, d.error_message, d.content_hash, d.char_count,
            d.recall_count, d.archived_at_ms,
            (SELECT COUNT(*) FROM knowledge_chunks chunks
             WHERE chunks.platform_id = d.platform_id AND chunks.company_id = d.company_id
               AND chunks.document_id = d.id) AS chunk_count
     FROM knowledge_documents d";

impl PostgresKnowledgeStore {
    /// 按数据集和查询条件列出当前租户文档。
    pub(in crate::knowledge::store) fn list_documents(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
        query: &KnowledgeDocumentListQuery,
    ) -> Result<Vec<KnowledgeDocumentDto>> {
        let mut connection = self.database.connect_postgres()?;
        ensure_dataset_exists(&mut *connection, tenant, dataset_id)?;
        let rows = if let Some(status) = query.status.as_ref() {
            connection
                .query(
                    &format!(
                        "{DOCUMENT_SELECT}
                         WHERE d.platform_id = $1 AND d.company_id = $2 AND d.dataset_id = $3
                           AND d.status = $4
                         ORDER BY d.updated_at_ms DESC, d.name ASC"
                    ),
                    &[
                        &tenant.platform_id().as_ref(),
                        &tenant.company_id().as_ref(),
                        &dataset_id,
                        &document_status_as_str(status),
                    ],
                )
                .context("filter tenant PostgreSQL knowledge documents")?
        } else {
            connection
                .query(
                    &format!(
                        "{DOCUMENT_SELECT}
                         WHERE d.platform_id = $1 AND d.company_id = $2 AND d.dataset_id = $3
                         ORDER BY d.updated_at_ms DESC, d.name ASC"
                    ),
                    &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
                )
                .context("list tenant PostgreSQL knowledge documents")?
        };
        rows.iter().map(document_from_row).collect()
    }

    /// 按标识读取单个租户文档。
    pub(in crate::knowledge::store) fn get_document(
        &self,
        tenant: &TenantScope,
        document_id: &str,
    ) -> Result<Option<KnowledgeDocumentDto>> {
        let mut connection = self.database.connect_postgres()?;
        query_document(&mut *connection, tenant, document_id)
    }

    /// 创建尚未分块的文档记录，并初始化内容摘要与状态。
    pub(in crate::knowledge::store) fn create_document(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
        body: KnowledgeDocumentCreateRequest,
    ) -> Result<KnowledgeDocumentDto> {
        let mut connection = self.database.connect_postgres()?;
        ensure_dataset_exists(&mut *connection, tenant, dataset_id)?;
        let document_id = Uuid::new_v4().to_string();
        let now = i64_from_u64(now_ms());
        let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata))
            .context("encode PostgreSQL knowledge document metadata")?;
        let content_hash = content_hash(&body.content);
        let char_count = i64::try_from(body.content.chars().count()).unwrap_or(i64::MAX);
        let status = if body.enabled {
            KnowledgeDocumentStatus::Completed
        } else {
            KnowledgeDocumentStatus::Disabled
        };
        let changed = connection
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
                    &document_status_as_str(&status),
                    &content_hash,
                    &char_count,
                    &now,
                    &now,
                ],
            )
            .context("create tenant PostgreSQL knowledge document")?;
        if changed != 1 {
            bail!("PostgreSQL knowledge document create affected an unexpected row count");
        }
        query_document(&mut *connection, tenant, &document_id)?
            .context("PostgreSQL knowledge document was not saved")
    }

    /// 在允许的状态转换范围内更新文档属性。
    pub(in crate::knowledge::store) fn update_document(
        &self,
        tenant: &TenantScope,
        document_id: &str,
        name: Option<&str>,
        enabled: bool,
        status: &KnowledgeDocumentStatus,
        archived_at_ms: Option<u64>,
        updated_at_ms: u64,
    ) -> Result<Option<KnowledgeDocumentDto>> {
        let archived_at_ms = archived_at_ms.map(i64_from_u64);
        let updated_at_ms = i64_from_u64(updated_at_ms);
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "UPDATE knowledge_documents
                 SET name = COALESCE($1, name), enabled = $2, status = $3, archived_at_ms = $4,
                     error_message = '', updated_at_ms = $5
                 WHERE platform_id = $6 AND company_id = $7 AND id = $8",
                &[
                    &name,
                    &enabled,
                    &document_status_as_str(status),
                    &archived_at_ms,
                    &updated_at_ms,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("update tenant PostgreSQL knowledge document")?;
        if changed == 0 {
            return Ok(None);
        }
        if changed != 1 {
            bail!("PostgreSQL knowledge document update affected an unexpected row count");
        }
        query_document(&mut *connection, tenant, document_id)
    }

    /// 删除文档并返回删除前的投影。
    pub(in crate::knowledge::store) fn delete_document(
        &self,
        tenant: &TenantScope,
        document_id: &str,
    ) -> Result<Option<KnowledgeDocumentDto>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge document delete")?;
        let Some(document) = query_document(&mut transaction, tenant, document_id)? else {
            return Ok(None);
        };
        let changed = transaction
            .execute(
                "DELETE FROM knowledge_documents
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
            )
            .context("delete tenant PostgreSQL knowledge document")?;
        if changed != 1 {
            bail!("PostgreSQL knowledge document delete affected an unexpected row count");
        }
        transaction.commit().context("commit PostgreSQL knowledge document delete")?;
        Ok(Some(document))
    }
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

fn query_document(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    document_id: &str,
) -> Result<Option<KnowledgeDocumentDto>> {
    connection
        .query_opt(
            &format!(
                "{DOCUMENT_SELECT}
                 WHERE d.platform_id = $1 AND d.company_id = $2 AND d.id = $3"
            ),
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("load tenant PostgreSQL knowledge document")?
        .as_ref()
        .map(document_from_row)
        .transpose()
}

fn document_from_row(row: &Row) -> Result<KnowledgeDocumentDto> {
    let metadata_json: String = row.try_get(3).context("read knowledge document metadata")?;
    let enabled: bool = row.try_get(4).context("read knowledge document enabled flag")?;
    let status_value: String = row.try_get(7).context("read knowledge document status")?;
    Ok(KnowledgeDocumentDto {
        id: row.try_get(0).context("read knowledge document id")?,
        dataset_id: row.try_get(1).context("read knowledge document dataset id")?,
        name: row.try_get(2).context("read knowledge document name")?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or(Value::Null),
        enabled,
        status: parse_document_status(&status_value, enabled),
        error_message: row.try_get(8).context("read knowledge document error")?,
        content_hash: row.try_get(9).context("read knowledge document content hash")?,
        char_count: u64_from_i64(row.try_get(10).context("read knowledge character count")?),
        recall_count: u64_from_i64(row.try_get(11).context("read knowledge recall count")?),
        archived_at_ms: row
            .try_get::<_, Option<i64>>(12)
            .context("read knowledge archive time")?
            .map(u64_from_i64),
        created_at_ms: u64_from_i64(row.try_get(5).context("read knowledge creation time")?),
        updated_at_ms: u64_from_i64(row.try_get(6).context("read knowledge update time")?),
        chunk_count: u64_from_i64(row.try_get(13).context("read knowledge chunk count")?),
    })
}

fn metadata_or_object(value: Value) -> Value {
    match value {
        Value::Null => Value::Object(Default::default()),
        other => other,
    }
}

fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
