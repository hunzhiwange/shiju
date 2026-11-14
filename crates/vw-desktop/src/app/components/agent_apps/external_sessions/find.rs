//! Search the loaded, visible projection without changing host turn identities.
use super::{Event as ParentEvent, details, message, reader};
use crate::app::{Message, components::system_settings_common::*};
use iced::{
    Element,
    widget::{button, column, row, text, text_input},
};
use vw_gateway_client::vw_api_types::external_session::*;
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Conversation,
    Changes,
}
#[derive(Clone, Default)]
pub(super) struct State {
    query: String,
    selected: usize,
    pub(super) filter: Filter,
}
#[derive(Clone, Debug)]
pub enum Event {
    Query(String),
    Move(bool),
    Filter(Filter),
}
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
            format!("{projected}\n{}", details::loaded_text(reader, reference).unwrap_or_default())
        }
    }
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
                if m.content.to_lowercase().contains(&query) {
                    hits.push(Hit { turn: t, text: m.content.clone() });
                }
            } else {
                for b in &m.blocks {
                    let value = searchable(reader, b, reader.find.filter);
                    if value.to_lowercase().contains(&query) {
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
pub(super) fn controls(reader: &reader::State) -> Element<'_, Message> {
    let count = hits(reader).len();
    let mut filters = row![].spacing(8);
    for (filter, label) in
        [(Filter::All, "全部"), (Filter::Conversation, "对话"), (Filter::Changes, "改动")]
    {
        filters = filters.push(
            button(text(format!(
                "{}{label}",
                if reader.find.filter == filter { "● " } else { "" }
            )))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(event(Event::Filter(filter))),
        );
    }
    column![
        filters,
        row![
            text_input("查找已加载内容", &reader.find.query)
                .style(settings_text_input_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .on_input(|q| event(Event::Query(q))),
            text(format!(
                "{} / {} 处",
                if count == 0 { 0 } else { reader.find.selected.min(count - 1) + 1 },
                count
            )),
            button(text("上一处"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((count > 0).then(|| event(Event::Move(false)))),
            button(text("下一处"))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe((count > 0).then(|| event(Event::Move(true))))
        ]
        .spacing(8),
        text("查找包含已加载正文；每处对应一个内容块，隐去及注入内容不参与")
            .style(settings_muted_text_style)
    ]
    .spacing(8)
    .into()
}
pub(super) fn focused(reader: &reader::State) -> Option<Element<'_, Message>> {
    let hit = hits(reader).into_iter().nth(reader.find.selected)?;
    if hit.turn != reader.active {
        return None;
    }
    Some(settings_group_card(column![text("当前命中").size(14), text(hit.text)].spacing(6)).into())
}
