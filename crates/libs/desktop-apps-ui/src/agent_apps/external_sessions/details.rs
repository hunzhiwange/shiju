//! Presentation of host-projected blocks. References are read only through the SDK.
use gateway_external_session::AgentAppsReadError;
use std::collections::BTreeSet;
use super::{Event as ParentEvent, find, images, message, reader};
use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::{
    selectable_text::markdown_selection::selectable_readonly_text,
    single_line_label::SingleLineLabel,
};
use super::Event as Message;
use crate::{Connection, Notice, error as notify_error};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, scrollable, text},
};
use vw_api_external_session::*;

fn event(value: Event) -> Message {
    message(ParentEvent::Details(value))
}

pub(super) fn update(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, value: Event) -> Task<Message> {
    let Some(scope) = reader::current_scope(state, connection) else {
        return Task::none();
    };
    let Some(reader) = &mut state.reader else {
        return Task::none();
    };
    match value {
        Event::Process => reader.details.process_open = !reader.details.process_open,
        Event::Input(m, b) => {
            if !reader.details.inputs.remove(&(m, b)) {
                reader.details.inputs.insert((m, b));
            }
        }
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
            let client = connection.client.clone();
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
            let token = connection.token.clone();
            let task = Task::perform(
                async move {
                    event(Event::Loaded(
                        scope,
                        serial,
                        reference,
                        offset,
                        gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_session_content(&request, &token).await,
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
                _ => return notify_error(notice, "正文读取失败，已加载内容保留，可重试"),
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
pub(super) fn selectable<'a>(
    value: impl Into<std::borrow::Cow<'a, str>>,
    preformatted: bool,
    query: &str,
    i18n: I18n,
) -> Element<'a, Message> {
    let value = value.into();
    let font = if preformatted && value.is_ascii() { iced::Font::MONOSPACE } else { iced::Font::DEFAULT };
    let width = if preformatted { Length::Shrink } else { Length::Fill };
    let ranges = find::match_ranges(&value, query);
    selectable_readonly_text(value, width, font, &ranges, i18n)
}

fn body<'a>(
    reader: &'a reader::State,
    preview: &'a str,
    full: Option<&'a ContentRef>,
    preformatted: bool,
    query: &str,
    i18n: I18n,
) -> Element<'a, Message> {
    let page =
        full.and_then(|reference| reader.details.pages.iter().find(|p| &p.reference == reference));
    let visible = page.filter(|p| p.started).map_or(preview, |p| p.text.as_str());
    let mut content = column![].spacing(6);
    if !visible.is_empty() {
        let content_text = selectable(visible, preformatted, query, i18n);
        if preformatted {
            content = content.push(container(scrollable(content_text)
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
                )).width(Length::Fill)).padding([7, 10]).style(super::list::subtle_surface));
        } else {
            content = content.push(content_text);
        }
    }
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
    content.width(Length::Fill).into()
}

pub(super) fn message_view<'a>(
    reader: &'a reader::State,
    message: &'a SessionMessage,
    query: &str,
    i18n: I18n,
) -> Element<'a, Message> {
    if message.blocks.is_empty() {
        return selectable(message.content.as_str(), false, query, i18n);
    }
    let m = reader.messages.iter().position(|m| std::ptr::eq(m, message)).unwrap_or(0);
    let mut content = column![].spacing(6);
    for (b, block) in message.blocks.iter().enumerate() {
        for image in images::references(block) {
            content = content.push(images::view(reader, image));
        }
        if let SessionBlock::Text { text, full } = block {
            if text.contains("```") {
                content = content.push(block_view(reader, m, b, block, query, i18n));
            } else {
                content = content.push(body(reader, text, full.as_ref(), false, query, i18n));
            }
        }
    }
    content.into()
}

fn failure(block: &SessionBlock) -> bool {
    matches!(
        block,
        SessionBlock::ToolResult { status: ToolStatus::Error | ToolStatus::Interrupted, .. }
            | SessionBlock::Event { kind: EventKind::Error | EventKind::Aborted, .. }
    )
}
pub(super) fn steps<'a>(reader: &'a reader::State, turn: &TurnIndex, query: &str, i18n: I18n) -> Element<'a, Message> {
    let mut blocks: Vec<_> = reader
        .messages
        .iter()
        .enumerate()
        .filter(|(i, message)| !message.injected && *i >= turn.first_message_index && *i <= turn.last_message_index)
        .flat_map(|(m, message)| {
            message.blocks.iter().enumerate().map(move |(b, block)| (m, b, block))
        })
        .filter(|(_, _, b)| !matches!(b, SessionBlock::Text { .. } | SessionBlock::Image { .. }))
        .collect();
    if reader.find.filter == find::Filter::Changes {
        blocks.retain(|(_, _, b)| matches!(b, SessionBlock::ToolCall { diff: Some(_), .. }));
    }
    // Preserve the host message and block order, including failures and interruptions.
    let open = reader.details.process_open || reader.find.filter == find::Filter::Changes;
    let mut heading = row![
        icon_svg(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 13.0),
        text(format!("执行过程 · {} 步", turn.step_count)).size(13),
    ].spacing(7).align_y(iced::Alignment::Center);
    if turn.error_count > 0 {
        heading = heading.push(text(format!("{} 个错误", turn.error_count)).size(12)
            .style(|theme: &iced::Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().danger.base.color),
            }));
    }
    if turn.aborted {
        heading = heading.push(text("已中止").size(12).style(settings_muted_text_style));
    }
    let summary = button(heading).padding([6, 8])
        .style(|theme, status| super::list::flat_button(theme, status, false))
        .on_press_maybe((reader.find.filter != find::Filter::Changes).then(|| event(Event::Process)));
    let mut timeline = column![].spacing(2);
    for (m, b, block) in blocks {
        if open || failure(block) || find::block_matches(reader, block, query) {
            timeline = timeline.push(block_view(reader, m, b, block, query, i18n));
        }
    }
    let timeline = row![
        iced::widget::rule::vertical(1),
        container(timeline).width(Length::Fill),
    ].spacing(6);
    column![summary, timeline].padding([0, 8]).spacing(4).into()
}

fn block_view<'a>(
    reader: &'a reader::State,
    m: usize,
    b: usize,
    block: &'a SessionBlock,
    query: &str,
    i18n: I18n,
) -> Element<'a, Message> {
    if let SessionBlock::Thinking { redacted: true, .. } = block {
        return container(text("·  思考内容已隐去").size(12).style(settings_muted_text_style))
            .padding([4, 9]).into();
    }
    let label = match block {
        SessionBlock::Text { .. } => "代码 / 正文".to_owned(),
        SessionBlock::Thinking { summary, duration_ms, .. } => {
            format!("思考 · {} · {}ms", summary.as_deref().unwrap_or(""), duration_ms.map(|ms| ms.to_string()).unwrap_or_else(|| "未知".into()))
        }
        SessionBlock::ToolCall { title, raw_name, detail, diff, .. } => {
            let title = if title.trim().is_empty() { raw_name } else { title };
            format!("{title}{}{}",
                detail.as_ref().filter(|value| *value != title)
                    .map(|d| format!("  ·  {d}")).unwrap_or_default(),
                diff.as_ref().map(|d| format!(" · {} 个文件 · +{} −{}", d.files.len(), d.added, d.removed)).unwrap_or_default())
        }
        SessionBlock::ToolResult { status, exit_code, .. } => {
            let label = match status {
                ToolStatus::Error => "工具执行失败",
                ToolStatus::Interrupted => "工具执行中断",
                ToolStatus::Pending => "工具执行中",
                _ => "工具输出",
            };
            exit_code.map_or_else(|| label.to_owned(), |code| format!("{label} · 退出码 {code}"))
        }
        SessionBlock::Event { kind, .. } => format!("事件 · {kind:?}"),
        SessionBlock::Step { phase, reason, .. } => {
            format!("步骤 · {phase:?} · {}", reason.as_deref().unwrap_or(""))
        }
        SessionBlock::Image { .. } => return column![].into(),
    };
    let expanded = reader.details.expanded.contains(&(m, b))
        || reader.find.filter == find::Filter::Changes
        || find::block_matches(reader, block, query);
    let failed = failure(block);
    let completed = matches!(block, SessionBlock::ToolResult { status: ToolStatus::Success, .. });
    let marker = text("•").size(14).style(move |theme: &iced::Theme| {
        let palette = theme.extended_palette();
        iced::widget::text::Style {
            color: Some(if failed { palette.danger.base.color }
                else if completed { palette.success.base.color }
                else { theme.palette().text.scale_alpha(0.55) }),
        }
    });
    let mut heading = row![
        container(marker).center_x(11),
        container(SingleLineLabel::new(label, 13.0)).width(Length::Fill),
    ].spacing(8).align_y(iced::Alignment::Center);
    if let SessionBlock::ToolResult { duration_ms: Some(ms), .. } = block {
        heading = heading.push(text(format!("{ms} ms")).size(12).style(settings_muted_text_style));
    }
    heading = heading.push(icon_svg(if expanded { Icon::ChevronDown } else { Icon::ChevronRight }, 12.0));
    let mut content = column![button(heading).width(Length::Fill).padding([4, 8])
        .style(|theme, status| super::list::flat_button(theme, status, false))
        .on_press(event(Event::Toggle(m, b)))].spacing(5);
    if expanded {
        let mut details = column![].spacing(6).width(Length::Fill);
        for image in images::references(block) {
            details = details.push(images::view(reader, image));
        }
        match block {
            SessionBlock::Text { text, full } | SessionBlock::Thinking { text, full, .. } => {
                details = details.push(body(reader, text, full.as_ref(), false, query, i18n));
            }
            SessionBlock::ToolCall { title, input_preview, input_full, detail, diff, .. } => {
                let description = detail.as_deref().unwrap_or(title.as_str());
                if title != description && find::matches(title, query) {
                    details = details.push(selectable(title.as_str(), false, query, i18n));
                }
                if !description.is_empty() {
                    details = details.push(selectable(description, false, query, i18n));
                }
                if !input_preview.is_empty() || input_full.is_some() {
                    let visible_input = loaded_text(reader, input_full.as_ref()).unwrap_or(input_preview);
                    let input_open = reader.details.inputs.contains(&(m, b)) || find::matches(visible_input, query);
                    let disclosure = row![
                        icon_svg(if input_open { Icon::ChevronDown } else { Icon::ChevronRight }, 12.0),
                        text("参数").size(12).style(settings_muted_text_style),
                    ].spacing(5).align_y(iced::Alignment::Center);
                    details = details.push(button(disclosure).padding([4, 5])
                        .style(|theme, status| super::list::flat_button(theme, status, false))
                        .on_press(event(Event::Input(m, b))));
                    if input_open {
                        details = details.push(nested_output(body(reader, input_preview, input_full.as_ref(), true, query, i18n)));
                    }
                }
                if let Some(diff) = diff {
                    details = details.push(text(format!("改动 · +{} −{}", diff.added, diff.removed))
                        .size(12).style(settings_muted_text_style));
                    for file in &diff.files {
                        details = details.push(nested_output(selectable(format!(
                            "{} · {:?} · +{} −{}",
                            file.path, file.op, file.added, file.removed
                        ), false, query, i18n)));
                    }
                    if diff.full.is_some() {
                        details = details.push(nested_output(body(reader, "", diff.full.as_ref(), true, query, i18n)));
                    }
                }
            }
            SessionBlock::ToolResult { preview, full, truncated, .. } => {
                if *truncated {
                    details = details.push(text("预览已截断").size(12).style(settings_muted_text_style));
                }
                details = details.push(nested_output(body(reader, preview, full.as_ref(), true, query, i18n)));
            }
            SessionBlock::Event { text, full, .. } => {
                details = details.push(body(reader, text.as_deref().unwrap_or(""), full.as_ref(), false, query, i18n));
            }
            _ => {}
        }
        content = content.push(container(details).padding(iced::Padding {
            left: 27.0,
            right: 8.0,
            bottom: 5.0,
            ..Default::default()
        }));
    }
    content.into()
}

fn nested_output(content: Element<'_, Message>) -> Element<'_, Message> {
    row![
        text("└").size(12).style(settings_muted_text_style),
        container(content).width(Length::Fill),
    ].spacing(7).into()
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

#[derive(Clone, Default)]
pub struct State {
    pub expanded: BTreeSet<(usize, usize)>,
    pub process_open: bool,
    pub inputs: BTreeSet<(usize, usize)>,
    pub pages: Vec<Page>,
}
#[derive(Clone)]
pub struct Page {
    pub reference: ContentRef,
    pub text: String,
    pub next: Option<u32>,
    pub started: bool,
    pub pending: bool,
    pub serial: u64,
    pub abort: Option<iced::task::Handle>,
}
impl State {
    pub fn cancel(&self) {
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
    Input(usize, usize),
    More(ContentRef),
    Loaded(reader::Scope, u64, ContentRef, u32, Result<SessionContentResponse, AgentAppsReadError>),
}
