//! Scoped, cancellable history reading. Turn boundaries come from the host projection.
use crate::AgentAppsContext;
use super::{Event, details, find, images, message};
use desktop_ui_controls::system_settings_common::*;
use gateway_external_session::AgentAppsReadError;
use super::Event as Message;
use crate::{Connection, Notice, error as notify_error};
use iced::widget::scrollable::{AbsoluteOffset, Direction, Scrollbar};
use iced::{
    Element, Length, Task,
    futures::SinkExt,
    widget::{button, column, container, row, scrollable, text},
};
use vw_api_external_session::*;

pub(super) fn handles(event: &Event) -> bool {
    matches!(
        event,
        Event::Find(_)
            | Event::Read(_)
            | Event::ReaderOutline
            | Event::ReaderOptions
            | Event::Back
            | Event::Adjacent(_)
            | Event::Turn(_)
            | Event::ListScrolled(_)
            | Event::ReaderScrolled(_)
            | Event::Chunk(..)
    )
}

pub(super) fn update(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, event: Event) -> Task<Message> {
    if let Some(task) = navigate(state, &event) { return task; }
    match event {
        Event::Adjacent(next) => {

            let Some(reader) = &state.reader else {
                return Task::none();
            };
            let sessions = super::list::readable_sessions(state);
            let Some(index) =
                sessions.iter().position(|session| session.identity == reader.identity)
            else {
                return Task::none();
            };
            let index = if next { index.checked_add(1) } else { index.checked_sub(1) };
            if let Some(session) = index.and_then(|i| sessions.get(i)) {
                let identity = session.identity.clone();
                return update(state, connection, notice, Event::Read(identity));
            }
        }
        Event::Read(identity) => return start(state, connection, notice, identity),
        Event::Chunk(scope, result) => {
            let context = connection.context.clone();
            if scope.context != context
                || connection.catalog_context.as_ref() != Some(&context)
                || scope.catalog != connection.generation
                || scope.identity.target_host != connection.host
                || Some(connection.project.as_str()) != Some(scope.identity.scope.project_id().as_ref())
            {
                return Task::none();
            }
            let Some(reader) = &mut state.reader else {
                return Task::none();
            };
            if reader.generation != scope.generation
                || reader.identity != scope.identity
                || !reader.loading
            {
                return Task::none();
            }
            match result {
                Ok(TranscriptChunk::Header { total, turns, .. }) => {
                    reader.total = total;
                    reader.turns = turns;
                }
                Ok(TranscriptChunk::Messages { messages, .. }) => reader.messages.extend(messages),
                Ok(TranscriptChunk::Done { .. }) => {
                    reader.loading = false;
                    reader.abort = None;
                }
                Ok(TranscriptChunk::Error { .. }) | Err(_) => {
                    reader.failed = true;
                    reader.loading = false;
                    reader.cancel();
                    reader.abort = None;
                    return notify_error(notice, "会话读取失败，已加载内容保留，请返回列表重试");
                }
            }
        }
        _ => {}
    }
    Task::none()
}

fn start(state: &mut super::State, connection: &Connection, notice: &mut Option<Notice>, identity: ExternalSessionIdentity) -> Task<Message> {
    let client = connection.client.clone();
    let context = connection.context.clone();

    let session =
        state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == identity));
    let Some(session) = session.filter(|s| s.capabilities.read) else {
        return notify_error(notice, "当前会话不支持读取");
    };
    if !state.open
        || connection.catalog_context.as_ref() != Some(&context)
        || identity.target_host != connection.host
        || Some(connection.project.as_str()) != Some(identity.scope.project_id().as_ref())
    {
        return notify_error(notice, "会话作用域已变化，请刷新列表");
    }
    let title = session.title.clone().unwrap_or_else(|| identity.session_id.0.clone());
    state.cancel_reader();
    state.busy = false;
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope {
        context,
        catalog: connection.generation,
        generation: state.generation,
        identity: identity.clone(),
    };
    let request = SessionStreamRequest {
        identity: identity.clone(),
        read_id: format!("{:032x}", rand::random::<u128>()),
    };
    let token = connection.token.clone();
    #[cfg(not(target_arch = "wasm32"))]
    let task = Task::run(
        iced::stream::channel(1, async move |mut output| {
            match gateway_external_session::GatewayExternalSessionClient::new(client.transport().clone()).external_session_stream(&request, &token).await {
                Ok(mut stream) => loop {
                    let chunk = stream.next_chunk().await;
                    let terminal = !matches!(
                        &chunk,
                        Ok(Some(TranscriptChunk::Header { .. } | TranscriptChunk::Messages { .. }))
                    );
                    let result = match chunk {
                        Ok(Some(chunk)) => Ok(chunk),
                        Ok(None) => Err(AgentAppsReadError::Transport(
                            "history stream ended before completion".into(),
                        )),
                        Err(error) => Err(error),
                    };
                    if output.send(message(Event::Chunk(scope.clone(), result))).await.is_err()
                        || terminal
                    {
                        break;
                    }
                },
                Err(error) => {
                    let _ = output.send(message(Event::Chunk(scope, Err(error)))).await;
                }
            }
        }),
        |message| message,
    );
    #[cfg(target_arch = "wasm32")]
    let task = Task::done(message(Event::Chunk(
        scope,
        Err(AgentAppsReadError::Transport(
            "authorized history reads require native transport".into(),
        )),
    )));
    let (task, abort) = task.abortable();
    state.reader = Some(State {
        identity,
        title,
        turns: Vec::new(),
        messages: Vec::new(),
        active: 0,
        loading: true,
        failed: false,
        total: 0,
        offset: AbsoluteOffset::<f32>::default(),
        abort: Some(abort.abort_on_drop()),
        generation: state.generation,
        details: details::State::default(),
        images: images::State::default(),
        find: find::State::default(),
        exporting: false,
        outline_open: true,
        options_open: false,
    });
    Task::batch([
        task,
        iced::widget::operation::scroll_to("external-session-reader", AbsoluteOffset::<f32>::default()),
    ])
}

pub(super) fn view<'a>(state: &'a super::State, reader: &'a State, loopback: bool) -> Element<'a, Message> {
    let sessions = super::list::readable_sessions(state);
    let index = sessions.iter().position(|session| session.identity == reader.identity);
    let count = sessions.len();
    let session = state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == reader.identity));
    let mut header = row![
        button(text("返回列表")).padding([8, 12]).style(rounded_action_btn_style).on_press(Event::Back),
        super::list::provider_icon(reader.identity.provider, 24.0),
        container(text(&reader.title).size(18)).width(Length::Fill),
        button(text("上一个")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe(index.filter(|i| *i > 0).map(|_| Event::Adjacent(false))),
        button(text("下一个")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe(index.filter(|i| i + 1 < count).map(|_| Event::Adjacent(true))),
    ].spacing(8);
    if let Some(session) = session {
        header = header.push(super::resume::reader_control(session, state));
    }
    let mut actions = vec![super::list::Action::Refresh, super::list::Action::CopyId,
        super::list::Action::CopyPath, super::list::Action::CopyMarkdown,
        super::list::Action::ExportMarkdown, super::list::Action::Options];
    if session.is_some_and(|s| s.capabilities.delete) { actions.push(super::list::Action::Delete); }
    let identity = reader.identity.clone();
    let generation = state.generation;
    header = header.push(super::list::action_menu(actions, "更多", move |action| {
        use super::list::Action;
        match action {
            Action::Refresh => Event::Read(identity.clone()),
            Action::CopyId => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Identity)),
            Action::CopyPath => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Source)),
            Action::CopyMarkdown => Event::Export(super::export::Event::Start(super::export::Destination::Clipboard, false)),
            Action::ExportMarkdown => Event::Export(super::export::Event::Start(super::export::Destination::File, false)),
            Action::Delete => Event::Delete(super::delete::Event::Ask(Some(identity.clone()))),
            _ => Event::ReaderOptions,
        }
    }));
    let status = if reader.failed {
        "读取失败，内容不完整".to_owned()
    } else if reader.loading {
        format!("正在读取… {} / {} 条消息", reader.messages.len(), reader.total)
    } else {
        format!("已读取 {} 条消息", reader.messages.len())
    };
    let mut toolbar = row![button(text(if reader.outline_open { "收起目录" } else { "展开目录" }))
        .padding([8, 12]).style(rounded_action_btn_style).on_press(Event::ReaderOutline),
        button(text("显示与复制选项")).padding([8, 12]).style(rounded_action_btn_style).on_press(Event::ReaderOptions)].spacing(8);
    if let Some(session) = session {
        if let Some(dir) = &session.project_dir { toolbar = toolbar.push(text(dir).size(12)); }
        if session.last_active_at.or(session.created_at).is_some() {
            toolbar = toolbar.push(text(super::list::relative_time(session)).size(12));
        }
    }
    toolbar = toolbar.push(button(text("导出 Markdown")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe((!reader.exporting && session.is_some_and(|s| s.capabilities.export_markdown))
            .then(|| Event::Export(super::export::Event::Start(super::export::Destination::File, false)))));
    let mut content = column![header.wrap(), toolbar.wrap(), find::controls(reader),
        text(status).size(12).style(settings_muted_text_style)].spacing(8);
    if reader.options_open {
        content = content.push(button(text(if reader.details.process_open { "收起执行过程" } else { "展开执行过程" }))
                .padding([8, 12]).style(rounded_action_btn_style).on_press(Event::Details(details::Event::Process)))
            .push(super::export::controls(state, reader))
            .push(text("复制和导出不包含思考内容").size(12).style(settings_muted_text_style));
        if let Some(session) = session { content = content.push(super::paths::controls(state, session, loopback)); }
    }
    let mut workspace = row![
        scrollable(container(body(reader)).width(Length::Fill))
            .id("external-session-reader")
            .on_scroll(|v| message(Event::ReaderScrolled(v.absolute_offset())))
            .width(Length::FillPortion(3)).height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    ].spacing(12).height(Length::Fill);
    if reader.outline_open {
        workspace = workspace.push(scrollable(container(outline(reader)))
            .width(Length::FillPortion(1)).height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))));
    }
    let base = content.push(workspace).height(Length::Fill).into();
    images::overlay(base, reader)
}

pub(super) fn outline(reader: &State) -> Element<'_, Message> {
    let mut outline = column![text("轮次目录").size(15)].spacing(8).padding([0, 7]);
    for (i, turn) in reader.turns.iter().enumerate() {
        outline = outline.push(
            button(column![text(format!(
                "{}{}. {}",
                if i == reader.active { "● " } else { "" },
                i + 1,
                turn.question_preview.chars().take(80).collect::<String>()
            )), text(format!("{} 个步骤{}", turn.step_count, if turn.aborted { " · 已中止" } else { "" })).size(12),
                text(reply_preview(reader, turn)).size(12).style(settings_muted_text_style)])
            .padding([8, 12])
            .style(if i == reader.active { primary_action_btn_style } else { rounded_action_btn_style })
            .on_press(message(Event::Turn(i))),
        );
    }
    outline.into()
}

pub(super) fn body(reader: &State) -> Element<'_, Message> {
    let mut body = column![].spacing(12).padding([0, 7]);
    if let Some(hit) = find::focused(reader) {
        body = body.push(hit);
    }
    if let Some(turn) = reader.turns.get(reader.active) {
        body = body.push(text(format!(
            "第 {} 轮 · {} 个步骤 · {} 个错误{}",
            reader.active + 1,
            turn.step_count,
            turn.error_count,
            if turn.aborted { " · 已中止" } else { "" }
        )));
        // Render only the selected server-defined range; do not regroup turns in the UI.
        let messages: Vec<_> = reader
            .messages
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= turn.first_message_index && *i <= turn.last_message_index)
            .map(|(_, m)| m)
            .collect();
        if reader.find.filter != find::Filter::Changes {
            for question in messages.iter().filter(|m| m.role == "user" && !m.injected) {
                body = body.push(row![container(text("")).width(Length::FillPortion(1)), container(settings_group_card(
                    column![text("提问").size(14), details::message_view(reader, question)].spacing(6).max_width(640),
                )).width(Length::FillPortion(4))]);
            }
        }
        if reader.find.filter != find::Filter::Conversation {
            body = body.push(details::steps(reader, turn));
        }
        if reader.find.filter != find::Filter::Changes {
            if turn.has_final_reply {
                if let Some(reply) = messages.iter().rev().find(|m| {
                    reader.messages.len() > turn.last_message_index
                        && m.role == "assistant"
                        && !m.injected
                        && (!display_text(m).is_empty()
                            || m.blocks.iter().any(|b| !images::references(b).is_empty()))
                }) {
                    body = body.push(settings_group_card(
                        column![text("最终回复").size(14), details::message_view(reader, reply)]
                            .spacing(6),
                    ));
                } else {
                    body = body.push(text(if reader.failed { "回复尚未读到，内容不完整" } else { "正在读取回复…" }).style(settings_muted_text_style));
                }
            } else {
                body = body.push(text(if turn.aborted { "本轮已中止，没有最终回复" } else { "本轮没有最终回复" }).style(settings_muted_text_style));
            }
        }
    } else if !reader.loading {
        body = body.push(text("此会话没有可阅读轮次"));
    }
    body.into()
}

fn reply_preview(reader: &State, turn: &TurnIndex) -> String {
    if !turn.has_final_reply || reader.messages.len() <= turn.last_message_index {
        return if reader.failed { "内容不完整" } else if reader.loading { "正在读取…" }
            else if turn.aborted { "已中止" } else { "暂无回复" }.to_owned();
    }
    reader.messages.iter().enumerate().rev()
        .filter(|(i, m)| *i >= turn.first_message_index && *i <= turn.last_message_index
            && m.role == "assistant" && !m.injected)
        .map(|(_, m)| display_text(m))
        .find(|text| !text.trim().is_empty())
        .map(|text| text.chars().take(80).collect())
        .unwrap_or_else(|| if reader.loading { "正在读取…" } else if reader.failed { "内容不完整" } else { "暂无回复" }.to_owned())
}

fn display_text(message: &SessionMessage) -> String {
    if message.blocks.is_empty() {
        return message.content.clone();
    }
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            SessionBlock::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Rebuild the current authorized reader identity before dispatch and completion.
pub(super) fn current_scope(state: &super::State, connection: &Connection) -> Option<Scope> {
    let context = connection.context.clone();
    let reader = state.reader.as_ref()?;
    if !state.open
        || connection.catalog_context.as_ref() != Some(&context)
        || reader.identity.target_host != connection.host
        || Some(connection.project.as_str()) != Some(reader.identity.scope.project_id().as_ref())
    {
        return None;
    }
    Some(Scope {
        context,
        catalog: connection.generation,
        generation: reader.generation,
        identity: reader.identity.clone(),
    })
}

#[derive(Clone)]
pub struct State {
    pub identity: ExternalSessionIdentity,
    pub title: String,
    pub turns: Vec<TurnIndex>,
    pub messages: Vec<SessionMessage>,
    pub active: usize,
    pub loading: bool,
    pub failed: bool,
    pub total: usize,
    pub offset: AbsoluteOffset,
    pub abort: Option<iced::task::Handle>,
    pub generation: u64,
    pub details: details::State,
    pub images: images::State,
    pub find: find::State,
    pub exporting: bool,
    pub outline_open: bool,
    pub options_open: bool,
}
impl State {
    pub fn cancel(&self) {
        self.details.cancel();
        self.images.cancel();
        if let Some(handle) = &self.abort {
            handle.abort();
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub context: AgentAppsContext,
    pub catalog: u64,
    pub generation: u64,
    pub identity: ExternalSessionIdentity,
}

pub(super) fn navigate(state: &mut super::State, event: &Event) -> Option<Task<Event>> {
    if !matches!(event, Event::ReaderOutline | Event::ReaderOptions | Event::Find(_) | Event::ListScrolled(_) | Event::ReaderScrolled(_) | Event::Back | Event::Turn(_)) { return None; }
    match event.clone() {
        Event::ReaderOutline => { if let Some(reader) = &mut state.reader { reader.outline_open = !reader.outline_open; } }
        Event::ReaderOptions => { if let Some(reader) = &mut state.reader { reader.options_open = !reader.options_open; } }
        Event::Find(value) => {
            if let Some(reader) = &mut state.reader {
                find::update(reader, value);
                reader.offset = AbsoluteOffset::<f32>::default();
                return Some(iced::widget::operation::scroll_to(
                    "external-session-reader",
                    reader.offset,
                ));
            }
        }
        Event::ListScrolled(offset) => {
            state.list_offset = offset;
        }
        Event::ReaderScrolled(offset) => {
            if let Some(reader) = &mut state.reader {
                reader.offset = offset;
            }
        }
        Event::Back => {
            state.cancel_reader();
            return Some(iced::widget::operation::scroll_to(
                "external-session-list",
                state.list_offset,
            ));
        }
        Event::Turn(index) => {
            if let Some(reader) = &mut state.reader {
                if index < reader.turns.len() {
                    reader.details.cancel();
                    reader.details = details::State::default();
                    reader.images.cancel();
                    reader.images = images::State::default();
                    reader.active = index;
                    reader.offset = AbsoluteOffset::<f32>::default();
                    return Some(iced::widget::operation::scroll_to(
                        "external-session-reader",
                        reader.offset,
                    ));
                }
            }
        }
        _ => unreachable!(),
    }
    Some(Task::none())
}
