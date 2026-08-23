// 验证多路检索结果的归一化、去重与排序融合。

// 融合排序使用稳定默认参数，单一检索源则退回按分数去重。
use super::{RetrievalSource, RetrievedCandidate, deduplicate_by_score, fuse_with_rrf};
use vw_api_types::knowledge::KnowledgeMatchType;

fn candidate(id: &str, score: f32, source: RetrievalSource) -> RetrievedCandidate {
    RetrievedCandidate { id: id.to_string(), score, source }
}

#[test]
fn rrf_fuses_by_rank_and_classifies_hybrid_matches() {
    let vector = vec![
        candidate("vector-only", 0.99, RetrievalSource::Vector),
        candidate("shared", 0.80, RetrievalSource::Vector),
    ];
    let keyword = vec![
        candidate("shared", 0.90, RetrievalSource::FullText),
        candidate("keyword-only", 0.70, RetrievalSource::FullText),
    ];

    let fused = fuse_with_rrf(&vector, &keyword, 60, 0.7, 0.3, 10);

    assert_eq!(fused[0].id, "shared");
    assert_eq!(fused[0].match_type, KnowledgeMatchType::Hybrid);
    assert_eq!(fused.iter().filter(|result| result.id == "shared").count(), 1);
    assert!(fused.iter().all(|result| result.score.is_finite()));
}

#[test]
fn deduplicate_by_score_keeps_single_source_scores_stable() {
    let results = deduplicate_by_score(
        &[
            candidate("a", 0.2, RetrievalSource::Vector),
            candidate("a", 0.8, RetrievalSource::Vector),
            candidate("nan", f32::NAN, RetrievalSource::FullText),
            candidate("inf", f32::INFINITY, RetrievalSource::FullText),
        ],
        3,
    );

    assert_eq!(results[0].id, "inf");
    assert_eq!(results[0].score, 1.0);
    assert_eq!(results[1].id, "a");
    assert_eq!(results[1].score, 0.8);
    assert_eq!(results[1].match_type, KnowledgeMatchType::Vector);
    assert!(results.iter().all(|result| result.score.is_finite()));
}

#[test]
fn zero_limit_returns_no_candidates_for_deduplication_or_rrf() {
    let candidate = candidate("only", 0.8, RetrievalSource::Vector);

    assert!(deduplicate_by_score(std::slice::from_ref(&candidate), 0).is_empty());
    assert!(fuse_with_rrf(&[candidate], &[], 60, 0.7, 0.3, 0).is_empty());
}

#[test]
fn single_retrieval_source_falls_back_to_score_deduplication() {
    let keyword = vec![
        candidate("same", 0.4, RetrievalSource::FullText),
        candidate("same", 0.9, RetrievalSource::FullText),
        candidate("other", 0.7, RetrievalSource::FullText),
    ];

    let fused = fuse_with_rrf(&[], &keyword, 60, 0.7, 0.3, 10);

    assert_eq!(fused.len(), 2);
    assert_eq!(fused[0].id, "same");
    assert_eq!(fused[0].score, 0.9);
    assert_eq!(fused[0].match_type, KnowledgeMatchType::FullText);
}

#[test]
fn invalid_rrf_parameters_use_stable_defaults() {
    let vector = vec![candidate("vector", 0.9, RetrievalSource::Vector)];
    let keyword = vec![candidate("keyword", 0.9, RetrievalSource::FullText)];

    let defaults = fuse_with_rrf(&vector, &keyword, 60, 0.7, 0.3, 10);
    let fallback = fuse_with_rrf(&vector, &keyword, 0, f32::NAN, -1.0, 10);

    assert_eq!(fallback, defaults);
}
