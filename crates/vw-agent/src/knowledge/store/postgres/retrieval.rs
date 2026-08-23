//! PostgreSQL 知识检索的作用域解析、候选召回与结果合并。

use anyhow::Context;
use postgres::{GenericClient, Row};
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_types::TenantScope;
use vw_api_types::knowledge::{
    KnowledgeChunkDto, KnowledgeChunkType, KnowledgeMatchType, KnowledgeRetrievalMode,
    KnowledgeRetrieveRequest, KnowledgeRetrieveResponse,
};

use super::super::{
    KeywordRetrievalDataset, MAX_CANDIDATES, MAX_TOP_K, RetrievalScope, database_error,
    keyword_limit, like_pattern, like_score, matched_content_from_metadata, merge_retrieval_scores,
    merge_text_chunks, metadata_matches, non_empty_context, now_ms, parse_chunk_type,
    parse_retrieval_mode, rerank_chunks, validate_id,
};
use super::PostgresKnowledgeStore;
use crate::app::agent::gateway::ApiError;
use crate::knowledge::fusion::{RetrievalSource, RetrievedCandidate};
use crate::memory::vector;

impl PostgresKnowledgeStore {
    /// 校验请求数据集并按各自检索模式构建全文、向量与重排作用域。
    pub(in crate::knowledge::store) fn resolve_retrieval_scope(
        &self,
        tenant: &TenantScope,
        dataset_ids: &[String],
    ) -> Result<RetrievalScope, ApiError> {
        for dataset_id in dataset_ids {
            validate_id(dataset_id, "dataset_id")?;
        }

        let mut connection = self.database.connect_postgres().map_err(database_error)?;
        let rows = connection
            .query(
                "SELECT id, retrieval_mode, rerank_enabled, keyword_count
                 FROM knowledge_datasets
                 WHERE platform_id = $1 AND company_id = $2
                 ORDER BY id ASC",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("resolve tenant PostgreSQL knowledge retrieval scope")
            .map_err(database_error)?;

        if !dataset_ids.is_empty() {
            for dataset_id in dataset_ids {
                if !rows.iter().any(|row| row.get::<_, String>(0) == *dataset_id) {
                    return Err(ApiError::not_found(format!(
                        "knowledge dataset not found: {dataset_id}"
                    )));
                }
            }
        }

        let mut keyword_datasets = Vec::new();
        let mut vector_dataset_ids = Vec::new();
        let mut rerank_dataset_ids = Vec::new();
        for row in rows {
            let dataset_id: String = row
                .try_get(0)
                .context("read PostgreSQL knowledge retrieval dataset id")
                .map_err(database_error)?;
            if !dataset_ids.is_empty() && !dataset_ids.contains(&dataset_id) {
                continue;
            }
            let retrieval_mode = parse_retrieval_mode(
                &row.try_get::<_, String>(1)
                    .context("read PostgreSQL knowledge retrieval mode")
                    .map_err(database_error)?,
            );
            let rerank_enabled: bool = row
                .try_get(2)
                .context("read PostgreSQL knowledge rerank flag")
                .map_err(database_error)?;
            let keyword_count = usize::try_from(
                row.try_get::<_, i64>(3)
                    .context("read PostgreSQL knowledge keyword count")
                    .map_err(database_error)?,
            )
            .unwrap_or_default();
            if rerank_enabled {
                rerank_dataset_ids.push(dataset_id.clone());
            }
            match retrieval_mode {
                KnowledgeRetrievalMode::FullText => {
                    keyword_datasets
                        .push(KeywordRetrievalDataset { id: dataset_id, keyword_count });
                }
                KnowledgeRetrievalMode::Vector => vector_dataset_ids.push(dataset_id),
                KnowledgeRetrievalMode::Hybrid => {
                    keyword_datasets
                        .push(KeywordRetrievalDataset { id: dataset_id.clone(), keyword_count });
                    vector_dataset_ids.push(dataset_id);
                }
            }
        }

        Ok(RetrievalScope { keyword_datasets, vector_dataset_ids, rerank_dataset_ids })
    }

    /// 执行 PostgreSQL 候选召回、分数融合、上下文扩展和召回计数更新。
    pub(in crate::knowledge::store) fn retrieve(
        &self,
        tenant: &TenantScope,
        request: KnowledgeRetrieveRequest,
        scope: RetrievalScope,
        query_embedding: Option<Vec<f32>>,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Result<KnowledgeRetrieveResponse, ApiError> {
        let mut connection = self.database.connect_postgres().map_err(database_error)?;
        let top_k = request.top_k.clamp(1, MAX_TOP_K);
        let candidate_limit = (top_k * 5).clamp(top_k, MAX_CANDIDATES);
        let mut keyword_results =
            retrieve_fts_candidates(&mut *connection, tenant, &request, &scope.keyword_datasets)?;
        if keyword_results.is_empty() {
            keyword_results = retrieve_like_candidates(
                &mut *connection,
                tenant,
                &request,
                &scope.keyword_datasets,
            )?;
        }
        let vector_results = query_embedding
            .as_ref()
            .map(|embedding| {
                retrieve_vector_candidates(
                    &mut *connection,
                    tenant,
                    &scope.vector_dataset_ids,
                    embedding,
                    candidate_limit,
                )
            })
            .transpose()?
            .unwrap_or_default();

        let merged = merge_retrieval_scores(
            &vector_results,
            &keyword_results,
            vector_weight,
            keyword_weight,
            candidate_limit,
        );
        let mut chunks = Vec::new();
        for scored in merged {
            let Some(chunk) = query_chunk(
                &mut *connection,
                tenant,
                &scored.id,
                Some(f64::from(scored.score)),
                scored.match_type,
            )?
            else {
                continue;
            };
            if !metadata_matches(&chunk.metadata, &request.metadata_filter) {
                continue;
            }
            if request
                .score_threshold
                .is_some_and(|threshold| chunk.score.unwrap_or_default() < threshold)
            {
                continue;
            }
            chunks.push(chunk);
            if chunks.len() >= candidate_limit {
                break;
            }
        }
        if !scope.rerank_dataset_ids.is_empty() {
            rerank_chunks(&request.query, &scope.rerank_dataset_ids, &mut chunks);
        }
        chunks.truncate(top_k);
        if request.include_context {
            enrich_chunk_contexts(&mut *connection, tenant, &mut chunks)?;
        }
        increment_recall_counts(&mut *connection, tenant, &chunks)?;

        Ok(KnowledgeRetrieveResponse { chunks })
    }
}

fn retrieve_fts_candidates(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    request: &KnowledgeRetrieveRequest,
    datasets: &[KeywordRetrievalDataset],
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    if datasets.is_empty() || !tsquery_has_terms(connection, &request.query)? {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();
    for dataset in datasets {
        let limit = i64::try_from(keyword_limit(dataset.keyword_count)).unwrap_or(i64::MAX);
        let rows = connection
            .query(
                "SELECT c.id,
                        ts_rank_cd(f.search_vector, websearch_to_tsquery('simple'::regconfig, $3), 32) AS rank
                 FROM knowledge_chunks_fts f
                 JOIN knowledge_chunks c
                   ON c.platform_id = f.platform_id AND c.company_id = f.company_id
                  AND c.id = f.chunk_id
                 JOIN knowledge_documents d
                   ON d.platform_id = c.platform_id AND d.company_id = c.company_id
                  AND d.id = c.document_id
                 WHERE f.platform_id = $1 AND f.company_id = $2
                   AND f.search_vector @@ websearch_to_tsquery('simple'::regconfig, $3)
                   AND c.enabled = TRUE AND d.enabled = TRUE AND d.status = 'completed'
                   AND c.dataset_id = $4
                 ORDER BY rank DESC, c.updated_at_ms DESC, c.id ASC
                 LIMIT $5",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &request.query,
                    &dataset.id,
                    &limit,
                ],
            )
            .context("retrieve tenant PostgreSQL knowledge full-text candidates")
            .map_err(database_error)?;
        for row in rows {
            candidates.push(RetrievedCandidate {
                id: row
                    .try_get(0)
                    .context("read PostgreSQL knowledge full-text chunk id")
                    .map_err(database_error)?,
                score: row
                    .try_get::<_, f32>(1)
                    .context("read PostgreSQL knowledge full-text rank")
                    .map_err(database_error)?,
                source: RetrievalSource::FullText,
            });
        }
    }
    Ok(candidates)
}

fn tsquery_has_terms(connection: &mut impl GenericClient, query: &str) -> Result<bool, ApiError> {
    connection
        .query_one("SELECT numnode(websearch_to_tsquery('simple'::regconfig, $1)) > 0", &[&query])
        .context("inspect parameterized PostgreSQL knowledge tsquery")
        .map(|row| row.get(0))
        .map_err(database_error)
}

fn retrieve_like_candidates(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    request: &KnowledgeRetrieveRequest,
    datasets: &[KeywordRetrievalDataset],
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    if datasets.is_empty() {
        return Ok(Vec::new());
    }

    let pattern = like_pattern(&request.query);
    let mut candidates = Vec::new();
    for dataset in datasets {
        let limit = i64::try_from(keyword_limit(dataset.keyword_count)).unwrap_or(i64::MAX);
        let rows = connection
            .query(
                "SELECT c.id, c.dataset_id, c.document_id, c.title, c.content, c.metadata_json,
                        c.ordinal, c.chunk_type, c.start_at, c.end_at, c.parent_chunk_id,
                        c.prev_chunk_id, c.next_chunk_id
                 FROM knowledge_chunks c
                 JOIN knowledge_documents d
                   ON d.platform_id = c.platform_id AND d.company_id = c.company_id
                  AND d.id = c.document_id
                 WHERE c.platform_id = $1 AND c.company_id = $2
                   AND c.enabled = TRUE AND d.enabled = TRUE AND d.status = 'completed'
                   AND c.chunk_type != 'parent_text' AND c.dataset_id = $3
                   AND (c.title ILIKE $4 ESCAPE E'\\\\' OR c.content ILIKE $4 ESCAPE E'\\\\')
                 ORDER BY c.updated_at_ms DESC, c.id ASC
                 LIMIT $5",
                &[
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &dataset.id,
                    &pattern,
                    &limit,
                ],
            )
            .context("retrieve tenant PostgreSQL knowledge ILIKE candidates")
            .map_err(database_error)?;
        for row in rows {
            let chunk = chunk_from_row(&row, None, KnowledgeMatchType::FullText)?;
            candidates.push(RetrievedCandidate {
                id: chunk.id.clone(),
                score: like_score(&chunk, &request.query),
                source: RetrievalSource::FullText,
            });
        }
    }
    Ok(candidates)
}

fn retrieve_vector_candidates(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    dataset_ids: &[String],
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<RetrievedCandidate>, ApiError> {
    let mut scored = Vec::new();
    for dataset_id in dataset_ids {
        let rows = connection
            .query(
                "SELECT c.id, c.embedding
                 FROM knowledge_chunks c
                 JOIN knowledge_documents d
                   ON d.platform_id = c.platform_id AND d.company_id = c.company_id
                  AND d.id = c.document_id
                 WHERE c.platform_id = $1 AND c.company_id = $2 AND c.dataset_id = $3
                   AND c.embedding IS NOT NULL AND c.enabled = TRUE AND d.enabled = TRUE
                   AND d.status = 'completed'",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &dataset_id],
            )
            .context("retrieve tenant PostgreSQL knowledge vector candidates")
            .map_err(database_error)?;
        for row in rows {
            let id: String = row
                .try_get(0)
                .context("read PostgreSQL knowledge vector chunk id")
                .map_err(database_error)?;
            let embedding = vector::bytes_to_vec(
                &row.try_get::<_, Vec<u8>>(1)
                    .context("read PostgreSQL knowledge vector embedding")
                    .map_err(database_error)?,
            );
            let score = vector::cosine_similarity(query_embedding, &embedding);
            if score > 0.0 {
                scored.push(RetrievedCandidate { id, score, source: RetrievalSource::Vector });
            }
        }
    }
    scored.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    scored.truncate(limit);
    Ok(scored)
}

fn query_chunk(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    chunk_id: &str,
    score: Option<f64>,
    match_type: KnowledgeMatchType,
) -> Result<Option<KnowledgeChunkDto>, ApiError> {
    connection
        .query_opt(
            "SELECT id, dataset_id, document_id, title, content, metadata_json, ordinal,
                    chunk_type, start_at, end_at, parent_chunk_id, prev_chunk_id, next_chunk_id
             FROM knowledge_chunks
             WHERE platform_id = $1 AND company_id = $2 AND id = $3 AND enabled = TRUE",
            &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref(), &chunk_id],
        )
        .context("load tenant PostgreSQL knowledge retrieval chunk")
        .map_err(database_error)?
        .as_ref()
        .map(|row| chunk_from_row(row, score, match_type))
        .transpose()
}

fn chunk_from_row(
    row: &Row,
    score: Option<f64>,
    match_type: KnowledgeMatchType,
) -> Result<KnowledgeChunkDto, ApiError> {
    let metadata_json: String = row
        .try_get(5)
        .context("read PostgreSQL knowledge chunk metadata")
        .map_err(database_error)?;
    let metadata = serde_json::from_str(&metadata_json).unwrap_or(Value::Null);
    let matched_content = matched_content_from_metadata(&metadata);
    Ok(KnowledgeChunkDto {
        id: row.try_get(0).context("read PostgreSQL knowledge chunk id").map_err(database_error)?,
        dataset_id: row
            .try_get(1)
            .context("read PostgreSQL knowledge chunk dataset id")
            .map_err(database_error)?,
        document_id: row
            .try_get(2)
            .context("read PostgreSQL knowledge chunk document id")
            .map_err(database_error)?,
        title: row
            .try_get(3)
            .context("read PostgreSQL knowledge chunk title")
            .map_err(database_error)?,
        content: row
            .try_get(4)
            .context("read PostgreSQL knowledge chunk content")
            .map_err(database_error)?,
        metadata,
        score,
        ordinal: read_usize(row, 6, "ordinal")?,
        chunk_type: parse_chunk_type(
            &row.try_get::<_, String>(7)
                .context("read PostgreSQL knowledge chunk type")
                .map_err(database_error)?,
        ),
        start_at: read_usize(row, 8, "start")?,
        end_at: read_usize(row, 9, "end")?,
        parent_chunk_id: row
            .try_get(10)
            .context("read PostgreSQL knowledge parent chunk id")
            .map_err(database_error)?,
        prev_chunk_id: row
            .try_get(11)
            .context("read PostgreSQL knowledge previous chunk id")
            .map_err(database_error)?,
        next_chunk_id: row
            .try_get(12)
            .context("read PostgreSQL knowledge next chunk id")
            .map_err(database_error)?,
        matched_content,
        match_type,
        context: None,
    })
}

fn read_usize(row: &Row, index: usize, field: &str) -> Result<usize, ApiError> {
    usize::try_from(
        row.try_get::<_, i64>(index)
            .with_context(|| format!("read PostgreSQL knowledge chunk {field}"))
            .map_err(database_error)?,
    )
    .map_err(|_| ApiError::internal(format!("invalid PostgreSQL knowledge chunk {field}")))
}

fn enrich_chunk_contexts(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    chunks: &mut [KnowledgeChunkDto],
) -> Result<(), ApiError> {
    for chunk in chunks {
        if chunk.chunk_type == KnowledgeChunkType::Qa {
            continue;
        }
        if let Some(parent_chunk_id) = chunk.parent_chunk_id.as_deref() {
            if let Some(parent) =
                query_chunk(connection, tenant, parent_chunk_id, None, KnowledgeMatchType::Parent)?
            {
                chunk.context = non_empty_context(parent.content);
                if chunk.matched_content.is_none() {
                    chunk.matched_content = Some(chunk.content.clone());
                }
            }
            continue;
        }
        if chunk.chunk_type != KnowledgeChunkType::Text {
            continue;
        }
        let mut linked_chunks = Vec::new();
        for chunk_id in [
            chunk.prev_chunk_id.as_deref(),
            Some(chunk.id.as_str()),
            chunk.next_chunk_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(candidate) =
                query_chunk(connection, tenant, chunk_id, None, KnowledgeMatchType::Nearby)?
                && candidate.document_id == chunk.document_id
                && candidate.chunk_type == KnowledgeChunkType::Text
            {
                linked_chunks.push(candidate);
            }
        }
        chunk.context = non_empty_context(merge_text_chunks(&linked_chunks, "\n"));
    }
    Ok(())
}

fn increment_recall_counts(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    chunks: &[KnowledgeChunkDto],
) -> Result<(), ApiError> {
    let mut counts = BTreeMap::<&str, u64>::new();
    for chunk in chunks {
        *counts.entry(chunk.document_id.as_str()).or_default() += 1;
    }
    for (document_id, count) in counts {
        let count = i64::try_from(count).unwrap_or(i64::MAX);
        let now = i64::try_from(now_ms()).unwrap_or(i64::MAX);
        connection
            .execute(
                "UPDATE knowledge_documents
                 SET recall_count = recall_count + $1, updated_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND id = $5",
                &[
                    &count,
                    &now,
                    &tenant.platform_id().as_ref(),
                    &tenant.company_id().as_ref(),
                    &document_id,
                ],
            )
            .context("increment tenant PostgreSQL knowledge recall count")
            .map_err(database_error)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "retrieval_tests.rs"]
mod tests;
