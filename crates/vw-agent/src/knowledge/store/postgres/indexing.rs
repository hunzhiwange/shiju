//! PostgreSQL 文档导入与索引任务的入队、租约和完成流程。

use anyhow::{Context, Result, anyhow, bail};
use postgres::{GenericClient, Row};
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeDocumentCreateRequest, KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportRequest, KnowledgeDocumentStatus, KnowledgeIndexJobDto,
    KnowledgeIndexJobStatus,
};

use super::super::{
    ClaimedIndexJob, INDEX_JOB_LEASE_TIMEOUT_MS, INDEX_JOB_MAX_ATTEMPTS, PreparedKnowledgeChunk,
    clean_import_filename, content_hash, document_status_as_str, final_document_status,
    import_source_bytes, index_job_retry_delay_ms, index_job_status_as_str,
    index_job_timed_out_message, metadata_or_object, normalize_import_mime, now_ms,
    parse_index_job_status,
};
use super::PostgresKnowledgeStore;
use super::chunks::replace_document_chunks;
use super::datasets::{i64_from_u64, u64_from_i64};

impl PostgresKnowledgeStore {
    pub(in crate::knowledge::store) fn enqueue_import(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
        body: KnowledgeDocumentImportRequest,
    ) -> Result<KnowledgeDocumentImportQueuedResponse> {
        let filename = clean_import_filename(&body.filename).map_err(|error| anyhow!(error))?;
        let mime_type = normalize_import_mime(body.mime_type.as_deref())
            .or_else(|| mime_guess::from_path(&filename).first_raw().map(str::to_string));
        let source_bytes =
            import_source_bytes(&body.content_base64).map_err(|error| anyhow!(error))?;
        let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
            .context("encode PostgreSQL knowledge import metadata")?;
        let payload_json =
            serde_json::to_string(&body).context("encode PostgreSQL knowledge import payload")?;
        let document_id = Uuid::new_v4().to_string();
        let job_id = Uuid::new_v4().to_string();
        let now = i64_from_u64(now_ms());
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge import enqueue")?;
        ensure_dataset_exists(&mut transaction, tenant, dataset_id)?;
        transaction
            .execute(
                "INSERT INTO knowledge_documents
                 (platform_id, company_id, id, dataset_id, name, content, metadata_json, enabled,
                  status, error_message, content_hash, char_count, recall_count, archived_at_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, '', $6, $7, $8, '', '', 0, 0, NULL, $9, $10)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id.as_str(),
                    &dataset_id,
                    &filename,
                    &metadata_json,
                    &body.enabled,
                    &document_status_as_str(&KnowledgeDocumentStatus::Pending),
                    &now,
                    &now,
                ],
            )
            .context("create tenant PostgreSQL pending knowledge document")?;
        transaction
            .execute(
                "INSERT INTO knowledge_index_jobs
                 (platform_id, company_id, id, dataset_id, document_id, status, error_message,
                  payload_kind, payload_json, created_at_ms, updated_at_ms, started_at_ms,
                  finished_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, '', 'import', $7, $8, $9, NULL, NULL)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &job_id.as_str(),
                    &dataset_id,
                    &document_id.as_str(),
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                    &payload_json,
                    &now,
                    &now,
                ],
            )
            .context("create tenant PostgreSQL knowledge index job")?;
        transaction.commit().context("commit PostgreSQL knowledge import enqueue")?;
        let document = self
            .get_document(tenant, &document_id)?
            .context("queued PostgreSQL knowledge document was not saved")?;
        let job = self
            .get_index_job(tenant, &job_id)?
            .context("PostgreSQL knowledge index job was not saved")?;
        Ok(KnowledgeDocumentImportQueuedResponse {
            document,
            job,
            filename,
            mime_type,
            source_bytes,
            warnings: Vec::new(),
        })
    }

    pub(in crate::knowledge::store) fn get_index_job(
        &self,
        tenant: &TenantScope,
        job_id: &str,
    ) -> Result<Option<KnowledgeIndexJobDto>> {
        let mut connection = self.database.connect_postgres()?;
        query_index_job(&mut *connection, tenant, job_id)
    }

    pub(in crate::knowledge::store) fn claim_next_index_job(
        &self,
        tenant: &TenantScope,
    ) -> Result<Option<ClaimedIndexJob>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge index job claim")?;
        let now = i64_from_u64(now_ms());
        let stale_started_before = now.saturating_sub(i64_from_u64(INDEX_JOB_LEASE_TIMEOUT_MS));
        let lease_expires_at_ms = now.saturating_add(i64_from_u64(INDEX_JOB_LEASE_TIMEOUT_MS));
        fail_expired_index_jobs(&mut transaction, tenant, now, stale_started_before)?;
        let claimed = transaction
            .query_opt(
                "SELECT id, dataset_id, document_id, payload_kind, payload_json, attempt_count + 1
                 FROM knowledge_index_jobs
                 WHERE platform_id = $1 AND company_id = $2 AND (
                    (status = $3 AND retry_after_ms <= $4)
                    OR (status = $5
                        AND ((lease_expires_at_ms > 0 AND lease_expires_at_ms <= $4)
                             OR (lease_expires_at_ms = 0 AND started_at_ms IS NOT NULL
                                 AND started_at_ms <= $6))
                        AND attempt_count < $7))
                 ORDER BY CASE WHEN status = $5 THEN 0 ELSE 1 END, retry_after_ms ASC,
                          created_at_ms ASC, id ASC
                 LIMIT 1 FOR UPDATE SKIP LOCKED",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                    &now,
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                    &stale_started_before,
                    &INDEX_JOB_MAX_ATTEMPTS,
                ],
            )
            .context("lock next tenant PostgreSQL knowledge index job")?
            .map(|row| ClaimedIndexJob {
                id: row.get(0),
                dataset_id: row.get(1),
                document_id: row.get(2),
                payload_kind: row.get(3),
                payload_json: row.get(4),
                attempt_count: row.get(5),
            });
        let Some(claimed) = claimed else {
            transaction.commit().context("commit empty PostgreSQL knowledge index job claim")?;
            return Ok(None);
        };
        let changed = transaction
            .execute(
                "UPDATE knowledge_index_jobs
                 SET status = $1, error_message = '', updated_at_ms = $2, started_at_ms = $3,
                     finished_at_ms = NULL, attempt_count = attempt_count + 1,
                     retry_after_ms = 0, lease_expires_at_ms = $4
                 WHERE platform_id = $5 AND company_id = $6 AND id = $7
                   AND status IN ($8, $9)",
                &[
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                    &now,
                    &now,
                    &lease_expires_at_ms,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &claimed.id,
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                ],
            )
            .context("claim tenant PostgreSQL knowledge index job")?;
        if changed != 1 {
            bail!("PostgreSQL knowledge index job claim affected an unexpected row count");
        }
        transaction
            .execute(
                "UPDATE knowledge_documents SET status = $1, error_message = '', updated_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND id = $5",
                &[
                    &document_status_as_str(&KnowledgeDocumentStatus::Processing),
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &claimed.document_id,
                ],
            )
            .context("mark claimed tenant PostgreSQL knowledge document processing")?;
        transaction.commit().context("commit PostgreSQL knowledge index job claim")?;
        Ok(Some(claimed))
    }

    pub(in crate::knowledge::store) fn complete_index_job(
        &self,
        tenant: &TenantScope,
        job: ClaimedIndexJob,
        body: KnowledgeDocumentCreateRequest,
        chunks: Vec<PreparedKnowledgeChunk>,
    ) -> Result<KnowledgeIndexJobDto> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge index job completion")?;
        let now = i64_from_u64(now_ms());
        replace_document_chunks(
            &mut transaction,
            tenant,
            &job.dataset_id,
            &job.document_id,
            body.name.trim(),
            &body.metadata,
            body.enabled,
            chunks,
            now,
        )?;
        let metadata_json = serde_json::to_string(&metadata_or_object(body.metadata.clone()))
            .context("encode PostgreSQL indexed document metadata")?;
        let document_hash = content_hash(&body.content);
        let char_count = i64::try_from(body.content.chars().count()).unwrap_or(i64::MAX);
        let document_changed = transaction
            .execute(
                "UPDATE knowledge_documents
                 SET name = $1, content = $2, metadata_json = $3, enabled = $4, status = $5,
                     error_message = '', content_hash = $6, char_count = $7, archived_at_ms = NULL,
                     updated_at_ms = $8
                 WHERE platform_id = $9 AND company_id = $10 AND id = $11",
                &[
                    &body.name.trim(),
                    &body.content,
                    &metadata_json,
                    &body.enabled,
                    &final_document_status(body.enabled),
                    &document_hash,
                    &char_count,
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &job.document_id,
                ],
            )
            .context("complete tenant PostgreSQL indexed document")?;
        let job_changed = transaction
            .execute(
                "UPDATE knowledge_index_jobs
                 SET status = $1, error_message = '', payload_json = '', updated_at_ms = $2,
                     retry_after_ms = 0, lease_expires_at_ms = 0, finished_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND id = $6",
                &[
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Completed),
                    &now,
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &job.id,
                ],
            )
            .context("complete tenant PostgreSQL knowledge index job")?;
        if document_changed != 1 || job_changed != 1 {
            bail!("PostgreSQL knowledge index job completion affected an unexpected row count");
        }
        transaction.commit().context("commit PostgreSQL knowledge index job completion")?;
        self.get_index_job(tenant, &job.id)?
            .context("completed PostgreSQL knowledge index job was not found")
    }

    pub(in crate::knowledge::store) fn retry_or_fail_index_job(
        &self,
        tenant: &TenantScope,
        job_id: &str,
        document_id: &str,
        attempt_count: i64,
        error_message: &str,
    ) -> Result<KnowledgeIndexJobDto> {
        if attempt_count >= INDEX_JOB_MAX_ATTEMPTS {
            return self.fail_index_job(tenant, job_id, document_id, error_message);
        }
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge index job retry")?;
        let now = i64_from_u64(now_ms());
        let retry_after_ms =
            now.saturating_add(i64_from_u64(index_job_retry_delay_ms(attempt_count)));
        let retry_message = format!(
            "attempt {attempt_count}/{INDEX_JOB_MAX_ATTEMPTS} failed: {error_message}; retrying"
        );
        transaction
            .execute(
                "UPDATE knowledge_documents SET status = $1, error_message = $2, updated_at_ms = $3
                 WHERE platform_id = $4 AND company_id = $5 AND id = $6",
                &[
                    &document_status_as_str(&KnowledgeDocumentStatus::Pending),
                    &retry_message,
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("retry tenant PostgreSQL knowledge document")?;
        transaction
            .execute(
                "UPDATE knowledge_index_jobs
                 SET status = $1, error_message = $2, updated_at_ms = $3, retry_after_ms = $4,
                     lease_expires_at_ms = 0, finished_at_ms = NULL
                 WHERE platform_id = $5 AND company_id = $6 AND id = $7",
                &[
                    &index_job_status_as_str(&KnowledgeIndexJobStatus::Queued),
                    &retry_message,
                    &now,
                    &retry_after_ms,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &job_id,
                ],
            )
            .context("retry tenant PostgreSQL knowledge index job")?;
        transaction.commit().context("commit PostgreSQL knowledge index job retry")?;
        self.get_index_job(tenant, job_id)?
            .context("retried PostgreSQL knowledge index job was not found")
    }

    fn fail_index_job(
        &self,
        tenant: &TenantScope,
        job_id: &str,
        document_id: &str,
        error_message: &str,
    ) -> Result<KnowledgeIndexJobDto> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge index job failure")?;
        mark_index_job_failed(
            &mut transaction,
            tenant,
            job_id,
            document_id,
            error_message,
            i64_from_u64(now_ms()),
        )?;
        transaction.commit().context("commit PostgreSQL knowledge index job failure")?;
        self.get_index_job(tenant, job_id)?
            .context("failed PostgreSQL knowledge index job was not found")
    }
}

fn fail_expired_index_jobs(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    now: i64,
    stale_started_before: i64,
) -> Result<()> {
    let rows = connection
        .query(
            "SELECT id, document_id FROM knowledge_index_jobs
             WHERE platform_id = $1 AND company_id = $2 AND status = $3
               AND ((lease_expires_at_ms > 0 AND lease_expires_at_ms <= $4)
                    OR (lease_expires_at_ms = 0 AND started_at_ms IS NOT NULL
                        AND started_at_ms <= $5))
               AND attempt_count >= $6
             FOR UPDATE SKIP LOCKED",
            &[
                &tenant.platform_id().as_ref(),
                &tenant.company_id().as_ref(),
                &index_job_status_as_str(&KnowledgeIndexJobStatus::Processing),
                &now,
                &stale_started_before,
                &INDEX_JOB_MAX_ATTEMPTS,
            ],
        )
        .context("lock expired tenant PostgreSQL knowledge index jobs")?;
    for row in rows {
        let job_id: String = row.get(0);
        let document_id: String = row.get(1);
        mark_index_job_failed(
            connection,
            tenant,
            &job_id,
            &document_id,
            &index_job_timed_out_message(),
            now,
        )?;
    }
    Ok(())
}

fn mark_index_job_failed(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    job_id: &str,
    document_id: &str,
    error_message: &str,
    now: i64,
) -> Result<()> {
    connection
        .execute(
            "DELETE FROM knowledge_chunks_fts
             WHERE platform_id = $1 AND company_id = $2 AND document_id = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("delete failed tenant PostgreSQL knowledge full-text chunks")?;
    connection
        .execute(
            "DELETE FROM knowledge_chunks
             WHERE platform_id = $1 AND company_id = $2 AND document_id = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &document_id],
        )
        .context("delete failed tenant PostgreSQL knowledge chunks")?;
    connection
        .execute(
            "UPDATE knowledge_documents SET status = $1, error_message = $2, updated_at_ms = $3
             WHERE platform_id = $4 AND company_id = $5 AND id = $6",
            &[
                &document_status_as_str(&KnowledgeDocumentStatus::Failed),
                &error_message,
                &now,
                &tenant.platform_id().as_ref(),
                &tenant.company_id().as_ref(),
                &document_id,
            ],
        )
        .context("fail tenant PostgreSQL knowledge document")?;
    connection
        .execute(
            "UPDATE knowledge_index_jobs
             SET status = $1, error_message = $2, payload_json = '', updated_at_ms = $3,
                 retry_after_ms = 0, lease_expires_at_ms = 0, finished_at_ms = $4
             WHERE platform_id = $5 AND company_id = $6 AND id = $7",
            &[
                &index_job_status_as_str(&KnowledgeIndexJobStatus::Failed),
                &error_message,
                &now,
                &now,
                &tenant.platform_id().as_ref(),
                &tenant.company_id().as_ref(),
                &job_id,
            ],
        )
        .context("fail tenant PostgreSQL knowledge index job")?;
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

fn query_index_job(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    job_id: &str,
) -> Result<Option<KnowledgeIndexJobDto>> {
    connection
        .query_opt(
            "SELECT id, dataset_id, document_id, status, error_message, created_at_ms,
                    updated_at_ms, started_at_ms, finished_at_ms
             FROM knowledge_index_jobs
             WHERE platform_id = $1 AND company_id = $2 AND id = $3",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &job_id],
        )
        .context("load tenant PostgreSQL knowledge index job")?
        .as_ref()
        .map(index_job_from_row)
        .transpose()
}

fn index_job_from_row(row: &Row) -> Result<KnowledgeIndexJobDto> {
    let status: String = row.try_get(3).context("read knowledge index job status")?;
    Ok(KnowledgeIndexJobDto {
        id: row.try_get(0).context("read knowledge index job id")?,
        dataset_id: row.try_get(1).context("read knowledge index job dataset id")?,
        document_id: row.try_get(2).context("read knowledge index job document id")?,
        status: parse_index_job_status(&status),
        error_message: row.try_get(4).context("read knowledge index job error")?,
        created_at_ms: u64_from_i64(row.try_get(5).context("read knowledge job creation time")?),
        updated_at_ms: u64_from_i64(row.try_get(6).context("read knowledge job update time")?),
        started_at_ms: row
            .try_get::<_, Option<i64>>(7)
            .context("read knowledge job start time")?
            .map(u64_from_i64),
        finished_at_ms: row
            .try_get::<_, Option<i64>>(8)
            .context("read knowledge job finish time")?
            .map(u64_from_i64),
    })
}

#[cfg(test)]
#[path = "indexing_tests.rs"]
mod tests;
