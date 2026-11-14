//! Presentation of host-projected blocks. References are read only through the SDK.
use super::{Event as ParentEvent, find, images, message, reader};
use crate::app::{App, Message, components::system_settings_common::*};
use iced::{
    Element, Task,
    widget::{button, column, text},
};
use std::collections::BTreeSet;
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(super) struct State {
    expanded: BTreeSet<(usize, usize)>,
    process_open: bool,
    pages: Vec<Page>,
}
#[derive(Clone)]
struct Page {
    reference: ContentRef,
    text: String,
    next: Option<u32>,
    started: bool,
    pending: bool,
    serial: u64,
    abort: Option<iced::task::Handle>,
}
impl State {
    pub(super) fn cancel(&self) {
        for page in &self.pages {
            if let Some(abort) = &page.abort {
                abort.abort();
            }
        }
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Process,
    Toggle(usize, usize),
    More(ContentRef),
    Loaded(reader::Scope, u64, ContentRef, u32, Result<SessionContentResponse, AgentAppsReadError>),
}
fn event(value: Event) -> Message {
    message(ParentEvent::Details(value))
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(scope) = reader::current_scope(app) else {
        return Task::none();
    };
    let Some(reader) = &mut app.apps_catalog.external_sessions.reader else {
        return Task::none();
    };
    match event {
        Event::Process => reader.details.process_open = !reader.details.process_open,
        Event::Toggle(m, b) => {
            if !reader.details.expanded.remove(&(m, b)) {
                reader.details.expanded.insert((m, b));
            }
        }
        Event::More(reference) => {
            // Accept only references in visible, non-redacted transcript blocks.
            if !reader
                .messages
                .iter()
                .flat_map(|m| &m.blocks)
                .any(|b| references(b).contains(&&reference))
            {
                return Task::none();
            }
            let Ok(client) = crate::app::config::gateway_client() else {
                return Task::none();
            };
            let state = &mut reader.details;
            let index =
                state.pages.iter().position(|p| p.reference == reference).unwrap_or_else(|| {
                    state.pages.push(Page {
                        reference: reference.clone(),
                        text: String::new(),
                        next: None,
                        started: false,
                        pending: false,
                        serial: 0,
                        abort: None,
                    });
                    state.pages.len() - 1
                });
            let page = &mut state.pages[index];
            if page.pending || (page.started && page.next.is_none()) {
                return Task::none();
            }
            let offset = page.next.unwrap_or(0);
            let serial = rand::random::<u64>();
            page.serial = serial;
            page.pending = true;
            let request = SessionContentRequest {
                identity: reader.identity.clone(),
                content_ref: reference.clone(),
                offset: Some(offset),
                limit: Some(8192),
            };
            let token = app.apps_catalog.token.clone();
            let task = Task::perform(
                async move {
                    event(Event::Loaded(
                        scope,
                        serial,
                        reference,
                        offset,
                        client.external_session_content(&request, &token).await,
                    ))
                },
                |m| m,
            );
            let (task, abort) = task.abortable();
            page.abort = Some(abort.abort_on_drop());
            return task;
        }
        Event::Loaded(expected, serial, reference, offset, result) => {
            if scope != expected {
                return Task::none();
            }
            let Some(page) = reader
                .details
                .pages
                .iter_mut()
                .find(|p| p.reference == reference && p.serial == serial && p.pending)
            else {
                return Task::none();
            };
            page.pending = false;
            page.abort = None;
            match result {
                Ok(response)
                    if response.next_offset.is_none_or(|next| next > offset)
                        && (!response.truncated || response.next_offset.is_some()) =>
                {
                    page.text.push_str(&response.text);
                    page.next = response.next_offset;
                    page.started = true;
                }
                _ => return app.show_error_toast("正文读取失败，已加载内容保留，可重试"),
            }
        }
    }
    Task::none()
}

fn references(block: &SessionBlock) -> Vec<&ContentRef> {
    match block {
        SessionBlock::Text { full, .. }
        | SessionBlock::ToolResult { full, .. }
        | SessionBlock::Event { full, .. } => full.iter().collect(),
        SessionBlock::Thinking { full, redacted: false, .. } => full.iter().collect(),
        SessionBlock::ToolCall { input_full, diff, .. } => {
            input_full.iter().chain(diff.iter().filter_map(|d| d.full.as_ref())).collect()
        }
        _ => Vec::new(),
    }
}
fn body<'a>(
    reader: &'a reader::State,
    preview: &'a str,
    full: Option<&'a ContentRef>,
) -> Element<'a, Message> {
    let page =
        full.and_then(|reference| reader.details.pages.iter().find(|p| &p.reference == reference));
    let mut content =
        column![text(page.filter(|p| p.started).map_or(preview, |p| p.text.as_str()))].spacing(6);
    if let Some(reference) = full {
        if page.is_none_or(|p| !p.started || p.next.is_some()) {
            content = content.push(
                button(text(if page.is_some_and(|p| p.pending) {
                    "正在读取…"
                } else {
                    "读取更多正文"
                }))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (!page.is_some_and(|p| p.pending))
                        .then(|| event(Event::More(reference.clone()))),
                ),
            );
        }
    }
    content.into()
}

pub(super) fn message_view<'a>(
    reader: &'a reader::State,
    message: &'a SessionMessage,
) -> Element<'a, Message> {
    if message.blocks.is_empty() {
        return text(&message.content).into();
    }
    let m = reader.messages.iter().position(|m| std::ptr::eq(m, message)).unwrap_or(0);
    let mut content = column![].spacing(6);
    for (b, block) in message.blocks.iter().enumerate() {
        for image in images::references(block) {
            content = content.push(images::view(reader, image));
        }
        if let SessionBlock::Text { text, full } = block {
            if text.contains("```") {
                content = content.push(block_view(reader, m, b, block));
            } else {
                content = content.push(body(reader, text, full.as_ref()));
            }
        }
    }
    content.into()
}

fn failure(block: &SessionBlock) -> bool {
    matches!(
        block,
        SessionBlock::ToolResult { status: ToolStatus::Error, .. }
            | SessionBlock::Event { kind: EventKind::Error | EventKind::Hook, .. }
    )
}
pub(super) fn steps<'a>(reader: &'a reader::State, turn: &TurnIndex) -> Element<'a, Message> {
    let mut blocks: Vec<_> = reader
        .messages
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= turn.first_message_index && *i <= turn.last_message_index)
        .flat_map(|(m, message)| {
            message.blocks.iter().enumerate().map(move |(b, block)| (m, b, block))
        })
        .filter(|(_, _, b)| !matches!(b, SessionBlock::Text { .. } | SessionBlock::Image { .. }))
        .collect();
    if reader.find.filter == find::Filter::Changes {
        blocks.retain(|(_, _, b)| matches!(b, SessionBlock::ToolCall { diff: Some(_), .. }));
    }
    // Stable priority within the execution area, without changing host turn boundaries.
    blocks.sort_by_key(|(_, _, block)| !failure(block));
    let mut content = column![
        button(text(if reader.details.process_open {
            "收起执行过程"
        } else {
            "展开执行过程"
        }))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(event(Event::Process))
    ]
    .spacing(8);
    for (m, b, block) in blocks {
        if reader.details.process_open
            || failure(block)
            || reader.find.filter == find::Filter::Changes
        {
            content = content.push(block_view(reader, m, b, block));
        }
    }
    content.into()
}

fn block_view<'a>(
    reader: &'a reader::State,
    m: usize,
    b: usize,
    block: &'a SessionBlock,
) -> Element<'a, Message> {
    if let SessionBlock::Thinking { redacted: true, .. } = block {
        return text("思考内容已隐去").style(settings_muted_text_style).into();
    }
    let label = match block {
        SessionBlock::Text { .. } => "代码 / 正文".to_owned(),
        SessionBlock::Thinking { summary, duration_ms, .. } => {
            format!("思考 · {} · {}ms", summary.as_deref().unwrap_or(""), duration_ms.unwrap_or(0))
        }
        SessionBlock::ToolCall { title, raw_name, id, .. } => {
            format!("工具 · {title} · {raw_name} · {id}")
        }
        SessionBlock::ToolResult { call_id, status, exit_code, .. } => format!(
            "{}工具结果 · {call_id} · {status:?} · 退出码 {exit_code:?}",
            if failure(block) { "失败 · " } else { "" }
        ),
        SessionBlock::Event { kind, .. } => format!("事件 · {kind:?}"),
        SessionBlock::Step { phase, reason, .. } => {
            format!("步骤 · {phase:?} · {}", reason.as_deref().unwrap_or(""))
        }
        SessionBlock::Image { .. } => return column![].into(),
    };
    let expanded = reader.details.expanded.contains(&(m, b))
        || failure(block)
        || reader.find.filter == find::Filter::Changes;
    let mut content = column![
        button(text(format!("{} {label}", if expanded { "▾" } else { "▸" })))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(event(Event::Toggle(m, b)))
    ]
    .spacing(6);
    if expanded {
        for image in images::references(block) {
            content = content.push(images::view(reader, image));
        }
        match block {
            SessionBlock::Text { text, full } | SessionBlock::Thinking { text, full, .. } => {
                content = content.push(body(reader, text, full.as_ref()))
            }
            SessionBlock::ToolCall { input_preview, input_full, detail, diff, .. } => {
                if let Some(detail) = detail {
                    content = content.push(text(detail));
                }
                content = content.push(text("参数")).push(body(
                    reader,
                    input_preview,
                    input_full.as_ref(),
                ));
                if let Some(diff) = diff {
                    content =
                        content.push(text(format!("差异 · +{} −{}", diff.added, diff.removed)));
                    for file in &diff.files {
                        content = content.push(text(format!(
                            "{} · {:?} · +{} −{}",
                            file.path, file.op, file.added, file.removed
                        )));
                    }
                    content = content.push(body(reader, "", diff.full.as_ref()));
                }
            }
            SessionBlock::ToolResult { preview, full, duration_ms, truncated, .. } => {
                content = content
                    .push(text(format!(
                        "耗时 {}ms{}",
                        duration_ms.unwrap_or(0),
                        if *truncated { " · 预览已截断" } else { "" }
                    )))
                    .push(body(reader, preview, full.as_ref()));
            }
            SessionBlock::Event { text, full, .. } => {
                content = content.push(body(reader, text.as_deref().unwrap_or(""), full.as_ref()))
            }
            _ => {}
        }
    }
    settings_group_card(content).into()
}

/// Only already-authorized, loaded text participates in local find.
pub(super) fn loaded_text<'a>(
    reader: &'a reader::State,
    reference: Option<&ContentRef>,
) -> Option<&'a str> {
    reader
        .details
        .pages
        .iter()
        .find(|p| Some(&p.reference) == reference && p.started)
        .map(|p| p.text.as_str())
}
