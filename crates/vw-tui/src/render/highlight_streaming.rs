//! Append-only syntax highlighting for complete lines in an open code fence.
//! 对尚未闭合代码围栏中的完整行执行仅追加语法高亮。
//!
//! 状态只保存 syntect 解析进度与主题修订号；语言未知、输入超限或重放失败时，
//! 调用方必须放弃增量状态并用规范渲染器重绘整个围栏。

use super::MAX_HIGHLIGHT_LINE_BYTES;
use super::exceeds_highlight_limits;
use super::find_syntax;
use super::highlighted_line_spans;
use super::syntax_set;
use super::syntax_theme_revision;
use super::theme_lock;
use ratatui::text::Line;
use syntect::easy::HighlightLines;
use syntect::highlighting::HighlightState;
use syntect::parsing::ParseState;
use syntect::util::LinesWithEndings;

/// Retains only Syntect's parser state, subject to the ordinary highlighting limits.
///
/// An absent state means the whole fence already uses the permanent plain-text fallback because
/// its language is unknown or a size limit was exceeded. The caller owns the rendered prefix.
/// 只保留 syntect 解析状态并遵守普通高亮护栏；状态缺失表示围栏已永久退回纯文本，已渲染前缀由调用方持有。
pub(crate) struct StreamingCodeHighlighter {
    state: Option<HighlightedCode>,
}

struct HighlightedCode {
    bytes: usize,
    lines: usize,
    theme_revision: u64,
    syntax: (HighlightState, ParseState),
}

impl StreamingCodeHighlighter {
    /// Reconstruct highlighting state after the canonical renderer emitted `code`.
    ///
    /// Unknown languages and oversized input retain a permanent plain-text fallback. A changed
    /// theme or failed replay returns `None`, requiring the caller to render the whole fence again.
    /// 根据规范渲染器已经输出的代码重建解析状态；主题变化、未知语言、超限或重放失败均返回 `None`。
    pub(crate) fn new(code: &str, lang: &str, theme_revision: u64) -> Option<Self> {
        let theme = match theme_lock().read() {
            Ok(theme) => theme,
            Err(poisoned) => poisoned.into_inner(),
        };
        if theme_revision != syntax_theme_revision() {
            return None;
        }
        let lines = code.lines().count();
        let syntax = find_syntax(lang).filter(|_| {
            !exceeds_highlight_limits(code.len(), lines)
                && !code.lines().any(|line| line.len() > MAX_HIGHLIGHT_LINE_BYTES)
        });
        let Some(syntax) = syntax else {
            return Some(Self { state: None });
        };
        let mut highlighter = HighlightLines::new(syntax, &theme);
        for line in LinesWithEndings::from(code) {
            highlighter.highlight_line(line, syntax_set()).ok()?;
        }
        Some(Self {
            state: Some(HighlightedCode {
                bytes: code.len(),
                lines,
                theme_revision,
                syntax: highlighter.state(),
            }),
        })
    }

    /// Consume the previous state and highlight just the newly committed complete lines.
    ///
    /// Every appended line must be newline-terminated to preserve Syntect's parser state.
    /// Returning `None` discards the state: the caller must render the whole fence again, which
    /// also removes old colors when the aggregate input crosses a highlighting limit.
    /// 消费旧状态并只高亮新提交的完整行；追加行必须以换行结尾才能维持解析状态。
    /// 返回 `None` 时调用方重绘整个围栏，也会在总输入越界时清除旧颜色。
    pub(crate) fn append(mut self, appended: &str) -> Option<(Self, Vec<Line<'static>>)> {
        if !appended.ends_with('\n') {
            return None;
        }
        let Some(mut state) = self.state.take() else {
            let lines = appended.lines().map(|line| Line::from(line.to_string())).collect();
            return Some((self, lines));
        };
        let bytes = state.bytes.checked_add(appended.len())?;
        let lines = state.lines.checked_add(appended.lines().count())?;
        if exceeds_highlight_limits(bytes, lines)
            || appended.lines().any(|line| line.len() > MAX_HIGHLIGHT_LINE_BYTES)
        {
            return None;
        }
        let theme = match theme_lock().read() {
            Ok(theme) => theme,
            Err(poisoned) => poisoned.into_inner(),
        };
        if state.theme_revision != syntax_theme_revision() {
            return None;
        }
        let (highlight_state, parse_state) = state.syntax;
        let mut highlighter = HighlightLines::from_state(&theme, highlight_state, parse_state);
        let mut rendered = Vec::with_capacity(lines - state.lines);
        for line in LinesWithEndings::from(appended) {
            let ranges = highlighter.highlight_line(line, syntax_set()).ok()?;
            rendered.push(Line::from(highlighted_line_spans(ranges)));
        }
        state.bytes = bytes;
        state.lines = lines;
        state.syntax = highlighter.state();
        self.state = Some(state);
        Some((self, rendered))
    }
}

#[cfg(test)]
#[path = "highlight_streaming_tests.rs"]
mod tests;
