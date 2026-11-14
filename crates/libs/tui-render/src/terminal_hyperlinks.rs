//! 将语义化终端超链接与可见 TUI 文本分离保存。
//!
//! 布局代码只测量和换行普通 Ratatui 行；到达终端缓冲区或回滚区 writer 时才应用
//! 超链接标注，确保 OSC 8 字节不会影响几何计算。
//! Semantic terminal hyperlinks carried separately from visible TUI text.
//!
//! Layout code measures and wraps ordinary ratatui lines. Hyperlink annotations are applied only
//! when text reaches a terminal buffer or scrollback writer so OSC 8 bytes never affect geometry.

use std::num::NonZeroU16;
use std::ops::Range;

use ratatui::buffer::Buffer;
use ratatui::buffer::CellDiffOption;
use ratatui::buffer::CellWidth;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use tui_text::line_truncation::line_width;
use url::Url;

use tui_text::width::char_width;
use tui_text::width::display_width;
use tui_text::wrapping::RtOptions;
use tui_text::wrapping::adaptive_wrap_line;

use crate::line_utils::line_to_borrowed;
use crate::line_utils::line_to_static;

#[derive(Clone, Debug, Eq, PartialEq)]
// 超链接范围使用显示列而非字节偏移，以正确覆盖宽字符。
pub struct TerminalHyperlink {
    pub columns: Range<usize>,
    pub destination: String,
    destination_kind: DestinationKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DestinationKind {
    Web,
    TrustedFile,
}

impl TerminalHyperlink {
    pub fn web(columns: Range<usize>, destination: String) -> Self {
        Self { columns, destination, destination_kind: DestinationKind::Web }
    }

    pub fn retarget_to_trusted_file(&mut self, destination: &Url) {
        // 文件 URL 不进入通用 Markdown 链接路径，只有生成的可视化链接可提升为此目标类型。
        // Keep file URLs out of the general Markdown link path. Only generated visualization links
        // are promoted to this destination kind.
        debug_assert_eq!(destination.scheme(), "file");
        self.destination = destination.to_string();
        self.destination_kind = DestinationKind::TrustedFile;
    }

    fn with_columns(&self, columns: Range<usize>) -> Self {
        Self {
            columns,
            destination: self.destination.clone(),
            destination_kind: self.destination_kind,
        }
    }

    fn terminal_destination(&self) -> Option<String> {
        match self.destination_kind {
            DestinationKind::Web => web_destination(&self.destination),
            DestinationKind::TrustedFile => trusted_file_destination(&self.destination),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
// 渲染行与其链接元数据成对移动，换行后可重新映射各段范围。
pub struct HyperlinkLine {
    pub line: Line<'static>,
    pub hyperlinks: Vec<TerminalHyperlink>,
}

impl HyperlinkLine {
    pub fn new(line: Line<'static>) -> Self {
        Self { line, hyperlinks: Vec::new() }
    }

    pub fn width(&self) -> usize {
        line_width(&self.line)
    }

    pub fn push_span(&mut self, span: Span<'static>, destination: Option<&str>) {
        let start = self.width();
        let end = start + display_width(span.content.as_ref());
        self.line.push_span(span);
        if end > start
            && let Some(destination) = destination.and_then(web_destination)
        {
            self.hyperlinks.push(TerminalHyperlink::web(start..end, destination));
        }
    }

    pub fn style(mut self, style: ratatui::style::Style) -> Self {
        self.line = self.line.style(style);
        self
    }
}

impl From<Line<'static>> for HyperlinkLine {
    fn from(line: Line<'static>) -> Self {
        Self::new(line)
    }
}

impl From<&'static str> for HyperlinkLine {
    fn from(text: &'static str) -> Self {
        Self::new(Line::from(text))
    }
}

impl From<String> for HyperlinkLine {
    fn from(text: String) -> Self {
        Self::new(Line::from(text))
    }
}

pub fn visible_lines(lines: Vec<HyperlinkLine>) -> Vec<Line<'static>> {
    lines.into_iter().map(|line| line.line).collect()
}

#[cfg(test)]
fn visible_lines_ref(lines: &[HyperlinkLine]) -> Vec<Line<'_>> {
    lines.iter().map(|line| line_to_borrowed(&line.line)).collect()
}

pub fn plain_hyperlink_lines(lines: Vec<Line<'static>>) -> Vec<HyperlinkLine> {
    lines.into_iter().map(HyperlinkLine::new).collect()
}

pub fn prefix_hyperlink_lines(
    lines: Vec<HyperlinkLine>,
    initial_prefix: Span<'static>,
    subsequent_prefix: Span<'static>,
) -> Vec<HyperlinkLine> {
    lines
        .into_iter()
        .enumerate()
        .map(|(index, mut line)| {
            let prefix =
                if index == 0 { initial_prefix.clone() } else { subsequent_prefix.clone() };
            let shift = display_width(prefix.content.as_ref());
            let mut spans = Vec::with_capacity(line.line.spans.len() + 1);
            spans.push(prefix);
            spans.extend(line.line.spans);
            line.line = Line::from(spans).style(line.line.style);
            for hyperlink in &mut line.hyperlinks {
                hyperlink.columns = hyperlink.columns.start + shift..hyperlink.columns.end + shift;
            }
            line
        })
        .collect()
}

pub fn adaptive_wrap_hyperlink_lines(
    lines: &[HyperlinkLine],
    options: RtOptions<'static>,
) -> Vec<HyperlinkLine> {
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let options = if index == 0 {
            options.clone()
        } else {
            options.clone().initial_indent(options.subsequent_indent.clone())
        };
        out.extend(remap_wrapped_line(
            line,
            adaptive_wrap_line(&line.line, options)
                .into_iter()
                .map(|wrapped| line_to_static(&wrapped))
                .collect(),
        ));
    }
    out
}

pub fn annotate_web_urls(lines: Vec<Line<'static>>) -> Vec<HyperlinkLine> {
    lines.into_iter().map(annotate_web_urls_in_line).collect()
}

pub fn annotate_web_urls_in_line(line: Line<'static>) -> HyperlinkLine {
    let text = line.spans.iter().map(|span| span.content.as_ref()).collect::<String>();
    let mut out = HyperlinkLine::new(line);
    out.hyperlinks = web_links_in_text(&text);
    out
}

/// 可见文本换行拆分一行后重新附加源超链接范围；按展示顺序匹配链接文本，
/// 忽略行边界增删的空白，但绝不从输出重建目标地址。
/// Re-attach source hyperlink ranges after visible-text wrapping has split a line.
///
/// Link text is matched in display order so a URL split across table rows retains the complete
/// destination on every rendered fragment. Whitespace inserted or removed at line boundaries is
/// ignored while matching; hyperlink destinations themselves are never reconstructed from output.
pub fn remap_wrapped_line(
    source: &HyperlinkLine,
    wrapped: Vec<Line<'static>>,
) -> Vec<HyperlinkLine> {
    let mut out = plain_hyperlink_lines(wrapped);
    if source.hyperlinks.is_empty() {
        return out;
    }

    let source_text = line_text(&source.line);
    let mut source_byte = 0usize;
    let mut source_column = 0usize;
    let mut link_index = 0usize;
    for (index, line) in out.iter_mut().enumerate() {
        if index > 0 {
            let trimmed = source_text[source_byte..].trim_start_matches(char::is_whitespace);
            let skipped = source_text[source_byte..].len() - trimmed.len();
            source_column += source_text[source_byte..source_byte + skipped]
                .chars()
                .map(char_cell_width)
                .sum::<usize>();
            source_byte += skipped;
        }

        let rendered = line_text(&line.line);
        let remaining = &source_text[source_byte..];
        let Some(rendered_start) = longest_suffix_matching_prefix(&rendered, remaining) else {
            continue;
        };
        let mapped = &rendered[rendered_start..];
        let mut output_column = display_width(&rendered[..rendered_start]);
        for ch in mapped.chars() {
            let width = char_cell_width(ch);
            while source
                .hyperlinks
                .get(link_index)
                .is_some_and(|link| link.columns.end <= source_column)
            {
                link_index += 1;
            }
            if let Some(link) = source
                .hyperlinks
                .get(link_index)
                .filter(|link| link.columns.contains(&source_column))
            {
                push_link_range(line, output_column..output_column + width, link);
            }
            source_column += width;
            output_column += width;
        }
        source_byte += mapped.len();
    }
    out
}

fn line_text(line: &Line<'_>) -> String {
    line.spans.iter().map(|span| span.content.as_ref()).collect()
}

fn longest_suffix_matching_prefix(rendered: &str, source: &str) -> Option<usize> {
    rendered
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(rendered.len()))
        .find(|index| source.starts_with(&rendered[*index..]) && *index < rendered.len())
}

fn push_link_range(line: &mut HyperlinkLine, range: Range<usize>, link: &TerminalHyperlink) {
    if range.is_empty() {
        return;
    }
    if let Some(previous) = line.hyperlinks.last_mut()
        && previous.destination == link.destination
        && previous.destination_kind == link.destination_kind
        && previous.columns.end == range.start
    {
        previous.columns.end = range.end;
        return;
    }
    line.hyperlinks.push(link.with_columns(range));
}

fn web_links_in_text(text: &str) -> Vec<TerminalHyperlink> {
    // 先按空白切分候选，再剔除外围标点并验证协议与主机。
    let mut links = Vec::new();
    let mut search_from = 0usize;
    for raw_token in text.split_ascii_whitespace() {
        let Some(relative_start) = text[search_from..].find(raw_token) else {
            continue;
        };
        let raw_start = search_from + relative_start;
        search_from = raw_start + raw_token.len();
        let trimmed_start =
            raw_token.find(|ch: char| !is_leading_punctuation(ch)).unwrap_or(raw_token.len());
        let trimmed_end = trailing_url_end(&raw_token[trimmed_start..]) + trimmed_start;
        if trimmed_start >= trimmed_end {
            continue;
        }
        let candidate = &raw_token[trimmed_start..trimmed_end];
        let Some(destination) = web_destination(candidate) else {
            continue;
        };
        let start = display_width(&text[..raw_start + trimmed_start]);
        let end = start + display_width(candidate);
        links.push(TerminalHyperlink::web(start..end, destination));
    }
    links
}

fn is_leading_punctuation(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | '.' | ';' | '!' | '\'' | '"')
}

fn trailing_url_end(candidate: &str) -> usize {
    let mut end = candidate.len();
    while end > 0 {
        let remaining = &candidate[..end];
        let Some(ch) = remaining.chars().next_back() else {
            break;
        };
        let trim = matches!(ch, ',' | '.' | ';' | '!' | '\'' | '"')
            || matches!(ch, ')' | ']' | '}' | '>')
                && has_unmatched_closing_delimiter(remaining, ch);
        if !trim {
            break;
        }
        end -= ch.len_utf8();
    }
    end
}

fn has_unmatched_closing_delimiter(candidate: &str, closing: char) -> bool {
    let opening = match closing {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        '>' => '<',
        _ => return false,
    };
    candidate.chars().filter(|ch| *ch == closing).count()
        > candidate.chars().filter(|ch| *ch == opening).count()
}

pub fn web_destination(destination: &str) -> Option<String> {
    let safe_destination = sanitized_destination(destination);
    let parsed = Url::parse(&safe_destination).ok()?;
    matches!(parsed.scheme(), "http" | "https").then(|| parsed.host_str()).flatten()?;
    Some(safe_destination)
}

fn trusted_file_destination(destination: &str) -> Option<String> {
    let safe_destination = sanitized_destination(destination);
    let parsed = Url::parse(&safe_destination).ok()?;
    (parsed.scheme() == "file" && parsed.to_file_path().is_ok()).then_some(safe_destination)
}

fn sanitized_destination(destination: &str) -> String {
    destination.chars().filter(|ch| !ch.is_control()).collect()
}

pub fn osc8_hyperlink(destination: &str, text: &str) -> String {
    let Some(safe_destination) = web_destination(destination) else {
        return text.to_string();
    };
    format!("\x1b]8;;{safe_destination}\x07{text}\x1b]8;;\x07")
}

pub fn decorate_spans(line: &HyperlinkLine) -> Vec<Span<'static>> {
    if line.hyperlinks.is_empty() {
        return line.line.spans.clone();
    }

    let mut out = Vec::new();
    let mut column = 0usize;
    let mut link_index = 0usize;
    let mut active_link_index = None;
    let mut active_destination: Option<String> = None;
    for span in &line.line.spans {
        for ch in span.content.chars() {
            let width = char_cell_width(ch);
            while line.hyperlinks.get(link_index).is_some_and(|link| link.columns.end <= column) {
                link_index += 1;
            }
            let selected_link_index = line
                .hyperlinks
                .get(link_index)
                .and_then(|link| link.columns.contains(&column).then_some(link_index));
            if active_link_index != selected_link_index {
                if active_destination.is_some() {
                    append_to_last_span(&mut out, "\x1b]8;;\x07");
                }
                active_destination = selected_link_index
                    .and_then(|index| line.hyperlinks[index].terminal_destination());
                if let Some(destination) = active_destination.as_ref() {
                    push_styled_content(
                        &mut out,
                        &format!("\x1b]8;;{destination}\x07"),
                        span.style,
                    );
                }
                active_link_index = selected_link_index;
            }
            push_styled_content(&mut out, ch.encode_utf8(&mut [0; 4]), span.style);
            column += width;
        }
    }
    if active_destination.is_some() {
        append_to_last_span(&mut out, "\x1b]8;;\x07");
    }
    out
}

fn push_styled_content(out: &mut Vec<Span<'static>>, content: &str, style: ratatui::style::Style) {
    if let Some(last) = out.last_mut()
        && last.style == style
    {
        last.content.to_mut().push_str(content);
        return;
    }
    out.push(Span::styled(content.to_string(), style));
}

fn append_to_last_span(out: &mut [Span<'static>], content: &str) {
    if let Some(last) = out.last_mut() {
        last.content.to_mut().push_str(content);
    }
}

fn char_cell_width(ch: char) -> usize {
    if ch.is_control() {
        return 0;
    }
    char_width(ch)
}

pub fn mark_buffer_hyperlinks(
    buf: &mut Buffer,
    area: Rect,
    lines: &[HyperlinkLine],
    scroll_rows: usize,
) {
    if area.width == 0 || area.height == 0 || lines.iter().all(|line| line.hyperlinks.is_empty()) {
        return;
    }
    let viewport_end = scroll_rows.saturating_add(usize::from(area.height));
    let mut logical_row = 0usize;
    for line in lines {
        if logical_row >= viewport_end {
            break;
        }
        let paragraph =
            Paragraph::new(Text::from(line_to_borrowed(&line.line))).wrap(Wrap { trim: false });
        let rendered_height = paragraph.line_count(area.width).max(/*other*/ 1);
        if line.hyperlinks.is_empty() || logical_row.saturating_add(rendered_height) <= scroll_rows
        {
            logical_row += rendered_height;
            continue;
        }

        let layout_area = Rect::new(
            /*x*/ 0,
            /*y*/ 0,
            area.width,
            u16::try_from(rendered_height).unwrap_or(u16::MAX),
        );
        let mut layout = Buffer::empty(layout_area);
        paragraph.render(layout_area, &mut layout);
        let rendered_lines = (0..layout_area.height)
            .map(|row| {
                let mut trailing_columns = 0usize;
                let text = (0..layout_area.width)
                    .filter_map(|column| {
                        if trailing_columns > 0 {
                            trailing_columns -= 1;
                            return None;
                        }
                        let cell = &layout[(column, row)];
                        if cell.diff_option == CellDiffOption::Skip {
                            return None;
                        }
                        trailing_columns = usize::from(cell.cell_width()).saturating_sub(1);
                        Some(cell.symbol())
                    })
                    .collect::<String>();
                Line::from(text.trim_end().to_string())
            })
            .collect();
        for (row, rendered) in remap_wrapped_line(line, rendered_lines).iter().enumerate() {
            let row = logical_row + row;
            if row < scroll_rows || row >= viewport_end {
                continue;
            }
            for link in &rendered.hyperlinks {
                let destination = link.terminal_destination();
                let mut trailing_columns = 0usize;
                for column in link.columns.clone() {
                    if trailing_columns > 0 {
                        trailing_columns -= 1;
                        continue;
                    }
                    let x = area.x + column as u16;
                    let y = area.y + (row - scroll_rows) as u16;
                    let cell = &mut buf[(x, y)];
                    if cell.diff_option == CellDiffOption::Skip {
                        continue;
                    }
                    trailing_columns = usize::from(cell.cell_width()).saturating_sub(1);
                    let symbol = destination.as_ref().map_or_else(
                        || cell.symbol().to_string(),
                        |destination| {
                            format!("\x1b]8;;{destination}\x07{}\x1b]8;;\x07", cell.symbol())
                        },
                    );
                    let width = NonZeroU16::new(cell.cell_width()).unwrap_or(NonZeroU16::MIN);
                    cell.set_symbol(&symbol).set_diff_option(CellDiffOption::ForcedWidth(width));
                }
            }
        }
        logical_row += rendered_height;
    }
}

pub fn mark_url_hyperlink(buf: &mut Buffer, area: Rect, destination: &str) {
    mark_matching_cells(buf, area, destination, |cell| {
        cell.fg == Color::Cyan && cell.modifier.contains(Modifier::UNDERLINED)
    });
}

pub fn mark_underlined_hyperlink(buf: &mut Buffer, area: Rect, destination: &str) {
    mark_matching_cells(buf, area, destination, |cell| {
        cell.modifier.contains(Modifier::UNDERLINED)
    });
}

fn mark_matching_cells(
    buf: &mut Buffer,
    area: Rect,
    destination: &str,
    matches: impl Fn(&ratatui::buffer::Cell) -> bool,
) {
    if web_destination(destination).is_none() {
        return;
    }
    for position in area.positions() {
        let cell = &mut buf[position];
        if cell.diff_option != CellDiffOption::Skip
            && !cell.symbol().trim().is_empty()
            && matches(cell)
        {
            let width = NonZeroU16::new(cell.cell_width()).unwrap_or(NonZeroU16::MIN);
            let symbol = osc8_hyperlink(destination, cell.symbol());
            cell.set_symbol(&symbol).set_diff_option(CellDiffOption::ForcedWidth(width));
        }
    }
}

#[cfg(test)]
#[path = "terminal_hyperlinks_tests.rs"]
mod tests;
