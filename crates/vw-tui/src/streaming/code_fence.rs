//! 针对单个开放、顶层且带语言标签代码围栏的保守快速路径。
//!
//! 只有源码与标准解析器代码文本完全一致时才启用；任何可能闭合围栏的行都回退到标准路径。
//! 快速状态只保留受限语言标记和源码锚点，从不复制不断增长的流式代码正文。
//! A conservative fast path for one open, top-level, language-tagged code fence.

use crate::render::highlight::MAX_HIGHLIGHT_LINE_BYTES;
use crate::render::highlight::StreamingCodeHighlighter;
use crate::render::highlight::syntax_theme_revision;
use crate::terminal_hyperlinks::HyperlinkLine;
use ratatui::text::Span;

/// 原始源码与 pulldown-cmark 代码文本一致的只追加围栏。
/// An append-only fence whose original source is identical to pulldown-cmark's code text.
pub(super) struct OpenCodeFence {
    marker: u8,
    marker_len: usize,
    language: String,
    content_start: usize,
    source_len: usize,
    theme_revision: u64,
    /// 仅在下一分块到达时初始化，避免对一次性围栏重复工作。
    /// Initialized only if another chunk arrives, avoiding duplicate work on one-shot fences.
    highlighter: Option<StreamingCodeHighlighter>,
}

impl OpenCodeFence {
    /// 在标准渲染完成后识别最终可变顶层块。
    ///
    /// `source` 是最终块后缀，`source_len` 包含完整已提交 Markdown 源码并为围栏提供锚点。
    /// 缩进、CR/NUL 规范化和转义信息串继续走标准路径；可能结束围栏的行也全部回退。
    /// Recognize the final mutable top-level block after its canonical render.
    ///
    /// `source` is the final block's suffix, while `source_len` includes the entire committed
    /// Markdown source and anchors the retained fence within it.
    ///
    /// Indentation, CR/NUL normalization, and escaped info strings stay on the canonical path.
    /// Any line that could end the fence also stays there, including deliberately conservative
    /// false positives, so this fast path need not duplicate CommonMark's closing-fence grammar.
    pub(super) fn detect(source: &str, source_len: usize, theme_revision: u64) -> Option<Self> {
        let marker = *source.as_bytes().first()?;
        if marker != b'`' && marker != b'~' {
            return None;
        }
        let (opening, code) = source.split_once('\n')?;
        let marker_len = opening.bytes().take_while(|byte| *byte == marker).count();
        if marker_len < 3
            || !source.ends_with('\n')
            || theme_revision != syntax_theme_revision()
            || source.contains(['\r', '\0'])
        {
            return None;
        }
        let info = opening[marker_len..].trim_matches([' ', '\t', '\u{b}', '\u{c}']);
        if info.contains(['&', '\\']) || (marker == b'`' && info.contains('`')) {
            return None;
        }
        let language =
            info.split([',', ' ', '\t']).next().filter(|language| !language.is_empty())?;
        // 最多保留一个受限语言标记，绝不保留流式代码正文。
        // Retain at most one bounded language token, never the streamed code itself.
        if language.len() > MAX_HIGHLIGHT_LINE_BYTES
            || has_possible_closing_line(code, marker, marker_len)
        {
            return None;
        }
        Some(Self {
            marker,
            marker_len,
            language: language.to_string(),
            content_start: source_len.checked_sub(code.len())?,
            source_len,
            theme_revision,
            highlighter: None,
        })
    }

    /// 仅当调用者按换行提交的源码继续扩展同一围栏时追加代码。
    ///
    /// 返回 `None` 会消费已保留状态，并要求重新执行完整标准围栏渲染。
    /// 成功行包含标准 Markdown 写入器为顶层围栏生成的空缩进 span。
    /// Append code only while the caller's newline-committed source extends this same fence.
    ///
    /// Returning `None` consumes the retained state and requires a canonical whole-fence render.
    /// Successful lines include the empty indent span expected by the canonical Markdown writer.
    pub(super) fn append(
        mut self,
        raw_source: &str,
        committed_source: &str,
    ) -> Option<(Self, Vec<HyperlinkLine>)> {
        if self.source_len.checked_add(committed_source.len()) != Some(raw_source.len())
            || !raw_source.ends_with(committed_source)
            || committed_source.contains(['\r', '\0'])
            || has_possible_closing_line(committed_source, self.marker, self.marker_len)
            || self.theme_revision != syntax_theme_revision()
        {
            return None;
        }
        let highlighter = match self.highlighter.take() {
            Some(highlighter) => highlighter,
            None => StreamingCodeHighlighter::new(
                &raw_source[self.content_start..self.source_len],
                &self.language,
                self.theme_revision,
            )?,
        };
        let (highlighter, lines) = highlighter.append(committed_source)?;
        self.highlighter = Some(highlighter);
        self.source_len = raw_source.len();
        let lines = lines
            .into_iter()
            .map(|mut line| {
                // 标准写入器会为顶层围栏安装一个空缩进 span。
                // The canonical writer installs an empty indent span for top-level fences.
                line.spans.insert(/*index*/ 0, Span::default());
                HyperlinkLine::new(line)
            })
            .collect();
        Some((self, lines))
    }
}

/// 保守检测任何可能闭合围栏的行。
///
/// 误报有意回退到 CommonMark，而不在此复制其结束围栏规则。
/// Conservatively detect any line that might close the fence.
///
/// False positives intentionally fall back to CommonMark instead of duplicating its closer rules.
fn has_possible_closing_line(source: &str, marker: u8, marker_len: usize) -> bool {
    source.lines().any(|line| {
        line.trim_start_matches([' ', '\t']).bytes().take_while(|byte| *byte == marker).count()
            >= marker_len
    })
}

#[cfg(test)]
#[path = "code_fence_tests.rs"]
mod tests;
