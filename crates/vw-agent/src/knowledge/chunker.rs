//! 本地知识导入的文本分块器，保留标题上下文、来源偏移与重叠窗口。
//!
//! 分块按字符而非字节计数，不在 Markdown 围栏内部按普通段落切割，并携带最近标题层级。
//!
//! Text chunking for local knowledge ingestion.

mod patterns;
mod profile;

use patterns::{MarkdownHeading, is_fence_marker, parse_markdown_heading};
use profile::DocProfile;

const DEFAULT_CHUNK_CHARS: usize = 700;
const DEFAULT_OVERLAP_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 一段可索引文本及其在原文中的位置和标题上下文。
pub(crate) struct TextChunk {
    /// 分块在文档中的零基顺序。
    pub ordinal: usize,
    /// 原始分块正文。
    pub content: String,
    /// 在原文中的起始字节偏移。
    pub start_at: usize,
    /// 在原文中的结束字节偏移。
    pub end_at: usize,
    /// 供检索理解上下文的 Markdown 标题链。
    pub context_header: String,
}

impl TextChunk {
    /// 返回用于生成向量的文本，在正文前拼接非空标题上下文。
    pub(crate) fn embedding_content(&self) -> String {
        let body = self.content.trim();
        if self.context_header.is_empty() {
            return body.to_string();
        }
        if body.is_empty() {
            return self.context_header.clone();
        }
        format!("{}\n\n{body}", self.context_header)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadingBoundary {
    start_at: usize,
    heading: Option<MarkdownHeading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeadingHierarchy {
    headings: [Option<String>; 6],
}

impl Default for HeadingHierarchy {
    fn default() -> Self {
        Self { headings: std::array::from_fn(|_| None) }
    }
}

impl HeadingHierarchy {
    fn observe(&mut self, heading: &MarkdownHeading) {
        let index = heading.level.saturating_sub(1);
        if index >= self.headings.len() {
            return;
        }
        self.headings[index] = Some(heading.title.clone());
        for deeper in heading.level..self.headings.len() {
            self.headings[deeper] = None;
        }
    }

    fn context_header(&self) -> String {
        let mut lines = Vec::new();
        for (index, heading) in self.headings.iter().enumerate() {
            if let Some(heading) = heading {
                lines.push(format!("{} {heading}", "#".repeat(index + 1)));
            }
        }
        lines.join("\n")
    }
}

/// 使用默认字符上限和重叠窗口切分文本。
pub(crate) fn chunk_text(text: &str) -> Vec<TextChunk> {
    chunk_text_with_limits(text, DEFAULT_CHUNK_CHARS, DEFAULT_OVERLAP_CHARS)
}

/// 使用显式字符上限和重叠大小切分文本。
pub(crate) fn chunk_text_with_limits(
    text: &str,
    chunk_chars: usize,
    overlap_chars: usize,
) -> Vec<TextChunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let chunk_chars = chunk_chars.max(1);
    let overlap_chars = overlap_chars.min(chunk_chars.saturating_sub(1));

    let profile = DocProfile::from_text(text);
    if profile.should_use_heading_split() {
        let chunks = split_by_headings(text, chunk_chars, overlap_chars, &profile);
        if validate_chunks(&chunks, profile.total_chars, chunk_chars).is_ok() {
            return chunks;
        }
    }

    split_legacy(text, chunk_chars, overlap_chars, "")
}

fn split_by_headings(
    text: &str,
    chunk_chars: usize,
    overlap_chars: usize,
    profile: &DocProfile,
) -> Vec<TextChunk> {
    let Some(dominant_level) = profile.dominant_heading_level() else {
        return Vec::new();
    };
    let boundaries = heading_boundaries(text, dominant_level);
    if boundaries.len() <= 1 {
        return Vec::new();
    }

    let chars = text.chars().collect::<Vec<_>>();
    let protected_spans = fenced_code_spans(text);
    let mut chunks = Vec::new();
    let mut hierarchy = HeadingHierarchy::default();

    for (index, boundary) in boundaries.iter().enumerate() {
        if let Some(heading) = &boundary.heading {
            hierarchy.observe(heading);
        }
        let section_start = boundary.start_at;
        let section_end = boundaries.get(index + 1).map_or(chars.len(), |next| next.start_at);
        let (content_start, content_end) = trim_bounds(&chars, section_start, section_end);
        if content_start >= content_end {
            continue;
        }
        let context_header = hierarchy.context_header();
        if content_end - content_start <= chunk_chars {
            chunks.push(TextChunk {
                ordinal: chunks.len(),
                content: collect_chars(&chars, content_start, content_end),
                start_at: content_start,
                end_at: content_end,
                context_header,
            });
            continue;
        }
        let mut section_chunks = split_legacy_chars(
            &chars,
            section_start,
            section_end,
            chunk_chars,
            overlap_chars,
            &context_header,
            &protected_spans,
        );
        chunks.append(&mut section_chunks);
        resequence(&mut chunks);
    }

    chunks
}

fn split_legacy(
    text: &str,
    chunk_chars: usize,
    overlap_chars: usize,
    context_header: &str,
) -> Vec<TextChunk> {
    let chars = text.chars().collect::<Vec<_>>();
    let protected_spans = fenced_code_spans(text);
    split_legacy_chars(
        &chars,
        0,
        chars.len(),
        chunk_chars,
        overlap_chars,
        context_header,
        &protected_spans,
    )
}

fn split_legacy_chars(
    chars: &[char],
    range_start: usize,
    range_end: usize,
    chunk_chars: usize,
    overlap_chars: usize,
    context_header: &str,
    protected_spans: &[Span],
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut start = range_start;

    while start < range_end {
        start = next_non_whitespace(chars, start, range_end);
        if start >= range_end {
            break;
        }
        let mut end = (start + chunk_chars).min(range_end);
        if end < range_end && remaining_fits_tail_in_range(start, range_end, chunk_chars) {
            end = range_end;
        }
        if end < range_end {
            end = prefer_boundary(&chars, start, end).max(start + 1);
        }
        end = adjust_end_for_protected(start, end, range_end, protected_spans);
        let (content_start, content_end) = trim_bounds(chars, start, end);
        if content_start < content_end {
            chunks.push(TextChunk {
                ordinal: chunks.len(),
                content: collect_chars(chars, content_start, content_end),
                start_at: content_start,
                end_at: content_end,
                context_header: context_header.to_string(),
            });
        }
        if end >= range_end {
            break;
        }
        let mut next_start = end.saturating_sub(overlap_chars).max(range_start);
        next_start = adjust_start_for_protected_overlap(next_start, end, protected_spans);
        if next_start <= start {
            next_start = end;
        }
        start = next_start;
    }

    chunks
}

fn heading_boundaries(text: &str, dominant_level: usize) -> Vec<HeadingBoundary> {
    let mut boundaries = vec![HeadingBoundary { start_at: 0, heading: None }];
    let mut position = 0;
    let mut in_fence = false;

    for segment in text.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if is_fence_marker(line) {
            in_fence = !in_fence;
            position += segment.chars().count();
            continue;
        }
        if !in_fence {
            if let Some(heading) = parse_markdown_heading(line) {
                if heading.level <= dominant_level {
                    if position == 0 {
                        boundaries[0].heading = Some(heading);
                    } else {
                        boundaries
                            .push(HeadingBoundary { start_at: position, heading: Some(heading) });
                    }
                }
            }
        }
        position += segment.chars().count();
    }

    boundaries
}

fn fenced_code_spans(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut position = 0;
    let mut fence_start = None;

    for segment in text.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if is_fence_marker(line) {
            if let Some(start) = fence_start.take() {
                spans.push(Span { start, end: position + segment.chars().count() });
            } else {
                fence_start = Some(position);
            }
        }
        position += segment.chars().count();
    }

    if let Some(start) = fence_start {
        spans.push(Span { start, end: position });
    }

    spans
}

#[cfg(test)]
fn remaining_fits_tail(chars: &[char], start: usize, chunk_chars: usize) -> bool {
    remaining_fits_tail_in_range(start, chars.len(), chunk_chars)
}

fn remaining_fits_tail_in_range(start: usize, range_end: usize, chunk_chars: usize) -> bool {
    let remaining = range_end.saturating_sub(start);
    let tail_slack = (chunk_chars / 4).max(1);
    remaining <= chunk_chars.saturating_add(tail_slack)
}

#[cfg(test)]
fn normalize_text(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

fn prefer_boundary(chars: &[char], start: usize, end: usize) -> usize {
    let min = start + ((end - start) / 2);
    for index in (min..end).rev() {
        if matches!(chars[index], '\n' | '。' | '！' | '？' | '.' | '!' | '?') {
            return index + 1;
        }
    }
    end
}

fn adjust_end_for_protected(
    start: usize,
    end: usize,
    range_end: usize,
    protected_spans: &[Span],
) -> usize {
    let mut adjusted = end;
    for span in protected_spans {
        if span.end <= start || span.start >= range_end {
            continue;
        }
        if span.start < adjusted && adjusted < span.end {
            adjusted = span.end.min(range_end);
        }
    }
    adjusted
}

fn adjust_start_for_protected_overlap(
    start: usize,
    previous_end: usize,
    protected_spans: &[Span],
) -> usize {
    for span in protected_spans {
        if span.start < start && start < span.end && previous_end >= span.end {
            return span.end;
        }
    }
    start
}

fn next_non_whitespace(chars: &[char], start: usize, end: usize) -> usize {
    let mut index = start;
    while index < end && chars[index].is_whitespace() {
        index += 1;
    }
    index
}

fn trim_bounds(chars: &[char], start: usize, end: usize) -> (usize, usize) {
    let mut trimmed_start = start;
    let mut trimmed_end = end;
    while trimmed_start < trimmed_end && chars[trimmed_start].is_whitespace() {
        trimmed_start += 1;
    }
    while trimmed_end > trimmed_start && chars[trimmed_end - 1].is_whitespace() {
        trimmed_end -= 1;
    }
    (trimmed_start, trimmed_end)
}

fn collect_chars(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end].iter().collect()
}

fn resequence(chunks: &mut [TextChunk]) {
    for (ordinal, chunk) in chunks.iter_mut().enumerate() {
        chunk.ordinal = ordinal;
    }
}

fn validate_chunks(
    chunks: &[TextChunk],
    total_chars: usize,
    chunk_chars: usize,
) -> Result<(), &'static str> {
    if chunks.is_empty() {
        return Err("no chunks produced");
    }
    if chunks.len() == 1 && total_chars > chunk_chars.saturating_mul(2) {
        return Err("single chunk for large document");
    }

    let max_len = chunks.iter().map(|chunk| chunk.content.chars().count()).max().unwrap_or(0);
    if max_len > chunk_chars.saturating_mul(2) {
        return Err("chunk exceeds 2x target size");
    }

    let tiny_limit = (chunk_chars / 4).clamp(20, 50);
    let tiny_count = chunks
        .iter()
        .take(chunks.len().saturating_sub(1))
        .filter(|chunk| chunk.content.chars().count() < tiny_limit)
        .count();
    if tiny_count > chunks.len() / 4 && tiny_count > 2 {
        return Err("too many tiny chunks");
    }

    Ok(())
}

#[cfg(test)]
#[path = "chunker_tests.rs"]
mod chunker_tests;
