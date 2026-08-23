//! 知识检索结果的排序融合辅助逻辑。
//!
//! 将全文与向量检索候选按来源去重，并用确定性的分数规则合并排序。

use std::cmp::Ordering;
use std::collections::BTreeMap;
use vw_api_types::knowledge::KnowledgeMatchType;

const DEFAULT_RRF_K: usize = 60;
const DEFAULT_VECTOR_WEIGHT: f32 = 0.7;
const DEFAULT_KEYWORD_WEIGHT: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetrievalSource {
    FullText,
    Vector,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RetrievedCandidate {
    pub(crate) id: String,
    pub(crate) score: f32,
    pub(crate) source: RetrievalSource,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FusedCandidate {
    pub(crate) id: String,
    /// Finite score. Single-source scores are clamped to [0, 1]; RRF scores are
    /// in [0, (vector_weight + keyword_weight) / (rrf_k + 1)].
    pub(crate) score: f32,
    pub(crate) match_type: KnowledgeMatchType,
}

#[derive(Debug, Clone)]
struct CandidateAccumulator {
    score: f32,
    has_full_text: bool,
    has_vector: bool,
}

#[derive(Debug, Clone, Default)]
struct RrfAccumulator {
    vector_rank: Option<usize>,
    keyword_rank: Option<usize>,
}

/// 对单一来源的候选按标识去重，保留每个标识的最高有效分数。
///
/// 非有限分数会被归零，输出顺序由分数、标识共同确定，避免数据库返回顺序影响结果。
pub(crate) fn deduplicate_by_score(
    results: &[RetrievedCandidate],
    limit: usize,
) -> Vec<FusedCandidate> {
    if limit == 0 {
        return Vec::new();
    }

    let mut by_id = BTreeMap::<String, CandidateAccumulator>::new();
    for result in results {
        let score = sanitize_score(result.score);
        by_id
            .entry(result.id.clone())
            .and_modify(|entry| {
                entry.score = entry.score.max(score);
                match result.source {
                    RetrievalSource::FullText => entry.has_full_text = true,
                    RetrievalSource::Vector => entry.has_vector = true,
                }
            })
            .or_insert_with(|| CandidateAccumulator {
                score,
                has_full_text: result.source == RetrievalSource::FullText,
                has_vector: result.source == RetrievalSource::Vector,
            });
    }

    let mut fused = by_id
        .into_iter()
        .map(|(id, entry)| FusedCandidate {
            id,
            score: entry.score,
            match_type: match_type(entry.has_full_text, entry.has_vector),
        })
        .collect::<Vec<_>>();
    sort_fused(&mut fused);
    fused.truncate(limit);
    fused
}

/// 使用倒数排名融合全文与向量候选，并标记每个结果实际命中的来源组合。
///
/// 权重或 `rrf_k` 非法时使用稳定默认值，保证融合分数始终有限且可排序。
pub(crate) fn fuse_with_rrf(
    vector: &[RetrievedCandidate],
    keyword: &[RetrievedCandidate],
    k: usize,
    vector_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<FusedCandidate> {
    if limit == 0 {
        return Vec::new();
    }
    if vector.is_empty() || keyword.is_empty() {
        let source = if vector.is_empty() { keyword } else { vector };
        return deduplicate_by_score(source, limit);
    }

    let rrf_k = if k == 0 { DEFAULT_RRF_K } else { k };
    let vector_weight = effective_weight(vector_weight, DEFAULT_VECTOR_WEIGHT);
    let keyword_weight = effective_weight(keyword_weight, DEFAULT_KEYWORD_WEIGHT);
    let vector_ids = ranked_ids(vector);
    let keyword_ids = ranked_ids(keyword);

    let mut by_id = BTreeMap::<String, RrfAccumulator>::new();
    for (index, id) in vector_ids.into_iter().enumerate() {
        by_id.entry(id).or_default().vector_rank = Some(index + 1);
    }
    for (index, id) in keyword_ids.into_iter().enumerate() {
        by_id.entry(id).or_default().keyword_rank = Some(index + 1);
    }

    let mut fused = by_id
        .into_iter()
        .map(|(id, entry)| {
            let mut score = 0.0;
            if let Some(rank) = entry.vector_rank {
                score += vector_weight / (rrf_k + rank) as f32;
            }
            if let Some(rank) = entry.keyword_rank {
                score += keyword_weight / (rrf_k + rank) as f32;
            }
            FusedCandidate {
                id,
                score,
                match_type: match_type(entry.keyword_rank.is_some(), entry.vector_rank.is_some()),
            }
        })
        .collect::<Vec<_>>();
    sort_fused(&mut fused);
    fused.truncate(limit);
    fused
}

fn ranked_ids(candidates: &[RetrievedCandidate]) -> Vec<String> {
    let mut by_id = BTreeMap::<String, f32>::new();
    for candidate in candidates {
        let score = sanitize_score(candidate.score);
        by_id
            .entry(candidate.id.clone())
            .and_modify(|existing| *existing = (*existing).max(score))
            .or_insert(score);
    }
    let mut ranked = by_id.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        compare_score_desc(left.1, right.1).then_with(|| left.0.cmp(&right.0))
    });
    ranked.into_iter().map(|(id, _)| id).collect()
}

fn sanitize_score(score: f32) -> f32 {
    if score.is_nan() || score <= 0.0 || score == f32::NEG_INFINITY {
        0.0
    } else if score >= 1.0 || score == f32::INFINITY {
        1.0
    } else {
        score
    }
}

fn effective_weight(weight: f32, default: f32) -> f32 {
    if weight.is_finite() && weight > 0.0 { weight } else { default }
}

fn match_type(has_full_text: bool, has_vector: bool) -> KnowledgeMatchType {
    match (has_full_text, has_vector) {
        (true, true) => KnowledgeMatchType::Hybrid,
        (false, true) => KnowledgeMatchType::Vector,
        _ => KnowledgeMatchType::FullText,
    }
}

fn sort_fused(results: &mut [FusedCandidate]) {
    results.sort_by(|left, right| {
        compare_score_desc(left.score, right.score)
            .then_with(|| {
                match_type_rank(&left.match_type).cmp(&match_type_rank(&right.match_type))
            })
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn compare_score_desc(left: f32, right: f32) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn match_type_rank(match_type: &KnowledgeMatchType) -> u8 {
    match match_type {
        KnowledgeMatchType::Hybrid => 0,
        KnowledgeMatchType::Vector => 1,
        KnowledgeMatchType::FullText => 2,
        KnowledgeMatchType::Parent => 3,
        KnowledgeMatchType::Nearby => 4,
    }
}

#[cfg(test)]
#[path = "fusion_tests.rs"]
mod fusion_tests;
