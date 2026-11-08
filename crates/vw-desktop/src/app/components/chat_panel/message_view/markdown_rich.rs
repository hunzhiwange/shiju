//! Inline Markdown, web links, and GFM tables in assistant replies.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use iced::alignment::Horizontal;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::{LineHeight, Span, Wrapping};
use iced::widget::{responsive, rich_text, scrollable, span, table};
use iced::{Border, Color, Element, Font, Length, Theme};
use pulldown_cmark::{Alignment, BrokenLink, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::app::Message;
use crate::app::message::ViewMessage;

use super::super::tool_text_support::chat_text_font;
use super::styles::MESSAGE_TEXT_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Inline {
    pub(super) text: String,
    pub(super) url: Option<String>,
    pub(super) style: InlineStyle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct InlineStyle {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) strike: bool,
    pub(super) code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MarkdownTable {
    alignments: Vec<Horizontal>,
    pub(super) rows: Vec<Vec<Vec<Inline>>>,
}

impl MarkdownTable {
    pub(super) fn stored_bytes(&self) -> usize {
        self.rows.iter().flatten().flatten().map(Inline::stored_bytes).sum()
    }
}

impl Inline {
    pub(super) fn stored_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.text.len() + self.url.as_ref().map_or(0, String::len)
    }
}

static BARE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?-u:\b)(?:https?://|www\.)[^\s<>"`|，。；：！？、（）【】《》“”‘’]+"#)
        .expect("web URL pattern")
});

fn web_url(target: &str) -> Option<String> {
    let url = reqwest::Url::parse(target).ok()?;
    if matches!(url.scheme(), "http" | "https")
        && url.has_host()
        && url.username().is_empty()
        && url.password().is_none()
    {
        Some(url.into())
    } else {
        None
    }
}

fn trim_url_punctuation(mut candidate: &str) -> &str {
    let mut balances = [('(', ')'), ('[', ']'), ('{', '}')].map(|(open, close)| {
        (close, candidate.matches(close).count().saturating_sub(candidate.matches(open).count()))
    });
    loop {
        candidate = candidate.trim_end_matches(['.', ',', ';', ':', '!', '?', '\'', '*']);
        let Some((_, balance)) = balances
            .iter_mut()
            .find(|(close, balance)| *balance > 0 && candidate.ends_with(*close))
        else {
            return candidate;
        };
        *balance -= 1;
        candidate = &candidate[..candidate.len() - 1];
    }
}

#[derive(Default)]
struct InlineBuilder {
    spans: Vec<Inline>,
    // An invalid Markdown link still suppresses detection inside its label.
    link: Option<Option<String>>,
    image_depth: usize,
    pending_text: String,
    bold: usize,
    italic: usize,
    strike: usize,
}

impl InlineBuilder {
    fn push(&mut self, text: &str, url: Option<String>) {
        self.push_span(text, url, false);
    }

    fn push_span(&mut self, text: &str, url: Option<String>, code: bool) {
        if text.is_empty() {
            return;
        }
        let style = InlineStyle {
            bold: self.bold > 0,
            italic: self.italic > 0,
            strike: self.strike > 0,
            code,
        };
        if let Some(previous) = self.spans.last_mut()
            && previous.url == url
            && previous.style == style
        {
            previous.text.push_str(text);
        } else {
            self.spans.push(Inline { text: text.to_owned(), url, style });
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(url) = &self.link {
            self.push(text, url.clone());
            return;
        }
        if self.image_depth > 0 {
            self.push(text, None);
            return;
        }
        let mut offset = 0;
        for matched in BARE_URL.find_iter(text) {
            if text[..matched.start()].ends_with('@') {
                continue;
            }
            let target = trim_url_punctuation(matched.as_str());
            let url = if target.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("www.")) {
                web_url(&format!("https://{target}"))
            } else {
                web_url(target)
            };
            if let Some(url) = url {
                self.push(&text[offset..matched.start()], None);
                self.push(target, Some(url));
                offset = matched.start() + target.len();
            }
        }
        self.push(&text[offset..], None);
    }

    fn flush_text(&mut self) {
        if !self.pending_text.is_empty() {
            let text = std::mem::take(&mut self.pending_text);
            self.push_text(&text);
        }
    }

    fn finish(mut self) -> Vec<Inline> {
        self.flush_text();
        self.spans
    }

    fn event(&mut self, event: Event<'_>) {
        if let Event::Text(text) = event {
            self.pending_text.push_str(&text);
            return;
        }
        self.flush_text();
        match event {
            Event::Start(Tag::Strong) => self.bold += 1,
            Event::End(TagEnd::Strong) => self.bold -= 1,
            Event::Start(Tag::Emphasis) => self.italic += 1,
            Event::End(TagEnd::Emphasis) => self.italic -= 1,
            Event::Start(Tag::Strikethrough) => self.strike += 1,
            Event::End(TagEnd::Strikethrough) => self.strike -= 1,
            Event::Start(Tag::Link { dest_url, .. }) => self.link = Some(web_url(&dest_url)),
            Event::End(TagEnd::Link) => self.link = None,
            Event::Start(Tag::Image { .. }) => self.image_depth += 1,
            Event::End(TagEnd::Image) => self.image_depth -= 1,
            Event::Code(text) => self.push_span(&text, self.link.clone().flatten(), true),
            Event::Html(text) | Event::InlineHtml(text) => {
                self.push(&text, self.link.clone().flatten());
            }
            Event::SoftBreak | Event::HardBreak => self.push("\n", self.link.clone().flatten()),
            _ => {}
        }
    }
}

pub(super) fn parse_inline(source: &str, references: &BTreeMap<String, String>) -> Vec<Inline> {
    if !source.bytes().any(|byte| b"*_`~[<&\\:".contains(&byte))
        && !source.as_bytes().windows(4).any(|window| window.eq_ignore_ascii_case(b"www."))
    {
        return Vec::new();
    }

    let resolve = |link: BrokenLink<'_>| {
        references.get(&link.reference.to_lowercase()).map(|dest| (dest.clone().into(), "".into()))
    };
    let mut inline = InlineBuilder::default();
    for event in
        Parser::new_with_broken_link_callback(source, Options::ENABLE_STRIKETHROUGH, Some(resolve))
    {
        inline.event(event);
    }
    inline.finish()
}

#[derive(Default)]
pub(super) struct MarkdownDocument {
    pub(super) tables: BTreeMap<usize, (usize, MarkdownTable)>,
    pub(super) references: BTreeMap<String, String>,
    pub(super) definitions: BTreeMap<usize, usize>,
}

pub(super) fn parse_document(input: &str) -> MarkdownDocument {
    if !input.contains('|') && !input.contains("]:") {
        return MarkdownDocument::default();
    }
    let parser = Parser::new_ext(input, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH);
    let mut document = MarkdownDocument::default();
    for (label, definition) in parser.reference_definitions().iter() {
        document.references.insert(label.to_lowercase(), definition.dest.to_string());
        let start = definition.span.start;
        let line_start = input[..start].rfind('\n').map_or(0, |newline| newline + 1);
        if input[line_start..start].trim().is_empty() {
            document.definitions.insert(line_start, definition.span.end);
        }
    }
    let mut events = parser.into_offset_iter();
    while let Some((event, range)) = events.next() {
        let Event::Start(Tag::Table(alignments)) = event else { continue };
        let line_start = input[..range.start].rfind('\n').map_or(0, |index| index + 1);
        if !input[line_start..range.start].trim().is_empty() {
            continue;
        }

        let mut rows = Vec::new();
        let mut row = Vec::new();
        let mut cell = InlineBuilder::default();
        for (event, _) in events.by_ref() {
            match event {
                Event::End(TagEnd::Table) => break,
                Event::End(TagEnd::TableCell) => row.push(std::mem::take(&mut cell).finish()),
                Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                    rows.push(std::mem::take(&mut row));
                }
                event => cell.event(event),
            }
        }
        let alignments = alignments
            .into_iter()
            .map(|alignment| match alignment {
                Alignment::None | Alignment::Left => Horizontal::Left,
                Alignment::Center => Horizontal::Center,
                Alignment::Right => Horizontal::Right,
            })
            .collect();
        document.tables.insert(line_start, (range.end, MarkdownTable { alignments, rows }));
    }
    document
}

pub(super) fn link_click_message(url: String) -> Message {
    Message::View(ViewMessage::OpenUrlExternal(url))
}

pub(super) fn inline_view<'a>(
    spans: &'a [Inline],
    size: f32,
    body_color: Color,
    theme: &Theme,
) -> iced::widget::text::Rich<'a, String, Message> {
    let (link_color, emphasis_color) = if super::styles::is_dark_theme(theme) {
        (Color::from_rgb8(147, 197, 253), Color::from_rgb8(216, 180, 254))
    } else {
        (Color::from_rgb8(29, 78, 216), Color::from_rgb8(107, 33, 168))
    };
    let code_background = theme.palette().text.scale_alpha(0.08);
    let spans: Vec<Span<'_, String>> = spans
        .iter()
        .map(|part| {
            let style = part.style;
            let mut value = span(part.text.as_str()).strikethrough(style.strike);
            if style.code {
                let font = if part.text.is_ascii() { Font::MONOSPACE } else { chat_text_font() };
                value = value
                    .font(font)
                    .background(code_background)
                    .border(Border { radius: 3.0.into(), ..Border::default() });
            } else if style.bold || style.italic {
                let mut font = if style.italic && part.text.is_ascii() {
                    Font::DEFAULT
                } else {
                    chat_text_font()
                };
                if style.bold {
                    font.weight = iced::font::Weight::Bold;
                }
                if style.italic {
                    font.style = iced::font::Style::Italic;
                    // The bundled CJK face lacks italics. Keep emphasis visible with color.
                    if !part.text.is_ascii() {
                        value = value.color(emphasis_color);
                    }
                }
                value = value.font(font);
            }
            if let Some(url) = &part.url {
                value = value.link(url.clone()).underline(true).color(link_color);
            }
            value
        })
        .collect();
    rich_text(spans)
        .size(size)
        .font(chat_text_font())
        .color(body_color)
        .line_height(LineHeight::Relative(1.6))
        .wrapping(Wrapping::WordOrGlyph)
        .on_link_click(link_click_message)
}

pub(super) fn table_view<'a>(
    data: &'a MarkdownTable,
    theme: &Theme,
    body_color: Color,
) -> Element<'a, Message> {
    let theme = theme.clone();
    responsive(move |size| {
        const PADDING_X: f32 = 10.0;
        const SEPARATOR: f32 = 1.0;
        const MIN_CELL_WIDTH: f32 = 144.0;
        let count = data.alignments.len() as f32;
        let decoration_width = count * (2.0 * PADDING_X + SEPARATOR) - SEPARATOR;
        let cell_width =
            Length::Fixed(((size.width - decoration_width) / count).max(MIN_CELL_WIDTH));
        let columns = data.alignments.iter().enumerate().map(|(index, &alignment)| {
            let theme = &theme;
            let cell = move |row: &'a Vec<Vec<Inline>>| -> Element<'a, Message> {
                let content = row.get(index).map_or(&[][..], Vec::as_slice);
                inline_view(content, MESSAGE_TEXT_SIZE, body_color, theme)
                    .width(cell_width)
                    .align_x(alignment)
                    .into()
            };
            let header = inline_view(&data.rows[0][index], MESSAGE_TEXT_SIZE, body_color, theme)
                .width(cell_width)
                .align_x(alignment)
                .font(Font { weight: iced::font::Weight::Bold, ..chat_text_font() });
            table::column(header, cell).width(cell_width)
        });
        scrollable(table(columns, &data.rows[1..]).padding_x(PADDING_X).separator_x(SEPARATOR))
            .direction(Direction::Horizontal(
                Scrollbar::new().width(4).scroller_width(4).spacing(3),
            ))
            .width(Length::Fill)
            .into()
    })
    .height(Length::Shrink)
    .into()
}
