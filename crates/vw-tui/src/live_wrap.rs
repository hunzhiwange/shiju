//! 将增量输入按终端显示宽度包装为稳定的可视行。

use crate::width::display_width;
use unicode_segmentation::UnicodeSegmentation;

/// `RowBuilder` 产生的一条可视行。
/// A single visual row produced by RowBuilder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    /// 此行以显式换行而非宽度硬换行结束时为真。
    /// True if this row ends with an explicit line break (as opposed to a hard wrap).
    pub explicit_break: bool,
}

impl Row {
    pub fn width(&self) -> usize {
        display_width(&self.text)
    }
}

/// 增量地将输入文本包装为最多 `width` 个单元格的可视行；当前第一阶段只处理纯文本，
/// ANSI 状态延续和带样式 span 留待后续。
/// Incrementally wraps input text into visual rows of at most `width` cells.
///
/// Step 1: plain-text only. ANSI-carry and styled spans will be added later.
pub struct RowBuilder {
    target_width: usize,
    /// 当前逻辑行的缓冲区，直到读到 `\n`。
    /// Buffer for the current logical line (until a '\n' is seen).
    current_line: String,
    /// 当前及先前逻辑行已构建的输出行。
    /// Output rows built so far for the current logical line and previous ones.
    rows: Vec<Row>,
}

impl RowBuilder {
    pub fn new(target_width: usize) -> Self {
        Self { target_width: target_width.max(1), current_line: String::new(), rows: Vec::new() }
    }

    pub fn set_width(&mut self, width: usize) {
        self.target_width = width.max(1);
        // 第一阶段采用简单策略：宽度变化时重新包装全部已有文本。
        // Rewrap everything we have (simple approach for Step 1).
        let mut all = String::new();
        for row in self.rows.drain(..) {
            all.push_str(&row.text);
            if row.explicit_break {
                all.push('\n');
            }
        }
        all.push_str(&self.current_line);
        self.current_line.clear();
        self.push_fragment(&all);
    }

    /// 推入可能包含换行的输入片段。
    /// Push an input fragment. May contain newlines.
    pub fn push_fragment(&mut self, fragment: &str) {
        if fragment.is_empty() {
            return;
        }
        let mut start = 0usize;
        for (i, ch) in fragment.char_indices() {
            if ch == '\n' {
                // 遇到换行前先刷新此前待处理内容。
                // Flush anything pending before the newline.
                if start < i {
                    self.current_line.push_str(&fragment[start..i]);
                }
                self.flush_current_line(/*explicit_break*/ true);
                start = i + ch.len_utf8();
            }
        }
        if start < fragment.len() {
            self.current_line.push_str(&fragment[start..]);
            self.wrap_current_line();
        }
    }

    /// 返回已产生行的非消耗式快照。
    /// Return a snapshot of produced rows (non-draining).
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// 返回适合展示的行，包含可能存在的当前未完整行。
    /// Rows suitable for display, including the current partial line if any.
    pub fn display_rows(&self) -> Vec<Row> {
        let mut out = self.rows.clone();
        if !self.current_line.is_empty() {
            out.push(Row { text: self.current_line.clone(), explicit_break: false });
        }
        out
    }

    /// 排出超过 `max_keep` 展示行的最旧行，计入当前部分行，并按原顺序返回。
    /// Drain the oldest rows that exceed `max_keep` display rows (including the
    /// current partial line, if any). Returns the drained rows in order.
    pub fn drain_commit_ready(&mut self, max_keep: usize) -> Vec<Row> {
        let display_count = self.rows.len() + if self.current_line.is_empty() { 0 } else { 1 };
        if display_count <= max_keep {
            return Vec::new();
        }
        let to_commit = display_count - max_keep;
        let commit_count = to_commit.min(self.rows.len());
        let mut drained = Vec::with_capacity(commit_count);
        for _ in 0..commit_count {
            drained.push(self.rows.remove(0));
        }
        drained
    }

    fn flush_current_line(&mut self, explicit_break: bool) {
        // 先包装当前行剩余内容，再以 `explicit_break` 完成该逻辑行。
        // Wrap any remaining content in the current line and then finalize with explicit_break.
        self.wrap_current_line();
        // 当前行恰在宽度边界结束且非空时，用空的显式行表示换行，以保持片段化不变性。
        // If the current line ended exactly on a width boundary and is non-empty, represent
        // the explicit break as an empty explicit row so that fragmentation invariance holds.
        if explicit_break {
            if self.current_line.is_empty() {
                // 前一行恰好在边界结束，因此添加空的显式换行行。
                // We ended on a boundary previously; add an empty explicit row.
                self.rows.push(Row { text: String::new(), explicit_break: true });
            } else {
                // 仍有尚未包装的内容，立即以显式换行标记推入。
                // There is leftover content that did not wrap yet; push it now with the explicit flag.
                let mut s = String::new();
                std::mem::swap(&mut s, &mut self.current_line);
                self.rows.push(Row { text: s, explicit_break: true });
            }
        }
        // 为下一逻辑行清空当前行缓冲区。
        // Reset current line buffer for next logical line.
        self.current_line.clear();
    }

    fn wrap_current_line(&mut self) {
        // 当前行超过宽度时循环截取可容纳前缀。
        // While the current_line exceeds width, cut a prefix.
        loop {
            if self.current_line.is_empty() {
                break;
            }
            let (prefix, suffix, taken) =
                take_prefix_by_width(&self.current_line, self.target_width);
            if taken == 0 {
                // 不可分字素本身宽于目标时至少消费一个字素，避免无限循环。
                // Avoid an infinite loop on an indivisible grapheme wider than the target.
                if let Some(grapheme) = self.current_line.graphemes(/*is_extended*/ true).next() {
                    let len = grapheme.len();
                    let p = self.current_line[..len].to_string();
                    self.rows.push(Row { text: p, explicit_break: false });
                    self.current_line = self.current_line[len..].to_string();
                    continue;
                }
                break;
            }
            if suffix.is_empty() {
                // 整体可容纳时继续留在缓冲区，便于之后追加更多内容。
                // Fits entirely; keep in buffer (do not push yet) so we can append more later.
                break;
            } else {
                // 将包装前缀作为非显式行输出，再继续处理剩余内容。
                // Emit wrapped prefix as a non-explicit row and continue with the remainder.
                self.rows.push(Row { text: prefix, explicit_break: false });
                self.current_line = suffix.to_string();
            }
        }
    }
}

/// 取得可见宽度不超过 `max_cols` 的文本前缀，并返回前缀、后缀和前缀宽度。
/// Take a prefix of `text` whose visible width is at most `max_cols`.
/// Returns (prefix, suffix, prefix_width).
pub fn take_prefix_by_width(text: &str, max_cols: usize) -> (String, &str, usize) {
    if max_cols == 0 || text.is_empty() {
        return (String::new(), text, 0);
    }
    let mut cols = 0usize;
    let mut end_idx = 0usize;
    for (i, grapheme) in text.grapheme_indices(/*is_extended*/ true) {
        let grapheme_width = display_width(grapheme);
        if cols.saturating_add(grapheme_width) > max_cols {
            break;
        }
        cols += grapheme_width;
        end_idx = i + grapheme.len();
        if cols == max_cols {
            break;
        }
    }
    let prefix = text[..end_idx].to_string();
    let suffix = &text[end_idx..];
    (prefix, suffix, cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn rows_do_not_exceed_width_ascii() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("hello whirl this is a test");
        let rows = rb.rows().to_vec();
        assert_eq!(
            rows,
            vec![
                Row { text: "hello whir".to_string(), explicit_break: false },
                Row { text: "l this is ".to_string(), explicit_break: false }
            ]
        );
    }

    #[test]
    fn rows_do_not_exceed_width_emoji_cjk() {
        // 😀 is width 2; 你/好 are width 2.
        let mut rb = RowBuilder::new(/*target_width*/ 6);
        rb.push_fragment("😀😀 你好");
        let rows = rb.rows().to_vec();
        // 宽度 6 时首行能容纳两个表情和一个空格；首个中日韩字符需两列会溢出，因此留在缓冲区。
        // At width 6, we expect the first row to fit exactly two emojis and a space
        // (2 + 2 + 1 = 5) plus one more column for the first CJK char (2 would overflow),
        // so only the two emojis and the space fit; the rest remains buffered.
        assert_eq!(rows, vec![Row { text: "😀😀 ".to_string(), explicit_break: false }]);
    }

    #[test]
    fn halfwidth_sound_marks_stay_with_their_grapheme_when_wrapping() {
        assert_eq!(take_prefix_by_width("abｶﾞc", /*max_cols*/ 3), ("ab".to_string(), "ｶﾞc", 2));
        assert_eq!(take_prefix_by_width("abｶﾞc", /*max_cols*/ 4), ("abｶﾞ".to_string(), "c", 4));

        let mut row_builder = RowBuilder::new(/*target_width*/ 1);
        row_builder.push_fragment("ｶﾞx");
        assert_eq!(row_builder.rows()[0].text, "ｶﾞ");
    }

    #[test]
    fn fragmentation_invariance_long_token() {
        let s = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 26 chars
        let mut rb_all = RowBuilder::new(/*target_width*/ 7);
        rb_all.push_fragment(s);
        let all_rows = rb_all.rows().to_vec();

        let mut rb_chunks = RowBuilder::new(/*target_width*/ 7);
        for i in (0..s.len()).step_by(3) {
            let end = (i + 3).min(s.len());
            rb_chunks.push_fragment(&s[i..end]);
        }
        let chunk_rows = rb_chunks.rows().to_vec();

        assert_eq!(all_rows, chunk_rows);
    }

    #[test]
    fn newline_splits_rows() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("hello\nworld");
        let rows = rb.display_rows();
        assert!(rows.iter().any(|r| r.explicit_break));
        assert_eq!(rows[0].text, "hello");
        // 第二行应以 `world` 开头。
        // Second row should begin with 'world'
        assert!(rows.iter().any(|r| r.text.starts_with("world")));
    }

    #[test]
    fn rewrap_on_width_change() {
        let mut rb = RowBuilder::new(/*target_width*/ 10);
        rb.push_fragment("abcdefghijK");
        assert!(!rb.rows().is_empty());
        rb.set_width(/*width*/ 5);
        for r in rb.rows() {
            assert!(r.width() <= 5);
        }
    }
}
