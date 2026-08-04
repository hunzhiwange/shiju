//! 纯 Markdown 到 Ratatui 文本行的转换。

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::TuiTheme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownLink {
    pub(crate) text: String,
    pub(crate) target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarkdownRender {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) links: Vec<MarkdownLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamingMarkdownSplit {
    pub(crate) stable_prefix: String,
    pub(crate) unstable_suffix: String,
}

#[derive(Debug, Clone)]
struct StyledText {
    text: String,
    style: Style,
}

#[derive(Debug)]
struct ListState {
    next: Option<u64>,
}

#[derive(Debug, Default)]
struct TableState {
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
}

/// 使用 GFM parser 将 Markdown 纯转换为带样式的终端行。
pub(crate) fn render_markdown(markdown: &str, width: u16, theme: TuiTheme) -> MarkdownRender {
    let width = usize::from(width.max(1));
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut renderer = MarkdownRenderer::new(width, theme);
    for event in Parser::new_ext(markdown, options) {
        renderer.handle(event);
    }
    renderer.finish()
}

/// 计算流式 Markdown 的单调稳定前缀。调用方只需缓存此前返回的前缀。
pub(crate) fn split_streaming_markdown(
    markdown: &str,
    previous_stable_prefix: &str,
) -> StreamingMarkdownSplit {
    let previous = if markdown.starts_with(previous_stable_prefix) {
        previous_stable_prefix
    } else {
        ""
    };
    let mut offset = 0usize;
    let mut safe_boundary = previous.len();
    let mut fence: Option<&str> = None;

    for line in markdown.split_inclusive('\n') {
        offset = offset.saturating_add(line.len());
        let trimmed = line.trim_start();
        let marker = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        if let Some(marker) = marker {
            fence = match fence {
                Some(active) if active == marker => None,
                None => Some(marker),
                active => active,
            };
        }
        if fence.is_none() && trimmed.trim().is_empty() {
            safe_boundary = safe_boundary.max(offset);
        }
    }

    let stable_prefix = markdown[..safe_boundary].to_string();
    let unstable_suffix = markdown[safe_boundary..].to_string();
    StreamingMarkdownSplit { stable_prefix, unstable_suffix }
}

struct MarkdownRenderer {
    width: usize,
    theme: TuiTheme,
    lines: Vec<Line<'static>>,
    current: Vec<StyledText>,
    lists: Vec<ListState>,
    item_prefix: Option<String>,
    quote_depth: usize,
    heading_level: Option<usize>,
    code_language: Option<String>,
    code: String,
    table: Option<TableState>,
    strong_depth: usize,
    emphasis_depth: usize,
    strike_depth: usize,
    active_link: Option<(String, String)>,
    links: Vec<MarkdownLink>,
}

impl MarkdownRenderer {
    fn new(width: usize, theme: TuiTheme) -> Self {
        Self {
            width,
            theme,
            lines: Vec::new(),
            current: Vec::new(),
            lists: Vec::new(),
            item_prefix: None,
            quote_depth: 0,
            heading_level: None,
            code_language: None,
            code: String::new(),
            table: None,
            strong_depth: 0,
            emphasis_depth: 0,
            strike_depth: 0,
            active_link: None,
            links: Vec::new(),
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        if self.handle_table_event(&event) {
            return;
        }
        if self.code_language.is_some() {
            match event {
                Event::Text(text) | Event::Code(text) => self.code.push_str(text.as_ref()),
                Event::End(Tag::CodeBlock(_)) => self.flush_code_block(),
                _ => {}
            }
            return;
        }

        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text(text.as_ref(), None),
            Event::Code(code) => {
                let style = self
                    .theme
                    .fg_style(self.theme.accent)
                    .bg(self.theme.surface_alt)
                    .add_modifier(Modifier::BOLD);
                self.push_text(code.as_ref(), Some(style));
            }
            Event::Html(html) => self.push_text(html.as_ref(), None),
            Event::SoftBreak => self.push_text(" ", None),
            Event::HardBreak => self.flush_current(),
            Event::Rule => {
                self.flush_current();
                self.push_block(vec![Line::styled(
                    "─".repeat(self.width),
                    self.theme.muted_style(),
                )]);
            }
            Event::TaskListMarker(checked) => {
                self.push_text(if checked { "[x] " } else { "[ ] " }, None);
            }
            Event::FootnoteReference(label) => {
                self.push_text(format!("[{label}]").as_str(), None);
            }
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading(level, _, _) => {
                self.flush_current();
                self.heading_level = Some(level as usize);
            }
            Tag::BlockQuote => self.quote_depth = self.quote_depth.saturating_add(1),
            Tag::CodeBlock(kind) => {
                self.flush_current();
                self.code_language = Some(match kind {
                    CodeBlockKind::Indented => String::new(),
                    CodeBlockKind::Fenced(language) => language.trim().to_string(),
                });
            }
            Tag::List(start) => self.lists.push(ListState { next: start }),
            Tag::Item => {
                self.flush_current();
                let depth = self.lists.len().saturating_sub(1);
                let marker = match self.lists.last_mut().and_then(|list| list.next.as_mut()) {
                    Some(next) => {
                        let marker = format!("{next}. ");
                        *next = next.saturating_add(1);
                        marker
                    }
                    None => "• ".to_string(),
                };
                self.item_prefix = Some(format!("{}{}", "  ".repeat(depth), marker));
            }
            Tag::Strong => self.strong_depth = self.strong_depth.saturating_add(1),
            Tag::Emphasis => self.emphasis_depth = self.emphasis_depth.saturating_add(1),
            Tag::Strikethrough => self.strike_depth = self.strike_depth.saturating_add(1),
            Tag::Link(_, target, _) | Tag::Image(_, target, _) => {
                self.active_link = Some((target.to_string(), String::new()));
            }
            Tag::Paragraph
            | Tag::FootnoteDefinition(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell => {}
        }
    }

    fn end_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph | Tag::Heading(_, _, _) | Tag::Item | Tag::FootnoteDefinition(_) => {
                self.flush_current();
                if matches!(tag, Tag::Heading(_, _, _)) {
                    self.heading_level = None;
                }
                if matches!(tag, Tag::Item) {
                    self.item_prefix = None;
                }
            }
            Tag::BlockQuote => self.quote_depth = self.quote_depth.saturating_sub(1),
            Tag::List(_) => {
                self.flush_current();
                self.lists.pop();
            }
            Tag::Strong => self.strong_depth = self.strong_depth.saturating_sub(1),
            Tag::Emphasis => self.emphasis_depth = self.emphasis_depth.saturating_sub(1),
            Tag::Strikethrough => self.strike_depth = self.strike_depth.saturating_sub(1),
            Tag::Link(_, _, _) | Tag::Image(_, _, _) => {
                if let Some((target, text)) = self.active_link.take() {
                    self.links.push(MarkdownLink { text, target });
                }
            }
            Tag::CodeBlock(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell => {}
        }
    }

    fn push_text(&mut self, text: &str, override_style: Option<Style>) {
        if let Some((_, link_text)) = self.active_link.as_mut() {
            link_text.push_str(text);
        }
        let mut style = override_style.unwrap_or_else(|| self.theme.body_style());
        if self.heading_level.is_some() {
            style = self
                .theme
                .accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        }
        if self.strong_depth > 0 {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.emphasis_depth > 0 {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.strike_depth > 0 {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if self.active_link.is_some() {
            style = self.theme.accent_style().add_modifier(Modifier::UNDERLINED);
        }
        self.current.push(StyledText { text: text.to_string(), style });
    }

    fn flush_current(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let prefix = if let Some(level) = self.heading_level {
            format!("{} ", "#".repeat(level))
        } else if let Some(prefix) = self.item_prefix.as_ref() {
            prefix.clone()
        } else if self.quote_depth > 0 {
            format!("{} ", "│".repeat(self.quote_depth))
        } else {
            String::new()
        };
        let segments = std::mem::take(&mut self.current);
        let lines = wrap_segments(prefix.as_str(), &segments, self.width, self.theme);
        self.push_block(lines);
    }

    fn flush_code_block(&mut self) {
        let language = self.code_language.take().unwrap_or_default();
        let mut lines = vec![Line::styled(
            if language.is_empty() { "┌─".to_string() } else { format!("┌─ {language}") },
            self.theme.muted_style(),
        )];
        for code_line in self.code.lines() {
            let segments = [StyledText {
                text: code_line.to_string(),
                style: self.theme.body_style().bg(self.theme.surface_alt),
            }];
            lines.extend(wrap_segments("│ ", &segments, self.width, self.theme));
        }
        if self.code.is_empty() {
            lines.push(Line::styled("│ ", self.theme.body_style().bg(self.theme.surface_alt)));
        }
        lines.push(Line::styled("└─", self.theme.muted_style()));
        self.code.clear();
        self.push_block(lines);
    }

    fn handle_table_event(&mut self, event: &Event<'_>) -> bool {
        if self.table.is_none() {
            if matches!(event, Event::Start(Tag::Table(_))) {
                self.flush_current();
                self.table = Some(TableState::default());
                return true;
            }
            return false;
        }

        let table = self.table.as_mut().expect("table state exists");
        match event {
            Event::Text(text) | Event::Code(text) => table.cell.push_str(text.as_ref()),
            Event::SoftBreak | Event::HardBreak => table.cell.push(' '),
            Event::End(Tag::TableCell) => table.row.push(std::mem::take(&mut table.cell)),
            Event::End(Tag::TableRow | Tag::TableHead) if !table.row.is_empty() => {
                table.rows.push(std::mem::take(&mut table.row));
            }
            Event::End(Tag::Table(_)) => {
                if !table.row.is_empty() {
                    table.rows.push(std::mem::take(&mut table.row));
                }
                let table = self.table.take().expect("table state exists");
                let lines = render_table(&table.rows, self.width, self.theme);
                self.push_block(lines);
            }
            _ => {}
        }
        true
    }

    fn push_block(&mut self, block: Vec<Line<'static>>) {
        if block.is_empty() {
            return;
        }
        if !self.lines.is_empty() && self.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.lines.push(Line::raw(""));
        }
        self.lines.extend(block);
    }

    fn finish(mut self) -> MarkdownRender {
        if self.code_language.is_some() {
            self.flush_code_block();
        }
        self.flush_current();
        while self.lines.last().is_some_and(|line| line.spans.is_empty()) {
            self.lines.pop();
        }
        MarkdownRender { lines: self.lines, links: self.links }
    }
}

fn wrap_segments(
    prefix: &str,
    segments: &[StyledText],
    width: usize,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    let prefix_width = UnicodeWidthStr::width(prefix);
    let continuation = " ".repeat(prefix_width.min(width));
    let mut lines = Vec::new();
    let mut spans = if prefix.is_empty() {
        Vec::new()
    } else {
        vec![Span::styled(truncate_to_width(prefix, width), theme.muted_style())]
    };
    let mut used = prefix_width.min(width);

    for segment in segments {
        for character in segment.text.chars() {
            if character == '\n' {
                lines.push(Line::from(std::mem::take(&mut spans)));
                spans = if continuation.is_empty() {
                    Vec::new()
                } else {
                    vec![Span::styled(continuation.clone(), theme.muted_style())]
                };
                used = continuation.len();
                continue;
            }
            let character_width = UnicodeWidthChar::width(character).unwrap_or_default();
            let rendered = if character_width > width {
                "?"
            } else {
                &segment.text[0..0]
            };
            let (text, cell_width) = if rendered == "?" {
                ("?".to_string(), 1)
            } else {
                (character.to_string(), character_width)
            };
            if used > 0 && used.saturating_add(cell_width) > width {
                lines.push(Line::from(std::mem::take(&mut spans)));
                spans = if continuation.is_empty() {
                    Vec::new()
                } else {
                    vec![Span::styled(continuation.clone(), theme.muted_style())]
                };
                used = continuation.len();
            }
            append_styled(&mut spans, text, segment.style);
            used = used.saturating_add(cell_width);
        }
    }
    if !spans.is_empty() || lines.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn append_styled(spans: &mut Vec<Span<'static>>, text: String, style: Style) {
    if let Some(last) = spans.last_mut()
        && last.style == style
    {
        last.content.to_mut().push_str(text.as_str());
        return;
    }
    spans.push(Span::styled(text, style));
}

fn render_table(rows: &[Vec<String>], width: usize, theme: TuiTheme) -> Vec<Line<'static>> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or_default();
    if columns == 0 {
        return Vec::new();
    }
    let overhead = columns.saturating_mul(3).saturating_add(1);
    if width <= overhead {
        return rows
            .iter()
            .flat_map(|row| {
                let text = format!("| {}", row.join(" | "));
                wrap_segments(
                    "",
                    &[StyledText { text, style: theme.body_style() }],
                    width,
                    theme,
                )
            })
            .collect();
    }

    let natural = (0..columns)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| UnicodeWidthStr::width(cell.as_str()))
                .max()
                .unwrap_or(1)
                .max(1)
        })
        .collect::<Vec<_>>();
    let mut allocated = vec![1usize; columns];
    let mut remaining = width.saturating_sub(overhead).saturating_sub(columns);
    while remaining > 0 {
        let mut advanced = false;
        for column in 0..columns {
            if allocated[column] < natural[column] && remaining > 0 {
                allocated[column] = allocated[column].saturating_add(1);
                remaining = remaining.saturating_sub(1);
                advanced = true;
            }
        }
        if !advanced {
            break;
        }
    }

    let border = |left: char, middle: char, right: char| {
        let mut line = String::new();
        line.push(left);
        for (index, cell_width) in allocated.iter().enumerate() {
            line.push_str("─".repeat(cell_width.saturating_add(2)).as_str());
            line.push(if index + 1 == allocated.len() { right } else { middle });
        }
        Line::styled(line, theme.muted_style())
    };
    let mut lines = vec![border('┌', '┬', '┐')];
    for (row_index, row) in rows.iter().enumerate() {
        let mut line = String::from("│");
        for (column, cell_width) in allocated.iter().copied().enumerate() {
            let cell = row.get(column).map(String::as_str).unwrap_or_default();
            let value = truncate_to_width(cell, cell_width);
            let value_width = UnicodeWidthStr::width(value.as_str());
            line.push(' ');
            line.push_str(value.as_str());
            line.push_str(" ".repeat(cell_width.saturating_sub(value_width)).as_str());
            line.push_str(" │");
        }
        let style = if row_index == 0 {
            theme.body_style().add_modifier(Modifier::BOLD)
        } else {
            theme.body_style()
        };
        lines.push(Line::styled(line, style));
        if row_index == 0 && rows.len() > 1 {
            lines.push(border('├', '┼', '┤'));
        }
    }
    lines.push(border('└', '┴', '┘'));
    lines
}

fn truncate_to_width(value: &str, width: usize) -> String {
    if UnicodeWidthStr::width(value) <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let target = width.saturating_sub(1);
    let mut output = String::new();
    let mut used = 0usize;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or_default();
        if used.saturating_add(character_width) > target {
            break;
        }
        output.push(character);
        used = used.saturating_add(character_width);
    }
    output.push('…');
    output
}
