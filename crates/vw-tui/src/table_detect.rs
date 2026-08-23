//! 对原始 Markdown 源码执行规范管道表格结构检测和围栏代码块跟踪。
//!
//! 流式控制器与 Markdown 围栏展开器共用本实现。GFM 表格由含非空单元的表头、紧随其后的
//! 对齐分隔行及正文组成；`FenceTracker` 区分围栏外、Markdown 围栏和其他代码围栏，
//! 避免把代码中的管道符当作表格。单行函数不保存跨行状态，连续行配对由调用方负责。
//! Canonical pipe-table structure detection and fenced-code-block tracking for
//! raw markdown source.
//!
//! Both the streaming controller (`streaming/controller.rs`) and the
//! markdown-fence unwrapper (`markdown.rs`) need to identify pipe-table
//! structure and fenced code blocks in raw markdown source.  This module
//! provides the canonical implementations so fixes only need to happen in one
//! place.
//!
//! ## Concepts
//!
//! A GFM pipe table is a sequence of lines where:
//! - A **header line** contains pipe-separated segments with at least one
//!   non-empty cell.
//! - A **delimiter line** immediately follows the header and contains only
//!   alignment markers (`---`, `:---`, `---:`, `:---:`), each with at least
//!   three dashes.
//! - **Body rows** follow the delimiter.
//!
//! A **fenced code block** starts with 3+ backticks or tildes and ends with a
//! matching close marker.  [`FenceTracker`] classifies each line as
//! [`FenceKind::Outside`], [`FenceKind::Markdown`], or [`FenceKind::Other`]
//! so callers can skip pipe characters that appear inside non-markdown fences.
//!
//! The table functions operate on single lines and do not maintain cross-line
//! state.  Callers (the streaming controller and fence unwrapper) are
//! responsible for pairing consecutive lines to confirm a table.

/// 将管道分隔行拆成去除首尾空白的片段；空行或无未转义分隔符时返回 `None`。
/// 这是结构解析器，会保留转义管道，因为调用方只判断是否可参与表格。
/// Split a pipe-delimited line into trimmed segments.
///
/// Returns `None` if the line is empty or has no unescaped separator marker.
/// Leading/trailing pipes are stripped before splitting.
///
/// This is intentionally a structural parser, not a renderer. It preserves
/// escaped pipes inside the returned segments because callers only care about
/// whether the line can participate in a table, not how the cell text should
/// finally be displayed.
pub(crate) fn parse_table_segments(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let has_outer_pipe = trimmed.starts_with('|') || trimmed.ends_with('|');
    let content = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let content = content.strip_suffix('|').unwrap_or(content);
    let raw_segments = split_unescaped_pipe(content);
    if !has_outer_pipe && raw_segments.len() <= 1 {
        return None;
    }

    let segments: Vec<&str> = raw_segments.into_iter().map(str::trim).collect();
    (!segments.is_empty()).then_some(segments)
}

/// 按未转义的 `|` 拆分内容；前有反斜杠的管道是字面文本，反斜杠仍留在片段中。
/// Split `content` on unescaped `|` characters.
///
/// A pipe preceded by `\` is treated as literal text, not a column separator.
/// The backslash remains in the segment (this is structure detection, not
/// rendering).
fn split_unescaped_pipe(content: &str) -> Vec<&str> {
    let mut segments = Vec::with_capacity(8);
    let mut start = 0;
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // 跳过已转义字符，使其后的管道不被当作分隔符。
            // Skip the escaped character.
            i += 2;
        } else if bytes[i] == b'|' {
            segments.push(&content[start..i]);
            start = i + 1;
            i += 1;
        } else {
            i += 1;
        }
    }
    segments.push(&content[start..]);
    segments
}

// 小型表格检测函数内联到流式热路径，因为增量保留扫描会对每一源码行调用它们。
// Small table-detection helpers inlined for the streaming hot path — they are
// called on every source line during incremental holdback scanning.

/// 判断行是否像表头，即含管道分隔片段且至少一个单元非空。
/// Whether `line` looks like a table header row (has pipe-separated
/// segments with at least one non-empty cell).
#[inline]
pub(crate) fn is_table_header_line(line: &str) -> bool {
    parse_table_segments(line).is_some_and(|segments| segments.iter().any(|s| !s.is_empty()))
}

/// 判断单个片段是否匹配 Markdown 分隔行的对齐冒号语法。
/// Whether a single segment matches the `---`, `:---`, `---:`, or `:---:`
/// alignment-colon syntax used in markdown table delimiter rows.
#[inline]
fn is_table_delimiter_segment(segment: &str) -> bool {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return false;
    }
    let without_leading = trimmed.strip_prefix(':').unwrap_or(trimmed);
    let without_ends = without_leading.strip_suffix(':').unwrap_or(without_leading);
    without_ends.len() >= 3 && without_ends.chars().all(|c| c == '-')
}

/// 判断整行是否为有效表格分隔行，即每个片段都通过分隔片段校验。
/// Whether `line` is a valid table delimiter row (every segment passes
/// [`is_table_delimiter_segment`]).
#[inline]
pub(crate) fn is_table_delimiter_line(line: &str) -> bool {
    parse_table_segments(line)
        .is_some_and(|segments| segments.into_iter().all(is_table_delimiter_segment))
}

// 以下部分负责跟踪围栏代码块。
// ---------------------------------------------------------------------------
// Fenced code block tracking
// ---------------------------------------------------------------------------

/// 源码行相对围栏代码块的位置；表格保留只作用于围栏外或 Markdown 围栏，
/// 其他代码围栏内的管道符属于代码而非表格语法。
/// Where a source line sits relative to fenced code blocks.
///
/// Table holdback only applies to lines that are `Outside` or inside a
/// `Markdown` fence. Lines inside `Other` fences (e.g. `sh`, `rust`) are
/// ignored by the table scanner because their pipe characters are code, not
/// table syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FenceKind {
    /// 不在任何围栏代码块内。
    /// Not inside any fenced code block.
    Outside,
    /// 位于 `md` 或 `markdown` 围栏内。
    /// Inside a `` ```md `` or `` ```markdown `` fence.
    Markdown,
    /// 位于信息字符串不是 Markdown 的围栏内。
    /// Inside a fence with a non-markdown info string.
    Other,
}

/// 围栏代码块开闭转换的增量跟踪器；处理三空格限制、块引用前缀及反引号/波浪线匹配，
/// `kind` 报告当前行改变状态前适用的上下文。
/// Incremental tracker for fenced-code-block open/close transitions.
///
/// Feed lines one at a time via [`advance`](Self::advance); query the current
/// context with [`kind`](Self::kind).  The tracker handles leading-whitespace
/// limits (>3 spaces → not a fence), blockquote prefix stripping, and
/// backtick/tilde marker matching.
///
/// The tracker reports the fence context that applies to the current line
/// before that line mutates the state. Callers rely on that when deciding
/// whether the current raw line can open or continue a table.
pub(crate) struct FenceTracker {
    state: Option<(char, usize, FenceKind)>,
}

impl FenceTracker {
    #[inline]
    pub(crate) fn new() -> Self {
        Self { state: None }
    }

    /// 处理一条原始源码行并更新围栏状态；超过三空格缩进的行不是围栏，块引用前缀先移除。
    /// Process one raw source line and update fence state.
    ///
    /// Lines with >3 leading spaces are ignored (indented code blocks, not
    /// fences).  Blockquote prefixes (`>`) are stripped before scanning.
    pub(crate) fn advance(&mut self, raw_line: &str) {
        let leading_spaces = raw_line.as_bytes().iter().take_while(|byte| **byte == b' ').count();
        if leading_spaces > 3 {
            return;
        }

        let trimmed = &raw_line[leading_spaces..];
        let fence_scan_text = strip_blockquote_prefix(trimmed);
        if let Some((marker, len)) = parse_fence_marker(fence_scan_text) {
            if let Some((open_char, open_len, _)) = self.state {
                // 标记字符及长度匹配时关闭当前围栏。
                // Close the current fence if the marker matches.
                if marker == open_char
                    && len >= open_len
                    && fence_scan_text[len..].trim().is_empty()
                {
                    self.state = None;
                }
            } else {
                // 打开新围栏并按信息字符串分类。
                // Opening a new fence.
                let kind = if is_markdown_fence_info(fence_scan_text, len) {
                    FenceKind::Markdown
                } else {
                    FenceKind::Other
                };
                self.state = Some((marker, len, kind));
            }
        }
    }

    /// 最近处理行所适用的当前围栏上下文。
    /// Current fence context for the most-recently-advanced line.
    #[inline]
    pub(crate) fn kind(&self) -> FenceKind {
        self.state.map_or(FenceKind::Outside, |(_, _, k)| k)
    }
}

/// 返回潜在围栏行的标记字符和连续长度；只识别至少三个反引号或波浪线。
/// Return fence marker character and run length for a potential fence line.
///
/// Recognises backtick and tilde fences with a minimum run of 3.
/// The input should already have leading whitespace and blockquote prefixes
/// stripped.
#[inline]
pub(crate) fn parse_fence_marker(line: &str) -> Option<(char, usize)> {
    let first = line.as_bytes().first().copied()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let len = line.bytes().take_while(|&b| b == first).count();
    if len < 3 {
        return None;
    }
    Some((first as char, len))
}

/// 判断围栏标记后的信息字符串是否为大小写不敏感的 `md` 或 `markdown`。
/// Whether the info string after a fence marker indicates markdown content.
///
/// Matches `md` and `markdown` (case-insensitive).
#[inline]
pub(crate) fn is_markdown_fence_info(trimmed_line: &str, marker_len: usize) -> bool {
    let info = trimmed_line[marker_len..].split_whitespace().next().unwrap_or_default();
    info.eq_ignore_ascii_case("md") || info.eq_ignore_ascii_case("markdown")
}

/// 移除一行所有前导 `>` 块引用标记，使块引用中的表格也能被结构扫描识别。
/// Peel all leading `>` blockquote markers from a line.
///
/// Tables can appear inside blockquotes (`> | A | B |`), so the holdback
/// scanner must strip these markers before checking for table syntax.
#[inline]
pub(crate) fn strip_blockquote_prefix(line: &str) -> &str {
    let mut rest = line.trim_start();
    loop {
        let Some(stripped) = rest.strip_prefix('>') else {
            return rest;
        };
        rest = stripped.strip_prefix(' ').unwrap_or(stripped).trim_start();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_table_segments_basic() {
        assert_eq!(parse_table_segments("| A | B | C |"), Some(vec!["A", "B", "C"]));
    }

    #[test]
    fn parse_table_segments_no_outer_pipes() {
        assert_eq!(parse_table_segments("A | B | C"), Some(vec!["A", "B", "C"]));
    }

    #[test]
    fn parse_table_segments_no_leading_pipe() {
        assert_eq!(parse_table_segments("A | B | C |"), Some(vec!["A", "B", "C"]));
    }

    #[test]
    fn parse_table_segments_no_trailing_pipe() {
        assert_eq!(parse_table_segments("| A | B | C"), Some(vec!["A", "B", "C"]));
    }

    #[test]
    fn parse_table_segments_single_segment_is_allowed() {
        assert_eq!(parse_table_segments("| only |"), Some(vec!["only"]));
    }

    #[test]
    fn parse_table_segments_without_pipe_returns_none() {
        assert_eq!(parse_table_segments("just text"), None);
    }

    #[test]
    fn parse_table_segments_empty_returns_none() {
        assert_eq!(parse_table_segments(""), None);
        assert_eq!(parse_table_segments("   "), None);
    }

    #[test]
    fn parse_table_segments_escaped_pipe() {
        // 转义管道不得拆分，必须留在同一片段内。
        // Escaped pipe should NOT split — stays inside the segment.
        assert_eq!(parse_table_segments(r"| A \| B | C |"), Some(vec![r"A \| B", "C"]));
    }

    #[test]
    fn is_table_delimiter_segment_valid() {
        assert!(is_table_delimiter_segment("---"));
        assert!(is_table_delimiter_segment(":---"));
        assert!(is_table_delimiter_segment("---:"));
        assert!(is_table_delimiter_segment(":---:"));
        assert!(is_table_delimiter_segment(":-------:"));
    }

    #[test]
    fn is_table_delimiter_segment_invalid() {
        assert!(!is_table_delimiter_segment(""));
        assert!(!is_table_delimiter_segment("--"));
        assert!(!is_table_delimiter_segment("abc"));
        assert!(!is_table_delimiter_segment(":--"));
    }

    #[test]
    fn is_table_delimiter_line_valid() {
        assert!(is_table_delimiter_line("| --- | --- |"));
        assert!(is_table_delimiter_line("|:---:|---:|"));
        assert!(is_table_delimiter_line("--- | --- | ---"));
    }

    #[test]
    fn is_table_delimiter_line_invalid() {
        assert!(!is_table_delimiter_line("| A | B |"));
        assert!(!is_table_delimiter_line("| -- | -- |"));
    }

    #[test]
    fn is_table_header_line_valid() {
        assert!(is_table_header_line("| A | B |"));
        assert!(is_table_header_line("Name | Value"));
    }

    #[test]
    fn is_table_header_line_all_empty_segments() {
        assert!(!is_table_header_line("| | |"));
    }

    // 以下测试覆盖 `FenceTracker` 状态转换。
    // -----------------------------------------------------------------------
    // FenceTracker tests
    // -----------------------------------------------------------------------

    #[test]
    fn fence_tracker_outside_by_default() {
        let tracker = FenceTracker::new();
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_opens_and_closes_backtick_fence() {
        let mut tracker = FenceTracker::new();
        tracker.advance("```rust");
        assert_eq!(tracker.kind(), FenceKind::Other);

        tracker.advance("let x = 1;");
        assert_eq!(tracker.kind(), FenceKind::Other);

        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_opens_and_closes_tilde_fence() {
        let mut tracker = FenceTracker::new();
        tracker.advance("~~~python");
        assert_eq!(tracker.kind(), FenceKind::Other);
        tracker.advance("~~~");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_markdown_fence() {
        let mut tracker = FenceTracker::new();
        tracker.advance("```md");
        assert_eq!(tracker.kind(), FenceKind::Markdown);
        tracker.advance("| A | B |");
        assert_eq!(tracker.kind(), FenceKind::Markdown);
        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_markdown_case_insensitive() {
        let mut tracker = FenceTracker::new();
        tracker.advance("```Markdown");
        assert_eq!(tracker.kind(), FenceKind::Markdown);
        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_nested_shorter_marker_does_not_close() {
        let mut tracker = FenceTracker::new();
        tracker.advance("````sh");
        assert_eq!(tracker.kind(), FenceKind::Other);
        // 围栏内更短的标记不能关闭围栏。
        // Shorter marker inside should not close.
        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Other);
        // 长度匹配的标记应关闭围栏。
        // Matching length closes.
        tracker.advance("````");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_mismatched_char_does_not_close() {
        let mut tracker = FenceTracker::new();
        tracker.advance("```sh");
        assert_eq!(tracker.kind(), FenceKind::Other);
        // 波浪线标记不能关闭反引号围栏。
        // Tilde marker should not close a backtick fence.
        tracker.advance("~~~");
        assert_eq!(tracker.kind(), FenceKind::Other);
        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_indented_4_spaces_ignored() {
        let mut tracker = FenceTracker::new();
        tracker.advance("    ```sh");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_blockquote_prefix_stripped() {
        let mut tracker = FenceTracker::new();
        tracker.advance("> ```sh");
        assert_eq!(tracker.kind(), FenceKind::Other);
        tracker.advance("> ```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    #[test]
    fn fence_tracker_close_with_trailing_content_does_not_close() {
        let mut tracker = FenceTracker::new();
        tracker.advance("```sh");
        assert_eq!(tracker.kind(), FenceKind::Other);
        // 关闭标记后的额外内容会阻止关闭。
        // Trailing content prevents closing.
        tracker.advance("``` extra");
        assert_eq!(tracker.kind(), FenceKind::Other);
        tracker.advance("```");
        assert_eq!(tracker.kind(), FenceKind::Outside);
    }

    // 以下测试覆盖围栏辅助函数。
    // -----------------------------------------------------------------------
    // Fence helper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_fence_marker_backtick() {
        assert_eq!(parse_fence_marker("```rust"), Some(('`', 3)));
        assert_eq!(parse_fence_marker("````"), Some(('`', 4)));
    }

    #[test]
    fn parse_fence_marker_tilde() {
        assert_eq!(parse_fence_marker("~~~python"), Some(('~', 3)));
    }

    #[test]
    fn parse_fence_marker_too_short() {
        assert_eq!(parse_fence_marker("``"), None);
        assert_eq!(parse_fence_marker("~~"), None);
    }

    #[test]
    fn parse_fence_marker_not_fence() {
        assert_eq!(parse_fence_marker("hello"), None);
        assert_eq!(parse_fence_marker(""), None);
    }

    #[test]
    fn is_markdown_fence_info_basic() {
        assert!(is_markdown_fence_info("```md", /*marker_len*/ 3));
        assert!(is_markdown_fence_info("```markdown", /*marker_len*/ 3));
        assert!(is_markdown_fence_info("```MD", /*marker_len*/ 3));
        assert!(!is_markdown_fence_info("```rust", /*marker_len*/ 3));
        assert!(!is_markdown_fence_info("```", /*marker_len*/ 3));
    }

    #[test]
    fn strip_blockquote_prefix_basic() {
        assert_eq!(strip_blockquote_prefix("> hello"), "hello");
        assert_eq!(strip_blockquote_prefix("> > nested"), "nested");
        assert_eq!(strip_blockquote_prefix("no prefix"), "no prefix");
    }
}
