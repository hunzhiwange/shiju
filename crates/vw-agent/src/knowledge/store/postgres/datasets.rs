use anyhow::{Context, Result, bail};
use postgres::{GenericClient, Row};
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{KnowledgeDatasetCreateRequest, KnowledgeDatasetDto};

use super::super::{
    MAX_TOP_K, chunking_mode_as_str, dataset_capabilities, indexing_mode_as_str, now_ms,
    parse_chunking_mode, parse_indexing_mode, parse_retrieval_mode, retrieval_mode_as_str,
};
use super::PostgresKnowledgeStore;

const DATASET_SELECT: &str =
    "SELECT d.id, d.name, d.description, d.chunking_mode, d.indexing_mode, d.retrieval_mode,
            d.keyword_count, d.top_k, d.score_threshold_enabled, d.score_threshold,
            d.rerank_enabled, d.embedding_model, d.rerank_model, d.created_at_ms, d.updated_at_ms,
            (SELECT COUNT(*) FROM knowledge_documents documents
             WHERE documents.platform_id = d.platform_id AND documents.company_id = d.company_id
               AND documents.dataset_id = d.id) AS document_count,
            (SELECT COUNT(*) FROM knowledge_chunks chunks
             WHERE chunks.platform_id = d.platform_id AND chunks.company_id = d.company_id
               AND chunks.dataset_id = d.id) AS chunk_count
     FROM knowledge_datasets d";

impl PostgresKnowledgeStore {
    pub(in crate::knowledge::store) fn list_datasets(
        &self,
        tenant: &TenantScope,
    ) -> Result<Vec<KnowledgeDatasetDto>> {
        let mut connection = self.database.connect_postgres()?;
        connection
            .query(
                &format!(
                    "{DATASET_SELECT}
                     WHERE d.platform_id = $1 AND d.company_id = $2
                     ORDER BY d.updated_at_ms DESC, d.name ASC"
                ),
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("list tenant PostgreSQL knowledge datasets")?
            .iter()
            .map(dataset_from_row)
            .collect()
    }

    pub(in crate::knowledge::store) fn get_dataset(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
    ) -> Result<Option<KnowledgeDatasetDto>> {
        let mut connection = self.database.connect_postgres()?;
        query_dataset(&mut *connection, tenant, dataset_id)
    }

    pub(in crate::knowledge::store) fn create_dataset(
        &self,
        tenant: &TenantScope,
        body: KnowledgeDatasetCreateRequest,
    ) -> Result<KnowledgeDatasetDto> {
        let id = Uuid::new_v4().to_string();
        let now = i64_from_u64(now_ms());
        let keyword_count = i64::try_from(body.keyword_count).unwrap_or(i64::MAX);
        let top_k = i64::try_from(body.top_k.clamp(1, MAX_TOP_K)).unwrap_or(i64::MAX);
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "INSERT INTO knowledge_datasets
                 (platform_id, company_id, id, name, description, chunking_mode, indexing_mode,
                  retrieval_mode, keyword_count, top_k, score_threshold_enabled, score_threshold,
                  rerank_enabled, embedding_model, rerank_model, created_at_ms, updated_at_ms)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &id.as_str(),
                    &body.name.trim(),
                    &body.description.trim(),
                    &chunking_mode_as_str(&body.chunking_mode),
                    &indexing_mode_as_str(&body.indexing_mode),
                    &retrieval_mode_as_str(&body.retrieval_mode),
                    &keyword_count,
                    &top_k,
                    &body.score_threshold_enabled,
                    &body.score_threshold,
                    &body.rerank_enabled,
                    &body.embedding_model.as_deref(),
                    &body.rerank_model.as_deref(),
                    &now,
                    &now,
                ],
            )
            .context("create tenant PostgreSQL knowledge dataset")?;
        if changed != 1 {
            bail!("PostgreSQL knowledge dataset create affected an unexpected row count");
        }
        query_dataset(&mut *connection, tenant, &id)?
            .context("PostgreSQL knowledge dataset was not saved")
    }

    pub(in crate::knowledge::store) fn delete_dataset(
        &self,
        tenant: &TenantScope,
        dataset_id: &str,
    ) -> Result<Option<KnowledgeDatasetDto>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL knowledge dataset delete")?;
        let Some(dataset) = query_dataset(&mut transaction, tenant, dataset_id)? else {
            return Ok(None);
        };
        let changed = transaction
            .execute(
                "DELETE FROM knowledge_datasets
                 WHERE platform_id = $1 AND company_id = $2 AND id = $3",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
            )
            .context("delete tenant PostgreSQL knowledge dataset")?;
        if changed != 1 {
            bail!("PostgreSQL knowledge dataset delete affected an unexpected row count");
        }
        transaction.commit().context("commit PostgreSQL knowledge dataset delete")?;
        Ok(Some(dataset))
    }
}

fn query_dataset(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    dataset_id: &str,
) -> Result<Option<KnowledgeDatasetDto>> {
    connection
        .query_opt(
            &format!(
                "{DATASET_SELECT}
                 WHERE d.platform_id = $1 AND d.company_id = $2 AND d.id = $3"
            ),
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
        )
        .context("load tenant PostgreSQL knowledge dataset")?
        .as_ref()
        .map(dataset_from_row)
        .transpose()
}

fn dataset_from_row(row: &Row) -> Result<KnowledgeDatasetDto> {
    let indexing_mode_value: String = row.try_get(4).context("read knowledge indexing mode")?;
    let retrieval_mode_value: String = row.try_get(5).context("read knowledge retrieval mode")?;
    let indexing_mode = parse_indexing_mode(&indexing_mode_value);
    let retrieval_mode = parse_retrieval_mode(&retrieval_mode_value);
    let rerank_enabled: bool = row.try_get(10).context("read knowledge rerank flag")?;
    Ok(KnowledgeDatasetDto {
        id: row.try_get(0).context("read knowledge dataset id")?,
        name: row.try_get(1).context("read knowledge dataset name")?,
        description: row.try_get(2).context("read knowledge dataset description")?,
        capabilities: dataset_capabilities(&indexing_mode, &retrieval_mode, rerank_enabled),
        chunking_mode: parse_chunking_mode(
            &row.try_get::<_, String>(3).context("read knowledge chunking mode")?,
        ),
        indexing_mode,
        retrieval_mode,
        keyword_count: usize_from_i64(row.try_get(6).context("read knowledge keyword count")?),
        top_k: usize_from_i64(row.try_get(7).context("read knowledge top k")?),
        score_threshold_enabled: row.try_get(8).context("read knowledge score threshold flag")?,
        score_threshold: row.try_get(9).context("read knowledge score threshold")?,
        rerank_enabled,
        embedding_model: row.try_get(11).context("read knowledge embedding model")?,
        rerank_model: row.try_get(12).context("read knowledge rerank model")?,
        created_at_ms: u64_from_i64(row.try_get(13).context("read knowledge creation time")?),
        updated_at_ms: u64_from_i64(row.try_get(14).context("read knowledge update time")?),
        document_count: u64_from_i64(row.try_get(15).context("read knowledge document count")?),
        chunk_count: u64_from_i64(row.try_get(16).context("read knowledge chunk count")?),
    })
}

pub(super) fn i64_from_u64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn u64_from_i64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn usize_from_i64(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}

#[cfg(test)]
#[path = "datasets_tests.rs"]
mod tests;
