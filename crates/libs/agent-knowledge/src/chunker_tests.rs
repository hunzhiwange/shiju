// 覆盖知识文本切块的大小限制、重叠与元数据保留。

// 每个块都核对原文偏移，裁剪外部空白不能破坏回溯到源文本的位置。
use super::*;

// 通过字节偏移回切原文，直接证明块元数据仍指向正确的 Unicode 源片段。
fn source_slice(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end - start).collect()
}

// 所有块统一检查偏移边界，避免只验证文本而漏掉不可回溯的元数据。
fn assert_source_offsets(text: &str, chunk: &TextChunk) {
    assert_eq!(source_slice(text, chunk.start_at, chunk.end_at), chunk.content);
    assert_eq!(chunk.end_at - chunk.start_at, chunk.content.chars().count());
}

#[test]
fn chunk_text_returns_no_chunks_for_blank_input() {
    assert!(chunk_text("").is_empty());
    assert!(chunk_text(" \n\t\n ").is_empty());
}

#[test]
fn chunk_text_preserves_source_offsets_after_trimming_outer_whitespace() {
    let chunks = chunk_text_with_limits("  alpha  \n\n beta \n", 50, 0);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].ordinal, 0);
    assert_eq!(chunks[0].start_at, 2);
    assert_eq!(chunks[0].content, "alpha  \n\n beta");
    assert_source_offsets("  alpha  \n\n beta \n", &chunks[0]);
}

#[test]
fn embedding_content_handles_missing_header_or_body() {
    let body_only = TextChunk {
        ordinal: 0,
        content: "  body text  ".to_string(),
        start_at: 0,
        end_at: 13,
        context_header: String::new(),
    };
    assert_eq!(body_only.embedding_content(), "body text");

    let header_only = TextChunk {
        ordinal: 0,
        content: "  \n ".to_string(),
        start_at: 0,
        end_at: 4,
        context_header: "# Guide".to_string(),
    };
    assert_eq!(header_only.embedding_content(), "# Guide");
}

#[test]
fn chunking_prefers_english_sentence_boundaries() {
    let chunks = chunk_text_with_limits("alpha beta. gamma delta. epsilon.", 18, 0);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "alpha beta.");
    assert_source_offsets("alpha beta. gamma delta. epsilon.", &chunks[0]);
    assert_eq!(chunks[1].ordinal, 1);
}

#[test]
fn chunking_prefers_cjk_sentence_boundaries() {
    let chunks = chunk_text_with_limits("你好世界。后续内容继续。", 6, 0);

    assert_eq!(chunks[0].content, "你好世界。");
    assert!(chunks[1].content.starts_with("后续"));
}

#[test]
fn chunking_uses_overlap_between_adjacent_chunks() {
    let chunks = chunk_text_with_limits("abcdef", 3, 1);

    assert_eq!(
        chunks.iter().map(|chunk| chunk.content.as_str()).collect::<Vec<_>>(),
        vec!["abc", "cdef"]
    );
    assert_eq!((chunks[0].start_at, chunks[0].end_at), (0, 3));
    assert_eq!((chunks[1].start_at, chunks[1].end_at), (2, 6));
}

#[test]
fn chunk_and_overlap_limits_are_clamped_to_progress() {
    let chunks = chunk_text_with_limits("abc", 0, usize::MAX);

    assert_eq!(
        chunks.iter().map(|chunk| chunk.content.as_str()).collect::<Vec<_>>(),
        vec!["a", "bc"]
    );
}

#[test]
fn small_tail_is_folded_into_previous_chunk() {
    let chunks = chunk_text_with_limits("abcd", 3, 0);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "abcd");
}

#[test]
fn private_helpers_cover_empty_and_boundary_cases() {
    assert!(remaining_fits_tail(&['a', 'b', 'c'], 0, 3));
    assert!(!remaining_fits_tail(&['a', 'b', 'c', 'd', 'e'], 0, 3));

    let chars = "one\ntwo".chars().collect::<Vec<_>>();
    assert_eq!(prefer_boundary(&chars, 0, 5), 4);
    assert_eq!(prefer_boundary(&['a', 'b', 'c'], 0, 3), 3);
    assert_eq!(normalize_text("  a \n b  "), "a\nb");
}

#[test]
fn heading_chunks_keep_breadcrumbs_out_of_content() {
    let markdown = "\
# Guide
Overview paragraph explains the guide purpose and keeps the opening section long enough.

## Install
Install instructions include setup details, commands, and expected results for operators.

## Usage
Usage instructions explain the daily workflow and how the local runtime should be queried.

## Install
Repeated install notes cover platform-specific details without changing the breadcrumb shape.
";

    let chunks = chunk_text_with_limits(markdown, 180, 0);

    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0].context_header, "# Guide");
    assert_eq!(chunks[1].context_header, "# Guide\n## Install");
    assert_eq!(chunks[2].context_header, "# Guide\n## Usage");
    assert_eq!(chunks[3].context_header, "# Guide\n## Install");
    for (ordinal, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.ordinal, ordinal);
        assert_source_offsets(markdown, chunk);
    }

    assert!(chunks[1].content.starts_with("## Install\nInstall instructions"));
    assert!(!chunks[1].content.starts_with("# Guide\n## Install"));
    assert!(
        chunks[1]
            .embedding_content()
            .starts_with("# Guide\n## Install\n\n## Install\nInstall instructions")
    );
}

#[test]
fn heading_chunker_does_not_split_or_parse_fenced_code_blocks() {
    let markdown = "\
# Notes
Opening notes are long enough to make a useful section before the protected code sample.

## Code
```rust
# not a markdown heading
fn main() {
    println!(\"hello\");
}
```
The explanation after the code remains in the same section.

## Result
The result section describes output and behaviour with enough words for a useful chunk.

## Follow up
Follow up text gives another repeated heading boundary for the profile.
";

    let chunks = chunk_text_with_limits(markdown, 80, 0);
    let code_chunks =
        chunks.iter().filter(|chunk| chunk.content.contains("fn main")).collect::<Vec<_>>();

    assert_eq!(code_chunks.len(), 1);
    assert!(code_chunks[0].content.contains("```rust"));
    assert!(code_chunks[0].content.contains("```"));
    assert_source_offsets(markdown, code_chunks[0]);
    assert!(chunks.iter().all(|chunk| !chunk.context_header.contains("not a markdown heading")));
}

#[test]
fn unclosed_fence_is_protected_through_end_of_document() {
    let text = "intro\n```rust\nlet value = 1;";

    assert_eq!(fenced_code_spans(text), vec![Span { start: 6, end: text.chars().count() }]);
}

#[test]
fn validate_chunks_rejects_invalid_chunk_shapes() {
    let chunk = |content: &str| TextChunk {
        ordinal: 0,
        content: content.to_string(),
        start_at: 0,
        end_at: content.chars().count(),
        context_header: String::new(),
    };

    assert_eq!(validate_chunks(&[], 0, 10), Err("no chunks produced"));
    assert_eq!(
        validate_chunks(&[chunk("content")], 21, 10),
        Err("single chunk for large document")
    );
    assert_eq!(
        validate_chunks(&[chunk("abcdefghijklmnopqrstu"), chunk("tail")], 25, 10),
        Err("chunk exceeds 2x target size")
    );

    let tiny_chunks = vec![chunk("a"), chunk("b"), chunk("c"), chunk("d"), chunk("final")];
    assert_eq!(validate_chunks(&tiny_chunks, 9, 100), Err("too many tiny chunks"));
}
