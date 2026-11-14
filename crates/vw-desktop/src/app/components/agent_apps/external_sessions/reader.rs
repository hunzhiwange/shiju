//! Scoped, cancellable history reading. Turn boundaries come from the host projection.
use super::{Event, details, find, images, message};
use crate::app::{App, Message, components::system_settings_common::*, state::AgentAppsContext};
use iced::widget::scrollable::{AbsoluteOffset, Direction, Scrollbar};
use iced::{
    Element, Length, Task,
    futures::SinkExt,
    widget::{button, column, container, row, scrollable, text},
};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone)]
pub(super) struct State {
    pub(super) identity: ExternalSessionIdentity,
    title: String,
    pub(super) turns: Vec<TurnIndex>,
    pub(super) messages: Vec<SessionMessage>,
    pub(super) active: usize,
    loading: bool,
    failed: bool,
    total: usize,
    offset: AbsoluteOffset,
    abort: Option<iced::task::Handle>,
    generation: u64,
    pub(super) details: details::State,
    pub(super) images: images::State,
    pub(super) find: find::State,
}
impl State {
    pub(super) fn cancel(&self) {
        self.details.cancel();
        self.images.cancel();
        if let Some(handle) = &self.abort {
            handle.abort();
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    context: AgentAppsContext,
    catalog: u64,
    generation: u64,
    identity: ExternalSessionIdentity,
}

pub(super) fn handles(event: &Event) -> bool {
    matches!(
        event,
        Event::Find(_)
            | Event::Read(_)
            | Event::Back
            | Event::Adjacent(_)
            | Event::Turn(_)
            | Event::ListScrolled(_)
            | Event::ReaderScrolled(_)
            | Event::Chunk(..)
    )
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Find(value) => {
            if let Some(reader) = &mut app.apps_catalog.external_sessions.reader {
                find::update(reader, value);
                reader.offset = AbsoluteOffset::default();
                return iced::widget::operation::scroll_to(
                    "external-session-reader",
                    reader.offset,
                );
            }
        }
        Event::ListScrolled(offset) => {
            app.apps_catalog.external_sessions.list_offset = offset;
        }
        Event::ReaderScrolled(offset) => {
            if let Some(reader) = &mut app.apps_catalog.external_sessions.reader {
                reader.offset = offset;
            }
        }
        Event::Back => {
            app.apps_catalog.external_sessions.cancel_reader();
            return iced::widget::operation::scroll_to(
                "external-session-list",
                app.apps_catalog.external_sessions.list_offset,
            );
        }
        Event::Turn(index) => {
            if let Some(reader) = &mut app.apps_catalog.external_sessions.reader {
                if index < reader.turns.len() {
                    reader.details.cancel();
                    reader.details = details::State::default();
                    reader.images.cancel();
                    reader.images = images::State::default();
                    reader.active = index;
                    reader.offset = AbsoluteOffset::default();
                    return iced::widget::operation::scroll_to(
                        "external-session-reader",
                        reader.offset,
                    );
                }
            }
        }
        Event::Adjacent(next) => {
            let state = &app.apps_catalog.external_sessions;
            let Some(reader) = &state.reader else {
                return Task::none();
            };
            let Some(page) = &state.page else {
                return Task::none();
            };
            let Some(index) =
                page.sessions.iter().position(|session| session.identity == reader.identity)
            else {
                return Task::none();
            };
            let index = if next { index.checked_add(1) } else { index.checked_sub(1) };
            if let Some(session) = index.and_then(|i| page.sessions.get(i)) {
                return update(app, Event::Read(session.identity.clone()));
            }
        }
        Event::Read(identity) => return start(app, identity),
        Event::Chunk(scope, result) => {
            let Ok(client) = crate::app::config::gateway_client() else {
                return Task::none();
            };
            let context = AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: client.endpoint().tenant_scope().clone(),
                project: app.project_path.clone(),
            };
            if scope.context != context
                || app.apps_catalog.context.as_ref() != Some(&context)
                || scope.catalog != app.apps_catalog.generation
                || scope.identity.target_host != app.apps_catalog.host
                || app.project_id.as_deref() != Some(scope.identity.scope.project_id().as_ref())
            {
                return Task::none();
            }
            let Some(reader) = &mut app.apps_catalog.external_sessions.reader else {
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
                    return app.show_error_toast("会话读取失败，已加载内容保留，请返回列表重试");
                }
            }
        }
        _ => {}
    }
    Task::none()
}

fn start(app: &mut App, identity: ExternalSessionIdentity) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，请重新连接");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let state = &mut app.apps_catalog.external_sessions;
    let session =
        state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == identity));
    let Some(session) = session.filter(|s| s.capabilities.read) else {
        return app.show_error_toast("当前会话不支持读取");
    };
    if !state.open
        || app.apps_catalog.context.as_ref() != Some(&context)
        || identity.target_host != app.apps_catalog.host
        || app.apps_catalog.token.is_empty()
        || app.project_id.as_deref() != Some(identity.scope.project_id().as_ref())
    {
        return app.show_error_toast("会话作用域已变化，请刷新列表");
    }
    let title = session.title.clone().unwrap_or_else(|| identity.session_id.0.clone());
    state.cancel_reader();
    state.busy = false;
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope {
        context,
        catalog: app.apps_catalog.generation,
        generation: state.generation,
        identity: identity.clone(),
    };
    let request = SessionStreamRequest {
        identity: identity.clone(),
        read_id: format!("{:032x}", rand::random::<u128>()),
    };
    let token = app.apps_catalog.token.clone();
    #[cfg(not(target_arch = "wasm32"))]
    let task = Task::run(
        iced::stream::channel(1, async move |mut output| {
            match client.external_session_stream(&request, &token).await {
                Ok(mut stream) => loop {
                    let chunk = stream.next_chunk().await;
                    let terminal = !matches!(
                        &chunk,
                        Ok(Some(TranscriptChunk::Header { .. } | TranscriptChunk::Messages { .. }))
                    );
                    let result = match chunk {
                        Ok(Some(chunk)) => Ok(chunk),
                        Ok(None) => break,
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
        offset: AbsoluteOffset::default(),
        abort: Some(abort.abort_on_drop()),
        generation: state.generation,
        details: details::State::default(),
        images: images::State::default(),
        find: find::State::default(),
    });
    Task::batch([
        task,
        iced::widget::operation::scroll_to("external-session-reader", AbsoluteOffset::default()),
    ])
}

pub(super) fn view<'a>(state: &'a super::State, reader: &'a State) -> Element<'a, Message> {
    let index = state
        .page
        .as_ref()
        .and_then(|page| page.sessions.iter().position(|s| s.identity == reader.identity));
    let count = state.page.as_ref().map_or(0, |page| page.sessions.len());
    let header = row![
        button(text("返回列表"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::Back)),
        text(&reader.title).size(18),
        button(text("上一个"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(index.filter(|i| *i > 0).map(|_| message(Event::Adjacent(false)))),
        button(text("下一个")).padding([8, 12]).style(rounded_action_btn_style).on_press_maybe(
            index.filter(|i| i + 1 < count).map(|_| message(Event::Adjacent(true)))
        ),
    ]
    .spacing(8)
    .wrap();
    let mut outline = column![text("轮次目录").size(15)].spacing(8).padding([0, 7]);
    for (i, turn) in reader.turns.iter().enumerate() {
        outline = outline.push(
            button(text(format!(
                "{}{}. {}",
                if i == reader.active { "● " } else { "" },
                i + 1,
                turn.question_preview.chars().take(80).collect::<String>()
            )))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::Turn(i))),
        );
    }
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
                body = body.push(settings_group_card(
                    column![text("提问").size(14), details::message_view(reader, question)]
                        .spacing(6),
                ));
            }
            if turn.has_final_reply {
                if let Some(reply) = messages.iter().rev().find(|m| {
                    m.role == "assistant"
                        && !m.injected
                        && (!display_text(m).is_empty()
                            || m.blocks.iter().any(|b| !images::references(b).is_empty()))
                }) {
                    body = body.push(settings_group_card(
                        column![text("最终回复").size(14), details::message_view(reader, reply)]
                            .spacing(6),
                    ));
                } else {
                    body = body.push(text("正在读取回复…").style(settings_muted_text_style));
                }
            } else {
                body = body.push(text("本轮没有最终回复").style(settings_muted_text_style));
            }
        }
        if reader.find.filter != find::Filter::Conversation {
            body = body.push(details::steps(reader, turn));
        }
    } else if !reader.loading {
        body = body.push(text("此会话没有可阅读轮次"));
    }
    let status = if reader.failed {
        "读取失败，内容不完整".to_owned()
    } else if reader.loading {
        format!("正在读取… {} / {} 条消息", reader.messages.len(), reader.total)
    } else {
        format!("已读取 {} 条消息", reader.messages.len())
    };
    let base = column![
        header,
        super::paths::source_button(),
        state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == reader.identity))
            .map(|session| super::paths::controls(state, session))
            .unwrap_or_else(|| text("来源位置不可用").into()),
        state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == reader.identity))
            .map(|session| super::resume::control(session, !state.busy && !state.deletion.busy()))
            .unwrap_or_else(|| text("会话已不在当前目录，请刷新").into()),
        find::controls(reader),
        text(status).style(settings_muted_text_style),
        row![
            scrollable(container(outline))
                .width(Length::FillPortion(1))
                .height(Length::Fixed(420.0))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
            scrollable(container(body).width(Length::Fill))
                .id("external-session-reader")
                .on_scroll(|v| message(Event::ReaderScrolled(v.absolute_offset())))
                .width(Length::FillPortion(3))
                .height(Length::Fixed(420.0))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(12)
    ]
    .spacing(10)
    .into();
    images::overlay(base, reader)
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
pub(super) fn current_scope(app: &App) -> Option<Scope> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let state = &app.apps_catalog.external_sessions;
    let reader = state.reader.as_ref()?;
    if !state.open
        || app.apps_catalog.context.as_ref() != Some(&context)
        || app.apps_catalog.token.is_empty()
        || reader.identity.target_host != app.apps_catalog.host
        || app.project_id.as_deref() != Some(reader.identity.scope.project_id().as_ref())
    {
        return None;
    }
    Some(Scope {
        context,
        catalog: app.apps_catalog.generation,
        generation: reader.generation,
        identity: reader.identity.clone(),
    })
}
