//! 源码驱动助手流的管道表格暂留扫描器。
//!
//! 活动表格保持为可变尾部，使新增行能重新排版之前的表格行，而非提交过期布局。
//! 扫描器只识别决定尾部起点所需的结构，不验证完整表格；最终布局仍由 Markdown 渲染器负责。
//! Pipe-table holdback scanner for source-backed agent streams.
//!
//! Agent streams with markdown tables keep the active table as mutable tail so
//! adding a row can reflow earlier table rows instead of committing a stale
//! render to scrollback.
//!
//! The scanner is intentionally conservative: it only looks for enough
//! structure to decide where the mutable tail should start. It does not try to
//! validate an entire table or predict final layout. Rendering remains the job
//! of the markdown renderer.

use std::time::Instant;

use crate::table_detect::FenceKind;
use crate::table_detect::FenceTracker;
use crate::table_detect::is_table_delimiter_line;
use crate::table_detect::is_table_header_line;
use crate::table_detect::parse_table_segments;
use crate::table_detect::strip_blockquote_prefix;

/// 扫描累计原始源码中管道表格模式的结果。
/// Result of scanning accumulated raw source for pipe-table patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TableHoldbackState {
    /// 未检测到表格，所有渲染行都可进入稳定队列。
    /// No table detected -- all rendered lines can flow into the stable queue.
    None,
    /// 最后一个非空行像表头，但尚未出现分隔行；暂留它以等待下一增量确认。
    /// The last non-blank line looks like a table header row but no delimiter
    /// row has followed yet. Hold back in case the next delta is a delimiter.
    PendingHeader { header_start: usize },
    /// 已找到表头与分隔行对；从表头开始的源码都保持可变。
    /// A header + delimiter pair was found -- the source contains a confirmed
    /// table. Content from the table header onward stays mutable.
    Confirmed { table_start: usize },
}

/// 关于上一条已提交源码行所记住的事实。
///
/// 表格由紧邻的表头和分隔行确认，因此扫描器只需回看一行。
/// Facts remembered about the previous committed source line.
///
/// The scanner only needs one-line lookbehind because a table is confirmed by
/// a header row immediately followed by a delimiter row.
#[derive(Clone, Copy)]
struct PreviousLineState {
    source_start: usize,
    fence_kind: FenceKind,
    is_header: bool,
}

/// 只追加源码流中表格暂留状态的增量扫描器。
///
/// 分块必须按最终追加顺序提供；扫描器保存逻辑源码缓冲区的字节偏移，乱序会让尾部边界失真。
/// Incremental scanner for table holdback state on append-only source streams.
///
/// `push_source_chunk` must receive source in the same order it will be
/// appended to the stream. The scanner stores byte offsets into that logical
/// source buffer, so feeding chunks out of order would make later tail
/// boundaries point at the wrong rendered region.
pub(super) struct TableHoldbackScanner {
    source_offset: usize,
    fence_tracker: FenceTracker,
    previous_line: Option<PreviousLineState>,
    pending_header_start: Option<usize>,
    confirmed_table_start: Option<usize>,
}

impl TableHoldbackScanner {
    pub(super) fn new() -> Self {
        Self {
            source_offset: 0,
            fence_tracker: FenceTracker::new(),
            previous_line: None,
            pending_header_start: None,
            confirmed_table_start: None,
        }
    }

    pub(super) fn reset(&mut self) {
        *self = Self::new();
    }

    /// 返回当前已提交源码前缀的暂留决定。
    ///
    /// 待定表头会乐观保留最新候选区域；确认表格后，表头及后续正文行一直留在实时尾部直至定稿。
    /// Return the current holdback decision for the committed source prefix.
    ///
    /// `PendingHeader` means the newest non-blank line looks like a table
    /// header but the delimiter row has not arrived yet, so callers should
    /// optimistically keep that region mutable. `Confirmed` means the header
    /// and delimiter pair has been seen and all subsequent body rows remain in
    /// the live tail until finalization.
    pub(super) fn state(&self) -> TableHoldbackState {
        if let Some(table_start) = self.confirmed_table_start {
            TableHoldbackState::Confirmed { table_start }
        } else if let Some(header_start) = self.pending_header_start {
            TableHoldbackState::PendingHeader { header_start }
        } else {
            TableHoldbackState::None
        }
    }

    /// 使用新提交源码推进扫描器。
    ///
    /// 分块通常是流收集器确认安全的换行终止行；排除局部行，避免把未完成行当作稳定结构信号。
    /// Advance the scanner with newly committed source.
    ///
    /// Chunks are expected to contain only source that is now safe to commit
    /// into `raw_source`, typically newline-terminated lines from the
    /// streaming collector. Partial rows are intentionally excluded so the
    /// scanner never treats an unfinished table row as a stable structural
    /// signal.
    pub(super) fn push_source_chunk(&mut self, source_chunk: &str) {
        if source_chunk.is_empty() {
            return;
        }

        let scan_start = Instant::now();
        let mut lines = 0usize;
        for source_line in source_chunk.split_inclusive('\n') {
            lines += 1;
            self.push_line(source_line);
        }
        tracing::trace!(
            bytes = source_chunk.len(),
            lines,
            state = ?self.state(),
            elapsed_us = scan_start.elapsed().as_micros(),
            "table holdback incremental scan",
        );
    }

    /// 将一条已提交源码行折叠进扫描器状态机。
    /// Fold one committed source line into the scanner state machine.
    fn push_line(&mut self, source_line: &str) {
        let line = source_line.strip_suffix('\n').unwrap_or(source_line);
        let source_start = self.source_offset;
        let fence_kind = self.fence_tracker.kind();

        let candidate_text =
            if fence_kind == FenceKind::Other { None } else { table_candidate_text(line) };
        let is_header = candidate_text.is_some_and(is_table_header_line);
        let is_delimiter = candidate_text.is_some_and(is_table_delimiter_line);

        if self.confirmed_table_start.is_none()
            && let Some(previous_line) = self.previous_line
            && previous_line.fence_kind != FenceKind::Other
            && fence_kind != FenceKind::Other
            && previous_line.is_header
            && is_delimiter
        {
            self.confirmed_table_start = Some(previous_line.source_start);
            self.pending_header_start = None;
        }

        if self.confirmed_table_start.is_none() && !line.trim().is_empty() {
            if fence_kind != FenceKind::Other && is_header {
                self.pending_header_start = Some(source_start);
            } else {
                self.pending_header_start = None;
            }
        }

        self.previous_line = Some(PreviousLineState { source_start, fence_kind, is_header });

        self.fence_tracker.advance(line);
        self.source_offset = self.source_offset.saturating_add(source_line.len());
    }
}

/// 去除块引用前缀；若剩余文本具有管道表格片段则返回裁切结果，否则返回空。
///
/// 引用内表格仍按真实表格处理，但去除引用标记后仍必须满足管道表格形状。
/// Strip blockquote prefixes and return the trimmed text if it contains
/// pipe-table segments, or `None` otherwise.
///
/// Table holdback treats quoted tables as real tables, but it still requires a
/// pipe-table shape after the quote markers are removed.
fn table_candidate_text(line: &str) -> Option<&str> {
    let stripped = strip_blockquote_prefix(line).trim();
    parse_table_segments(stripped).map(|_| stripped)
}

/// 标注是否位于围栏代码块内的源码行。
/// A source line annotated with whether it falls inside a fenced code block.
#[cfg(test)]
struct ParsedLine<'a> {
    text: &'a str,
    fence_context: FenceKind,
    source_start: usize,
}

/// 把源码解析为带围栏代码上下文标记的行，供表格扫描使用。
/// Parse source into lines tagged with fenced-code context for table scanning.
#[cfg(test)]
fn parse_lines_with_fence_state(source: &str) -> Vec<ParsedLine<'_>> {
    let mut tracker = FenceTracker::new();
    let mut lines = Vec::new();
    let mut source_start = 0usize;

    for raw_line in source.split('\n') {
        lines.push(ParsedLine { text: raw_line, fence_context: tracker.kind(), source_start });

        tracker.advance(raw_line);
        source_start = source_start.saturating_add(raw_line.len()).saturating_add(1);
    }

    lines
}

/// 在非 Markdown 围栏代码块之外扫描 `source` 中的管道表格模式。
/// Scan `source` for pipe-table patterns outside of non-markdown fenced code
/// blocks.
#[cfg(test)]
pub(super) fn table_holdback_state(source: &str) -> TableHoldbackState {
    let lines = parse_lines_with_fence_state(source);
    for pair in lines.windows(2) {
        let [header_line, delimiter_line] = pair else {
            continue;
        };
        if header_line.fence_context == FenceKind::Other
            || delimiter_line.fence_context == FenceKind::Other
        {
            continue;
        }

        let Some(header_text) = table_candidate_text(header_line.text) else {
            continue;
        };
        let Some(delimiter_text) = table_candidate_text(delimiter_line.text) else {
            continue;
        };

        if is_table_header_line(header_text) && is_table_delimiter_line(delimiter_text) {
            return TableHoldbackState::Confirmed { table_start: header_line.source_start };
        }
    }

    let pending_header = lines.iter().rev().find(|line| !line.text.trim().is_empty());
    if let Some(line) = pending_header
        && line.fence_context != FenceKind::Other
        && table_candidate_text(line.text).is_some_and(is_table_header_line)
    {
        return TableHoldbackState::PendingHeader { header_start: line.source_start };
    }
    TableHoldbackState::None
}
