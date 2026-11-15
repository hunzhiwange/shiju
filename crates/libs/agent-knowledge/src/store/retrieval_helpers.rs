//! 共用的候选融合、重排与相邻分块文本合并。
use super::{MAX_CANDIDATES, MIN_OVERLAP_CHARS, DEFAULT_CONTEXT_SEARCH_SPAN};
use serde_json::Value;
use vw_api_knowledge::KnowledgeChunkDto;
use crate::fusion::{FusedCandidate, RetrievedCandidate, deduplicate_by_score, fuse_with_rrf};

pub fn merge_retrieval_scores(
    vector_results: &[RetrievedCandidate],
    keyword_results: &[RetrievedCandidate],
    vector_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<FusedCandidate> {
    if !vector_results.is_empty() && !keyword_results.is_empty() {
        return fuse_with_rrf(
            vector_results,
            keyword_results,
            60,
            vector_weight,
            keyword_weight,
            limit,
        );
    }
    let source = if vector_results.is_empty() { keyword_results } else { vector_results };
    deduplicate_by_score(source, limit)
}

pub fn keyword_limit(keyword_count: usize) -> usize {
    keyword_count.clamp(1, MAX_CANDIDATES)
}

pub fn rerank_chunks(query: &str, rerank_dataset_ids: &[String], chunks: &mut [KnowledgeChunkDto]) {
    chunks.sort_by(|left, right| {
        let left_score = rerank_score(query, rerank_dataset_ids, left);
        let right_score = rerank_score(query, rerank_dataset_ids, right);
        right_score.partial_cmp(&left_score).unwrap_or(std::cmp::Ordering::Equal)
    });
    for chunk in chunks {
        if rerank_dataset_ids.iter().any(|dataset_id| dataset_id == &chunk.dataset_id) {
            chunk.score = Some(rerank_score(query, rerank_dataset_ids, chunk));
        }
    }
}

pub fn rerank_score(query: &str, rerank_dataset_ids: &[String], chunk: &KnowledgeChunkDto) -> f64 {
    let base = chunk.score.unwrap_or_default();
    if !rerank_dataset_ids.iter().any(|dataset_id| dataset_id == &chunk.dataset_id) {
        return base;
    }
    let lexical = lexical_match_score(query, chunk);
    (base * 0.70 + lexical * 0.30).clamp(0.0, 1.0)
}

pub fn lexical_match_score(query: &str, chunk: &KnowledgeChunkDto) -> f64 {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return 0.0;
    }
    let mut haystack = format!("{}\n{}", chunk.title, chunk.content).to_lowercase();
    if let Some(child_content) = chunk.metadata.get("child_content").and_then(Value::as_str) {
        haystack.push('\n');
        haystack.push_str(&child_content.to_lowercase());
    }
    if let Some(question) = chunk.metadata.get("question").and_then(Value::as_str) {
        haystack.push('\n');
        haystack.push_str(&question.to_lowercase());
    }
    if haystack.contains(&query) {
        return 1.0;
    }
    let terms = query.split_whitespace().filter(|term| !term.is_empty()).collect::<Vec<_>>();
    if terms.is_empty() {
        return 0.0;
    }
    let matched = terms.iter().filter(|term| haystack.contains(**term)).count();
    matched as f64 / terms.len() as f64
}

pub fn merge_text_chunks(chunks: &[KnowledgeChunkDto], gap_sep: &str) -> String {
    let mut sorted = chunks.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.start_at
            .cmp(&right.start_at)
            .then_with(|| left.ordinal.cmp(&right.ordinal))
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut merged = String::new();
    let mut merged_end = None;
    for chunk in sorted {
        if chunk.content.is_empty() {
            continue;
        }
        if merged.is_empty() {
            merged.push_str(&chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        }

        let Some(last_end) = merged_end else {
            append_gap_and_text(&mut merged, gap_sep, &chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        };
        if chunk.end_at == 0 || chunk.start_at > last_end {
            append_gap_and_text(&mut merged, gap_sep, &chunk.content);
            if chunk.end_at > 0 {
                merged_end = Some(chunk.end_at);
            }
            continue;
        }
        if chunk.end_at > last_end {
            merged = append_with_overlap(
                &merged,
                &chunk.content,
                last_end.saturating_sub(chunk.start_at),
            );
            merged_end = Some(chunk.end_at);
        }
    }
    merged
}

pub fn append_gap_and_text(target: &mut String, gap_sep: &str, text: &str) {
    if !gap_sep.is_empty() {
        target.push_str(gap_sep);
    }
    target.push_str(text);
}

pub fn append_with_overlap(acc: &str, next: &str, position_overlap: usize) -> String {
    if acc.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return acc.to_string();
    }

    let acc_chars = acc.chars().collect::<Vec<_>>();
    let next_chars = next.chars().collect::<Vec<_>>();
    let mut max_overlap = acc_chars.len().min(next_chars.len());
    max_overlap =
        max_overlap.min((position_overlap.saturating_mul(3)).max(DEFAULT_CONTEXT_SEARCH_SPAN));
    let head_slack = (position_overlap.saturating_mul(2)).max(320);

    for overlap in (MIN_OVERLAP_CHARS..=max_overlap).rev() {
        let needle = &acc_chars[acc_chars.len() - overlap..];
        if let Some(position) = index_chars(&next_chars, needle, head_slack) {
            let suffix = next_chars[position + overlap..].iter().collect::<String>();
            return format!("{acc}{suffix}");
        }
    }

    format!("{acc}{next}")
}

pub fn index_chars(haystack: &[char], needle: &[char], max_start: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let limit = haystack.len().saturating_sub(needle.len()).min(max_start);
    (0..=limit).find(|position| &haystack[*position..*position + needle.len()] == needle)
}

pub fn non_empty_context(context: String) -> Option<String> {
    (!context.is_empty()).then_some(context)
}
