//! `/status` 卡片共享的令牌、字段对齐与宽度截断工具。
//! 所有宽度计算均使用终端显示宽度，避免多码点字素被拆开。

use ratatui::prelude::*;
use ratatui::style::Stylize;
use std::collections::BTreeSet;
use unicode_segmentation::UnicodeSegmentation;

use crate::width::display_width;

/// 将令牌数压缩为状态卡片使用的 `K` 计数形式。
pub(super) fn format_tokens_compact(tokens: u64) -> String {
    if tokens < 1_000 {
        return tokens.to_string();
    }
    let whole = tokens / 1_000;
    let remainder = (tokens % 1_000) / 100;
    if remainder == 0 { format!("{whole}K") } else { format!("{whole}.{remainder}K") }
}

/// 根据全部标签的最大显示宽度生成稳定的字段列布局。
#[derive(Debug, Clone)]
pub(crate) struct FieldFormatter {
    indent: &'static str,
    label_width: usize,
    value_offset: usize,
    value_indent: String,
}

impl FieldFormatter {
    /// 状态卡片字段统一使用的左缩进。
    pub(crate) const INDENT: &'static str = " ";

    /// 预先计算标签列及续行缩进，空标签集合也可安全使用。
    pub(crate) fn from_labels<S>(labels: impl IntoIterator<Item = S>) -> Self
    where
        S: AsRef<str>,
    {
        let label_width =
            labels.into_iter().map(|label| display_width(label.as_ref())).max().unwrap_or(0);
        let indent_width = display_width(Self::INDENT);
        let value_offset = indent_width + label_width + 1 + 3;

        Self {
            indent: Self::INDENT,
            label_width,
            value_offset,
            value_indent: " ".repeat(value_offset),
        }
    }

    /// 生成一行带对齐标签的字段内容。
    pub(crate) fn line(
        &self,
        label: &'static str,
        value_spans: Vec<Span<'static>>,
    ) -> Line<'static> {
        Line::from(self.full_spans(label, value_spans))
    }

    /// 生成与上一字段值列对齐的续行。
    pub(crate) fn continuation(&self, mut spans: Vec<Span<'static>>) -> Line<'static> {
        let mut all_spans = Vec::with_capacity(spans.len() + 1);
        all_spans.push(Span::from(self.value_indent.clone()).dim());
        all_spans.append(&mut spans);
        Line::from(all_spans)
    }

    /// 返回给字段值保留的可用显示宽度。
    pub(crate) fn value_width(&self, available_inner_width: usize) -> usize {
        available_inner_width.saturating_sub(self.value_offset)
    }

    /// 拼接标签 span 与值 spans，保留每个值原有样式。
    pub(crate) fn full_spans(
        &self,
        label: &str,
        mut value_spans: Vec<Span<'static>>,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::with_capacity(value_spans.len() + 1);
        spans.push(self.label_span(label));
        spans.append(&mut value_spans);
        spans
    }

    fn label_span(&self, label: &str) -> Span<'static> {
        let mut buf = String::with_capacity(self.value_offset);
        buf.push_str(self.indent);

        buf.push_str(label);
        buf.push(':');

        let label_width = display_width(label);
        let padding = 3 + self.label_width.saturating_sub(label_width);
        for _ in 0..padding {
            buf.push(' ');
        }

        Span::from(buf).dim()
    }
}

/// 按首次出现顺序收集唯一标签。
pub(crate) fn push_label(labels: &mut Vec<String>, seen: &mut BTreeSet<String>, label: &str) {
    if seen.contains(label) {
        return;
    }

    let owned = label.to_string();
    seen.insert(owned.clone());
    labels.push(owned);
}

/// 按完整字素簇截断一行，同时保留已纳入内容的样式。
pub(crate) fn truncate_line_to_width(line: Line<'static>, max_width: usize) -> Line<'static> {
    if max_width == 0 {
        return Line::from(Vec::<Span<'static>>::new());
    }

    let mut used = 0usize;
    let mut spans_out: Vec<Span<'static>> = Vec::new();

    for span in line.spans {
        let text = span.content.into_owned();
        let style = span.style;
        let span_width = display_width(text.as_str());

        if span_width == 0 {
            // 零宽 span 可能携带样式边界，应原样保留且不占宽度预算。
            spans_out.push(Span::styled(text, style));
            continue;
        }

        if used >= max_width {
            break;
        }

        if used + span_width <= max_width {
            used += span_width;
            spans_out.push(Span::styled(text, style));
            continue;
        }

        // 截断只能发生在字素簇边界，避免拆散组合字符或全角字符。
        let mut truncated = String::new();
        for grapheme in text.graphemes(/*is_extended*/ true) {
            let grapheme_width = display_width(grapheme);
            if used + grapheme_width > max_width {
                break;
            }
            truncated.push_str(grapheme);
            used += grapheme_width;
        }

        if !truncated.is_empty() {
            spans_out.push(Span::styled(truncated, style));
        }

        break;
    }

    Line::from(spans_out)
}
