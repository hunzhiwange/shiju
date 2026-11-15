use super::{ScoredResult, bytes_to_vec, hybrid_merge, vec_to_bytes};

#[test]
fn vector_reexport_preserves_scored_result_identity_and_encoding() {
    let merged: Vec<memory_algorithms::vector::ScoredResult> =
        hybrid_merge(&[("记忆".to_string(), 1.0)], &[], 1.0, 0.0, 1);
    assert_eq!(merged.len(), 1);
    let result: &ScoredResult = &merged[0];
    assert_eq!(result.id, "记忆");
    assert_eq!(result.vector_score, Some(1.0));
    assert_eq!(result.keyword_score, None);
    let original = vec![1.0_f32, 2.0, 3.0];
    let bytes = vec_to_bytes(&original);
    assert_eq!(bytes, memory_algorithms::vector::vec_to_bytes(&original));
    assert_eq!(bytes_to_vec(&bytes), original);
    assert_eq!(memory_algorithms::vector::bytes_to_vec(&bytes), original);
}
