//! Scoped, cancellable history reading. Turn boundaries come from the host projection.
use crate::AgentAppsContext;
use super::{Event, details, find, images, message};
use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::Icon;
use desktop_ui_controls::{overlays::BelowOverlay, single_line_label::SingleLineLabel};
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
            | Event::ReaderSearch
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
        search_open: false,
    });
    Task::batch([
        task,
        iced::widget::operation::scroll_to("external-session-reader", AbsoluteOffset::<f32>::default()),
    ])
}

pub(super) fn view<'a>(state: &'a super::State, reader: &'a State, loopback: bool, i18n: desktop_i18n::I18n) -> Element<'a, Message> {
    let sessions = super::list::readable_sessions(state);
    let index = sessions.iter().position(|session| session.identity == reader.identity);
    let count = sessions.len();
    let session = state.page.as_ref().and_then(|page| page.sessions.iter().find(|s| s.identity == reader.identity));
    let mut header = row![
        icon_btn(Icon::ArrowLeft, "返回列表", Some(Event::Back)),
        super::list::provider_icon(reader.identity.provider, 21.0),
        container(SingleLineLabel::new(reader.title.clone(), 17.0).font(iced::Font {
            weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT
        })).width(Length::Fill),
        icon_btn(Icon::ChevronLeft, "上一个会话", index.filter(|i| *i > 0).map(|_| Event::Adjacent(false))),
        icon_btn(Icon::ChevronRight, "下一个会话", index.filter(|i| i + 1 < count).map(|_| Event::Adjacent(true))),
    ].spacing(8).align_y(iced::Alignment::Center);
    if let Some(session) = session {
        header = header.push(super::resume::reader_control(session, state));
    }
    use super::list::Action;
    let mut actions = Vec::new();
    if session.is_some_and(|s| s.capabilities.export_markdown) {
        actions.push(Action::CopyMarkdown);
        #[cfg(not(target_arch = "wasm32"))]
        actions.push(Action::ExportMarkdown);
    }
    actions.extend([Action::CopyId, Action::CopyPath, Action::Options, Action::Reload]);
    if session.is_some_and(|s| s.capabilities.delete) { actions.push(super::list::Action::Delete); }
    let identity = reader.identity.clone();
    let generation = state.generation;
    header = header.push(super::list::action_menu(actions, "更多", move |action| {
        use super::list::Action;
        match action {
            Action::Reload => Event::Read(identity.clone()),
            Action::CopyId => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Identity)),
            Action::CopyPath => Event::Path(super::paths::Event::Copy(identity.clone(), generation, super::paths::Location::Source)),
            Action::CopyMarkdown => Event::Export(super::export::Event::Start(super::export::Destination::Clipboard, false)),
            #[cfg(not(target_arch = "wasm32"))]
            Action::ExportMarkdown => Event::Export(super::export::Event::Start(super::export::Destination::File, false)),
            Action::Delete => Event::Delete(super::delete::Event::Ask(Some(identity.clone()))),
            _ => Event::ReaderOptions,
        }
    }));
    let mut metadata = row![].spacing(7).align_y(iced::Alignment::Center);
    if let Some(session) = session {
        if let Some(dir) = &session.project_dir {
            metadata = metadata.push(container(row![
                icon_svg(Icon::FileText, 13.0),
                container(SingleLineLabel::new(dir.clone(), 12.0)).width(Length::Fill),
            ].spacing(6).align_y(iced::Alignment::Center)).padding([4, 7])
                .max_width(280).width(Length::Fill).style(super::list::subtle_surface));
        }
        if let Some(at) = session.last_active_at.or(session.created_at).and_then(timestamp) {
            metadata = metadata.push(text(at).size(12).style(settings_muted_text_style));
        }
    }
    let mut options = column![
        button(text(if reader.details.process_open { "收起执行过程" } else { "展开执行过程" }).size(13))
            .padding([6, 9]).style(rounded_action_btn_style).on_press(Event::Details(details::Event::Process)),
        super::export::controls(state, reader),
    ].spacing(7);
    if let Some(session) = session { options = options.push(super::paths::controls(state, session, loopback)); }
    let options = BelowOverlay::new(
        icon_btn(Icon::Sliders, "显示与复制选项", Some(Event::ReaderOptions)),
        container(options).width(320).padding(10).style(settings_modal_card_style),
    ).show(reader.options_open).gap(5.0).on_close(Event::ReaderOptions).dismiss_on_escape();
    let toolbar = row![
        container(metadata).width(Length::Fill),
        icon_btn(Icon::Columns, if reader.outline_open { "收起对话目录" } else { "展开对话目录" }, Some(Event::ReaderOutline)),
        find::filters(reader),
        options,
    ];
    #[cfg(not(target_arch = "wasm32"))]
    let toolbar = toolbar.push(icon_btn(Icon::CloudDownload, "导出为 Markdown 文件",
        (!reader.exporting && session.is_some_and(|s| s.capabilities.export_markdown))
            .then(|| Event::Export(super::export::Event::Start(super::export::Destination::File, false)))));
    let toolbar = toolbar.push(icon_btn(Icon::Search, "查找会话内容", Some(Event::ReaderSearch)))
        .spacing(7).align_y(iced::Alignment::Center);
    let mut tools = column![toolbar].spacing(8);
    if reader.search_open {
        tools = tools.push(find::controls(reader));
    }
    if reader.failed {
        tools = tools.push(settings_error_banner("读取中断，当前内容可能不完整。"));
    } else if reader.loading {
        tools = tools.push(text("正在读取会话…").size(12).style(settings_muted_text_style));
    }
    let base = column![
        container(header).padding([5, 8]),
        settings_divider(),
        container(tools).padding([7, 8]),
        settings_divider(),
        iced::widget::responsive(move |size| workspace(reader, i18n, size.width < 640.0)),
    ].height(Length::Fill).into();
    images::overlay(base, reader)
}

fn workspace(reader: &State, i18n: desktop_i18n::I18n, narrow: bool) -> Element<'_, Message> {
    let scrollbars = || Direction::Vertical(Scrollbar::new().width(4).scroller_width(4));
    let transcript = scrollable(container(
        container(body(reader, i18n, narrow)).width(Length::Fill).max_width(900)
    ).center_x(Length::Fill).padding([0, 7]))
        .id("external-session-reader")
        .on_scroll(|v| Event::ReaderScrolled(v.absolute_offset()))
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbars());
    if !reader.outline_open {
        return transcript.into();
    }
    let directory = container(scrollable(outline(reader))
        .width(Length::Fill).height(Length::Fill).direction(scrollbars()))
        .style(outline_surface);
    if narrow {
        column![directory.height(136).width(Length::Fill), settings_divider(), transcript]
            .height(Length::Fill).into()
    } else {
        row![transcript, iced::widget::rule::vertical(1), directory.width(228).height(Length::Fill)]
            .height(Length::Fill).into()
    }
}

fn timestamp(ms: i64) -> Option<String> {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ms).single().map(|date| date.format("%m月%d日 %H:%M").to_string())
}

fn outline(reader: &State) -> Element<'_, Message> {
    let mut outline = column![row![
        text("对话目录").size(13).font(semibold()),
        text(reader.turns.len().to_string()).size(12).style(settings_muted_text_style),
    ].spacing(8).align_y(iced::Alignment::Center)]
        .spacing(8).padding([13, 7]).width(Length::Fill);
    for (i, turn) in reader.turns.iter().enumerate() {
        let title = if turn.question_preview.trim().is_empty() {
            format!("第 {} 轮", i + 1)
        } else {
            super::list::preview(&turn.question_preview, 120)
        };
        let active = i == reader.active;
        let preview = container(SingleLineLabel::new(reply_preview(reader, turn), 12.0))
            .style(muted_content).width(Length::Fill);
        let mut status = row![text(format!("{} 步", turn.step_count)).size(11)
            .style(settings_muted_text_style)].spacing(7);
        if turn.aborted || turn.error_count > 0 {
            status = status.push(text(if turn.aborted { "已中止" } else { "有错误" })
                .size(11).style(|theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                }));
        }
        let item = button(column![
            container(SingleLineLabel::new(title, 13.0).font(semibold())).width(Length::Fill),
            preview,
            status,
        ].spacing(4)).width(Length::Fill).padding([8, 10])
            .style(move |theme, status| {
                let mut style = super::list::flat_button(theme, status, active);
                style.border = iced::border::rounded(8);
                if active && status == button::Status::Active {
                    style.background = Some(theme.palette().text.scale_alpha(0.085).into());
                    style.border.width = 1.0;
                    style.border.color = theme.palette().text.scale_alpha(0.13);
                }
                style
            }).on_press(Event::Turn(i));
        outline = outline.push(item);
    }
    outline.into()
}

pub(super) fn semibold() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }
}

fn outline_surface(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.026).into()),
        ..Default::default()
    }
}

pub(super) fn muted_content(theme: &iced::Theme) -> container::Style {
    container::Style { text_color: settings_muted_text_style(theme).color, ..Default::default() }
}

fn question_bubble(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.105).into()),
        border: iced::Border { radius: iced::border::Radius::new(14).top_right(5), ..Default::default() },
        ..Default::default()
    }
}

fn body(reader: &State, i18n: desktop_i18n::I18n, narrow: bool) -> Element<'_, Message> {
    let query = reader.find.query.trim().to_lowercase();
    let mut body = column![].spacing(16).padding([16, 8]);
    if let Some(hit) = find::focused(reader, i18n) {
        body = body.push(hit);
    }
    let Some(turn) = reader.turns.get(reader.active) else {
        if !reader.loading {
            body = body.push(text("此会话没有可阅读轮次").style(settings_muted_text_style));
        }
        return body.into();
    };

    // Use the host-projected range, including partial streamed turns.
    let messages: Vec<_> = reader.messages.iter().enumerate()
        .filter(|(index, message)| !message.injected
            && *index >= turn.first_message_index && *index <= turn.last_message_index)
        .collect();
    let changes_only = reader.find.filter == find::Filter::Changes;
    let conversation_only = reader.find.filter == find::Filter::Conversation;
    if !changes_only {
        for (_, question) in messages.iter().copied().filter(|(_, m)| m.role == "user") {
            let mut bubble = column![].spacing(5).width(Length::Fill);
            if let Some(at) = question.ts.or(turn.ts).and_then(timestamp) {
                bubble = bubble.push(container(text(at).size(11).style(settings_muted_text_style))
                    .align_right(Length::Fill));
            }
            bubble = bubble.push(container(details::message_view(reader, question, &query, i18n))
                .width(Length::Fill).padding([10, 13]).style(question_bubble));
            let mut line = row![];
            if !narrow {
                line = line.push(iced::widget::space::horizontal().width(Length::FillPortion(1)));
            }
            body = body.push(line.push(container(
                container(bubble).width(Length::Fill).max_width(530)
            ).align_right(if narrow { Length::Fill } else { Length::FillPortion(3) })));
        }
    }

    let mut author = row![text(super::list::provider_name(reader.identity.provider)).size(14)
        .font(semibold())].spacing(8).align_y(iced::Alignment::Center);
    if let Some(at) = turn.ts.and_then(timestamp) {
        author = author.push(text(at).size(12).style(settings_muted_text_style));
    }
    if let Some(model) = messages.iter().rev().find_map(|(_, message)| {
        message.meta.as_ref()?.model.as_deref().filter(|model| !model.trim().is_empty())
    }) {
        author = author.push(container(SingleLineLabel::new(model.to_owned(), 12.0))
            .width(Length::Fill).style(muted_content));
    }
    let reply = if turn.has_final_reply && reader.messages.len() > turn.last_message_index {
        messages.iter().rev().copied().find(|(_, m)| m.role == "assistant"
            && (!display_text(m).is_empty() || m.blocks.iter().any(|b| !images::references(b).is_empty())))
    } else {
        None
    };

    let mut answer = column![container(author).center_y(27)].spacing(7).width(Length::Fill);
    if !conversation_only {
        answer = answer.push(details::steps(reader, turn, reply.map(|(index, _)| index), &query, i18n));
    }
    if !changes_only {
        if let Some((_, reply)) = reply {
            answer = answer.push(container(details::message_view(reader, reply, &query, i18n)).padding([3, 0]));
        } else {
            let status = if turn.has_final_reply {
                if reader.failed { "回复尚未读到，内容不完整" } else { "正在读取回复…" }
            } else if turn.aborted {
                "本轮已中止，没有最终回复"
            } else {
                "本轮没有最终回复"
            };
            answer = answer.push(text(status).size(12).style(settings_muted_text_style));
        }
    }
    let avatar = container(super::list::provider_icon(reader.identity.provider, 17.0))
        .center_x(27).center_y(27).style(|theme: &iced::Theme| container::Style {
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.16),
                radius: 14.0.into(),
            },
            ..Default::default()
        });
    body.push(row![avatar, answer].spacing(9)).into()
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
    pub search_open: bool,
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
    if !matches!(event, Event::ReaderOutline | Event::ReaderOptions | Event::ReaderSearch | Event::Find(_) | Event::ListScrolled(_) | Event::ReaderScrolled(_) | Event::Back | Event::Turn(_)) { return None; }
    match event.clone() {
        Event::ReaderOutline => { if let Some(reader) = &mut state.reader { reader.outline_open = !reader.outline_open; } }
        Event::ReaderOptions => { if let Some(reader) = &mut state.reader { reader.options_open = !reader.options_open; } }
        Event::ReaderSearch => {
            if let Some(reader) = &mut state.reader {
                reader.search_open = !reader.search_open;
                if reader.search_open {
                    return Some(iced::widget::operation::focus(find::INPUT_ID));
                }
                find::update(reader, find::Event::Query(String::new()));
            }
        }
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
