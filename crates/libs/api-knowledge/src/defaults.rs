//! 兼容旧载荷的内部默认值。

use crate::{
    KnowledgeCapabilitiesDto, KnowledgeChunkType, KnowledgeChunkingMode, KnowledgeDocumentStatus,
    KnowledgeIndexingMode, KnowledgeMatchType, KnowledgeRetrievalMode,
};

pub(crate) fn default_indexing_mode() -> KnowledgeIndexingMode {
    KnowledgeIndexingMode::Economy
}

pub(crate) fn default_retrieval_mode() -> KnowledgeRetrievalMode {
    KnowledgeRetrievalMode::FullText
}

pub(crate) fn default_chunking_mode() -> KnowledgeChunkingMode {
    KnowledgeChunkingMode::General
}

pub(crate) fn default_top_k() -> usize {
    10
}

pub(crate) fn default_keyword_count() -> usize {
    10
}

pub(crate) fn default_score_threshold() -> f64 {
    0.15
}

pub(crate) fn default_enabled() -> bool {
    true
}

pub(crate) fn default_capabilities() -> KnowledgeCapabilitiesDto {
    KnowledgeCapabilitiesDto::default()
}

pub(crate) fn default_document_status() -> KnowledgeDocumentStatus {
    KnowledgeDocumentStatus::default()
}

pub(crate) fn default_chunk_type() -> KnowledgeChunkType {
    KnowledgeChunkType::default()
}

pub(crate) fn default_match_type() -> KnowledgeMatchType {
    KnowledgeMatchType::default()
}

pub(crate) fn is_false(value: &bool) -> bool {
    !*value
}
