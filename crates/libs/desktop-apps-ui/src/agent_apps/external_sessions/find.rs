//! Search the loaded, visible projection without changing host turn identities.
use super::{Event as ParentEvent, details, message, reader};
use desktop_ui_controls::system_settings_common::*;
use super::Event as Message;

use iced::{
    Element,
    widget::{column, row, text, text_input},
};
use vw_api_external_session::*;

struct Hit {
    turn: usize,
    text: String,
}
fn event(value: Event) -> Message {
    message(ParentEvent::Find(value))
}
fn searchable(reader: &reader::State, block: &SessionBlock, filter: Filter) -> String {
    if matches!(block, SessionBlock::Thinking { redacted: true, .. }) {
        return String::new();
    }
    match filter {
        Filter::Conversation if !matches!(block, SessionBlock::Text { .. }) => String::new(),
        Filter::Changes => match block {
            SessionBlock::ToolCall { title, diff: Some(diff), .. } => format!(
                "{}\n{}\n{}",
                title,
                diff.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>().join("\n"),
                details::loaded_text(reader, diff.full.as_ref()).unwrap_or_default()
            ),
            _ => String::new(),
        },
        _ => {
            let projected = match block {
                SessionBlock::Thinking { text, summary, .. } => {
                    format!("{}\n{text}", summary.as_deref().unwrap_or(""))
                }
                SessionBlock::ToolCall { title, detail, input_preview, diff, .. } => format!(
                    "{title}\n{}\n{input_preview}\n{}",
                    detail.as_deref().unwrap_or(""),
                    diff.as_ref()
                        .map(|d| d
                            .files
                            .iter()
                            .map(|f| f.path.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"))
                        .unwrap_or_default()
                ),
                SessionBlock::Step { reason, .. } => reason.clone().unwrap_or_default(),
                _ => project_content(std::slice::from_ref(block)),
            };
            let reference = match block {
                SessionBlock::Text { full, .. }
                | SessionBlock::Thinking { full, .. }
                | SessionBlock::ToolResult { full, .. }
                | SessionBlock::Event { full, .. } => full.as_ref(),
                SessionBlock::ToolCall { input_full, .. } => input_full.as_ref(),
                _ => None,
            };
            let diff = match block {
                SessionBlock::ToolCall { diff: Some(diff), .. } => details::loaded_text(reader, diff.full.as_ref()),
                _ => None,
            };
            format!("{projected}\n{}\n{}", details::loaded_text(reader, reference).unwrap_or_default(),
                diff.unwrap_or_default())
        }
    }
}
/// Convert case-insensitive matches back to byte ranges in the displayed text.
pub(super) fn match_ranges(value: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let folded = value.to_lowercase();
    let mut matches = folded.match_indices(query).peekable();
    if matches.peek().is_none() {
        return Vec::new();
    }
    if value.is_ascii() {
        return matches.map(|(start, hit)| start..start + hit.len()).collect();
    }

    let mut folded_offset = 0;
    let offsets: Vec<_> = value.char_indices().map(|(original, ch)| {
        let start = folded_offset;
        folded_offset += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
        (start, original)
    }).chain(std::iter::once((folded.len(), value.len()))).collect();
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for (start, hit) in matches {
        let from = offsets.partition_point(|&(offset, _)| offset <= start) - 1;
        let to = offsets.partition_point(|&(offset, _)| offset < start + hit.len());
        let range = offsets[from].1..offsets[to].1;
        if let Some(last) = ranges.last_mut().filter(|last| last.end > range.start) {
            last.end = last.end.max(range.end);
        } else {
            ranges.push(range);
        }
    }
    ranges
}

pub(super) fn matches(value: &str, query: &str) -> bool {
    !query.is_empty() && value.to_lowercase().contains(query)
}

pub(super) fn block_matches(reader: &reader::State, block: &SessionBlock, query: &str) -> bool {
    !query.is_empty() && matches(&searchable(reader, block, reader.find.filter), query)
}

fn hits(reader: &reader::State) -> Vec<Hit> {
    let query = reader.find.query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for (t, turn) in reader.turns.iter().enumerate() {
        for (_, m) in reader.messages.iter().enumerate().filter(|(i, m)| {
            !m.injected && *i >= turn.first_message_index && *i <= turn.last_message_index
        }) {
            if m.blocks.is_empty() && reader.find.filter != Filter::Changes {
                if matches(&m.content, &query) {
                    hits.push(Hit { turn: t, text: m.content.clone() });
                }
            } else {
                for b in &m.blocks {
                    let value = searchable(reader, b, reader.find.filter);
                    if matches(&value, &query) {
                        hits.push(Hit { turn: t, text: value });
                    }
                }
            }
        }
    }
    hits
}
pub(super) fn update(reader: &mut reader::State, value: Event) {
    match value {
        Event::Query(query) => {
            reader.find.query = query;
            reader.find.selected = 0;
        }
        Event::Filter(filter) => {
            reader.find.filter = filter;
            reader.find.selected = 0;
            return;
        }
        Event::Move(next) => {
            let count = hits(reader).len();
            if count == 0 {
                return;
            }
            reader.find.selected = if next {
                (reader.find.selected + 1) % count
            } else {
                (reader.find.selected + count - 1) % count
            };
        }
    }
    if let Some(hit) = hits(reader).get(reader.find.selected) {
        reader.active = hit.turn;
    }
}
pub(super) const INPUT_ID: &str = "external-session-find";

pub(super) fn filters(i18n: desktop_i18n::I18n, reader: &reader::State) -> Element<'_, Message> {
    let mut filters = row![].spacing(2);
    for (filter, label) in
        [(Filter::All, i18n.__("全部")), (Filter::Conversation, i18n.__("对话")), (Filter::Changes, i18n.__("改动"))]
    {
        filters = filters.push(super::list::segment(label, reader.find.filter == filter, event(Event::Filter(filter))));
    }
    iced::widget::container(filters).padding(3).style(super::list::subtle_surface).into()
}

pub(super) fn controls(i18n: desktop_i18n::I18n, reader: &reader::State) -> Element<'_, Message> {
    use desktop_assets::assets::Icon;
    let count = hits(reader).len();
    row![
            text_input(i18n.__("查找已加载内容"), &reader.find.query)
                .id(INPUT_ID)
                .style(settings_text_input_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .on_input(|q| event(Event::Query(q))),
            text(i18n.sprintf("%s / %s 处", &[format!("{}", if count == 0 { 0 } else { reader.find.selected.min(count - 1) + 1 }).into(), format!("{}", count).into()])),
            icon_btn(Icon::ArrowUp, i18n.__("上一处"), (count > 0).then(|| event(Event::Move(false)))),
            icon_btn(Icon::ArrowDown, i18n.__("下一处"), (count > 0).then(|| event(Event::Move(true)))),
            icon_btn(Icon::X, i18n.__("关闭查找"), Some(ParentEvent::ReaderSearch)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}
pub(super) fn focused(i18n: desktop_i18n::I18n, reader: &reader::State) -> Option<Element<'_, Message>> {
    let hit = hits(reader).into_iter().nth(reader.find.selected)?;
    if hit.turn != reader.active {
        return None;
    }
    Some(settings_group_card(column![text(i18n.__("当前命中")).size(14), details::selectable(hit.text, false, &reader.find.query.trim().to_lowercase(), i18n)].spacing(6)).into())
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Conversation,
    Changes,
}
#[derive(Clone, Default)]
pub struct State {
    pub query: String,
    pub selected: usize,
    pub filter: Filter,
}
#[derive(Clone, Debug)]
pub enum Event {
    Query(String),
    Move(bool),
    Filter(Filter),
}
