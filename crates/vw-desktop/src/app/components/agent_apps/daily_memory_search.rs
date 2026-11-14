//! Ephemeral daily search. Dropping the page signals cancellation to the SDK worker.
use super::{
    daily_memory::{self, Scope},
    system_settings_common::*,
};
use crate::app::Message;
use iced::{
    Element, Task,
    widget::{button, column, row, text},
};
use vw_gateway_client::{
    GatewayClient, WorkspaceMemoryScope,
    vw_api_types::{task::TaskStatus, workspace_memory::DailyMemorySearchResult},
};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    query: String,
    request: u64,
    searching: bool,
    results: Vec<DailyMemorySearchResult>,
    error: Option<String>,
    cancel: Option<tokio::sync::watch::Sender<()>>,
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Query(String),
    Finished(Scope, u64, Result<Vec<DailyMemorySearchResult>, String>),
}
impl State {
    pub(super) fn close(&mut self) {
        self.cancel.take();
        *self = Self { request: self.request.wrapping_add(1), ..Self::default() };
    }
    pub(super) fn active(&self) -> bool {
        self.open && !self.query.trim().is_empty()
    }
    fn accept(&mut self, request: u64, result: Result<Vec<DailyMemorySearchResult>, String>) {
        if !self.active() || request != self.request {
            return;
        }
        self.searching = false;
        self.cancel.take();
        match result {
            Ok(results) => {
                self.results = results;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}
fn message(event: Event) -> Message {
    daily_memory::message(daily_memory::Event::Search(event))
}
pub(super) fn controls(state: &State) -> Element<'_, Message> {
    if !state.open {
        return button(text("搜索每日记忆"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::Open))
            .into();
    }
    row![
        settings_text_input("搜索日期或内容", &state.query)
            .id("daily-memory-search")
            .on_input(|q| message(Event::Query(q))),
        button(text("取消搜索"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(message(Event::Close))
    ]
    .spacing(8)
    .into()
}
pub(super) fn results(state: &State, busy: bool) -> Element<'_, Message> {
    let mut body = column![].spacing(8);
    if state.searching {
        body = body.push(text("正在搜索…"));
    } else if let Some(error) = &state.error {
        body = body.push(text(error));
    } else if state.results.is_empty() {
        body = body.push(text("没有匹配的每日记忆"));
    } else {
        for result in &state.results {
            body = body.push(
                column![
                    button(text(format!("{} · {} 次匹配", result.date, result.match_count)))
                        .padding([8, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!busy).then(|| daily_memory::message(
                            daily_memory::Event::Select(result.filename.clone())
                        ))),
                    text(&result.snippet)
                ]
                .spacing(4),
            );
        }
    }
    body.into()
}
pub(super) fn update(
    state: &mut State,
    event: Event,
    client: GatewayClient,
    scope: Scope,
    query: WorkspaceMemoryScope,
    token: String,
) -> Task<Message> {
    match event {
        Event::Open => {
            state.open = true;
            return iced::widget::text_input::focus("daily-memory-search");
        }
        Event::Close => {
            state.close();
            return Task::none();
        }
        Event::Finished(_, request, result) => {
            state.accept(request, result);
            return Task::none();
        }
        Event::Query(value) => {
            state.query = value;
        }
    }
    state.cancel.take();
    state.request = state.request.wrapping_add(1);
    state.results.clear();
    state.error = None;
    state.searching = state.active();
    if !state.searching {
        return Task::none();
    }
    let request = state.request;
    let term = state.query.trim().to_owned();
    let (sender, mut cancel) = tokio::sync::watch::channel(());
    state.cancel = Some(sender);
    Task::perform(
        async move {
            let result = async {
                tokio::select! {
                    _ = cancel.changed() => return Err("搜索已取消".into()),
                    _ = tokio::time::sleep(std::time::Duration::from_millis(300)) => {}
                }
                // Let submission finish even after cancellation so its operation can be released.
                let (mut search, mut response) = client
                    .search_daily_memory_files(&query, term, &token)
                    .await
                    .map_err(|e| e.to_string())?;
                let outcome = loop {
                    if cancel.has_changed().is_err() {
                        break Err("搜索已取消".into());
                    }
                    match response.progress.status {
                        TaskStatus::Completed => {
                            break response.results.take().ok_or_else(|| "搜索响应缺少结果".into());
                        }
                        TaskStatus::Failed => break Err("搜索失败，请修改关键词重试".into()),
                        TaskStatus::Cancelled => break Err("搜索已取消".into()),
                        _ => {}
                    }
                    tokio::select! {
                        _ = cancel.changed() => break Err("搜索已取消".into()),
                        _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {}
                    }
                    let polled = tokio::select! {
                        _ = cancel.changed() => break Err("搜索已取消".into()),
                        result = search.poll(&token) => result,
                    };
                    match polled {
                        Ok(next) => response = next,
                        Err(e) => break Err(e.to_string()),
                    }
                };
                // Release server-side retained snippets on every terminal path.
                if search.cancel(&token).await.is_err() && outcome.is_ok() {
                    return Err("搜索结果释放失败，请重试".into());
                }
                outcome
            }
            .await;
            message(Event::Finished(scope, request, result))
        },
        |m| m,
    )
}
