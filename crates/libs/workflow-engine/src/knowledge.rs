//! 工作流知识检索适配；存储由知识库领域提供。
use agent_knowledge::store::SqliteKnowledgeStore;
use crate::{WorkflowKnowledgeChunk, WorkflowKnowledgeProvider, WorkflowKnowledgeRequest};
use vw_api_knowledge::KnowledgeRetrieveRequest;

#[async_trait::async_trait]
impl WorkflowKnowledgeProvider for SqliteKnowledgeStore {
    async fn retrieve(
        &self,
        request: WorkflowKnowledgeRequest,
    ) -> Result<Vec<WorkflowKnowledgeChunk>, String> {
        let response = SqliteKnowledgeStore::retrieve(
            self,
            KnowledgeRetrieveRequest {
                query: request.query,
                dataset_ids: request.dataset_ids,
                top_k: request.top_k,
                score_threshold: request.score_threshold,
                metadata_filter: request.metadata_filter,
                include_context: false,
            },
        )
        .await
        .map_err(|error| error.to_string())?;

        Ok(response
            .chunks
            .into_iter()
            .map(|chunk| WorkflowKnowledgeChunk {
                content: chunk.content,
                title: chunk.title,
                metadata: chunk.metadata,
                score: chunk.score,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;
