//! 知识库网关数据契约；存储、索引与检索实现由运行时负责。

mod defaults;
mod dataset;
mod document;
mod retrieval;

pub use dataset::{
    KnowledgeIndexingMode,
    KnowledgeChunkingMode,
    KnowledgeDatasetDto,
    KnowledgeDatasetCreateRequest,
};
pub use document::{
    KnowledgeDocumentStatus,
    KnowledgeIndexJobStatus,
    KnowledgeDocumentDto,
    KnowledgeDocumentCreateRequest,
    KnowledgeDocumentListQuery,
    KnowledgeDocumentPatchRequest,
    KnowledgeDocumentsBatchAction,
    KnowledgeDocumentsBatchActionRequest,
    KnowledgeDocumentImportRequest,
    KnowledgeDocumentImportResponse,
    KnowledgeIndexJobDto,
    KnowledgeDocumentImportQueuedResponse,
    KnowledgeDocumentImportLimitsDto,
    KnowledgeDocumentImportTypeDto,
    KnowledgeImportSupportDto,
    KnowledgeDocumentReindexRequest,
};
pub use retrieval::{
    KnowledgeRetrievalMode,
    KnowledgeChunkType,
    KnowledgeMatchType,
    KnowledgeCapabilitiesDto,
    KnowledgeChunkDto,
    KnowledgeRetrieveRequest,
    KnowledgeRetrieveResponse,
    KnowledgeRuntimeStatus,
    KnowledgeEmbeddingStatusDto,
    KnowledgeEmbeddingPresetDto,
    KnowledgeEmbeddingTestRequest,
    KnowledgeEmbeddingTestResponse,
};

#[cfg(test)]
mod knowledge_tests;
