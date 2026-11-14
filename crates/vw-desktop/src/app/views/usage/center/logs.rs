//! Server-paged ledger requests. Column preferences never change the query.
use super::{Identity, identity, Event as CenterEvent};
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task, Length, widget::{button, column, row, text, container, scrollable}};
use vw_gateway_client::{UsageLogResult, UsageLogDetail, UsageReadScope};
use vw_gateway_client::vw_api_types::usage::*;

#[derive(Default)]
pub struct State {
    ticket: u64,
    detail_ticket: u64,
    pub query: Option<UsageQuery>,
    cursor_history: Vec<Option<String>>,
    cursor: Option<String>,
    loading: bool,
    result: Option<Result<UsageLogResult, String>>,
    detail: Option<Result<UsageLogDetail, String>>,
    detail_open: bool,
    columns: [bool; 4],
    initialized: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Reload, Next, Previous, Toggle(usize), Open(String), CloseDetail,
    Loaded(u64, u64, Identity, UsageQuery, Result<UsageLogResult, String>),
    DetailLoaded(u64, u64, Identity, UsageQuery, Result<UsageLogDetail, String>),
}
fn message(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(CenterEvent::Logs(event))) }

/// Called by the shared filter loader, with its exact query snapshot.
pub fn reset(app: &mut App, query: UsageQuery) -> Task<Message> {
    let state = &mut app.usage_center.logs;
    state.query = Some(query);
    state.cursor = None;
    state.cursor_history.clear();
    state.detail_open = false;
    state.detail = None;
    update(app, Event::Reload)
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let generation = app.usage_center.sequence;
    let scope = identity(app);
    let active = app.usage_center.open && app.screen == crate::app::Screen::Usage;
    let state = &mut app.usage_center.logs;
    match event {
        Event::Toggle(index) => { if let Some(value) = state.columns.get_mut(index) { *value = !*value; } }
        Event::CloseDetail => { state.detail_open = false; state.detail_ticket = state.detail_ticket.wrapping_add(1); }
        Event::Loaded(ticket, seq, expected, query, result) => {
            if active && generation == seq && scope.as_ref() == Some(&expected)
                && state.ticket == ticket && state.query.as_ref() == Some(&query) {
                state.loading = false;
                state.result = Some(result);
            }
        }
        Event::DetailLoaded(ticket, seq, expected, query, result) => {
            if active && generation == seq && scope.as_ref() == Some(&expected)
                && state.detail_ticket == ticket && state.detail_open && state.query.as_ref() == Some(&query) {
                state.detail = Some(result);
            }
        }
        Event::Open(id) => {
            if !active { return Task::none(); }
            let Some(scope) = scope else { return Task::none(); };
            let Some(query) = state.query.clone() else { return Task::none(); };
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
            let ticket = state.detail_ticket;
            state.detail_open = true;
            state.detail = None;
            let client = crate::app::config::gateway_client();
            return Task::perform(async move {
                let result = match client {
                    Ok(client) => client.usage_log_detail(&read_scope(&scope), &query, &id).await.map_err(|e| e.to_string()),
                    Err(error) => Err(error),
                };
                (scope, query, result)
            }, move |(scope, query, result)| message(Event::DetailLoaded(ticket, generation, scope, query, result)));
        }
        Event::Reload | Event::Next | Event::Previous => {
            if !active { return Task::none(); }
            if state.loading && !matches!(event, Event::Reload) { return Task::none(); }
            if matches!(event, Event::Next) {
                let next = state.result.as_ref().and_then(|r| r.as_ref().ok()).and_then(|r| r.page.next_cursor.clone());
                let Some(next) = next else { return Task::none(); };
                state.cursor_history.push(state.cursor.clone());
                state.cursor = Some(next);
            } else if matches!(event, Event::Previous) {
                let Some(cursor) = state.cursor_history.pop() else { return Task::none(); };
                state.cursor = cursor;
            }
            if !state.initialized { state.columns = [true; 4]; state.initialized = true; }
            state.ticket = state.ticket.wrapping_add(1);
            let ticket = state.ticket;
            state.result = None;
            let Some(scope) = scope else {
                state.loading = false;
                state.result = Some(Err("请选择项目与执行主机".into()));
                return Task::none();
            };
            let Some(query) = state.query.clone() else { return Task::none(); };
            let mut page_query = query.clone();
            page_query.cursor = state.cursor.clone();
            page_query.limit = Some(20);
            state.loading = true;
            let client = crate::app::config::gateway_client();
            return Task::perform(async move {
                let result = match client {
                    Ok(client) => client.usage_logs(&read_scope(&scope), &page_query).await.map_err(|e| e.to_string()),
                    Err(error) => Err(error),
                };
                (scope, query, result)
            }, move |(scope, query, result)| message(Event::Loaded(ticket, generation, scope, query, result)));
        }
    }
    Task::none()
}
fn read_scope(scope: &Identity) -> UsageReadScope {
    UsageReadScope { project_id: scope.project.clone().into(), target_host: scope.host.clone(), time_zone: "UTC".into() }
}
fn unknown(value: Option<&str>) -> String { value.unwrap_or("未知").to_owned() }
pub fn cost(value: &UsageCost) -> String {
    let source = match value.source { UsageCostSource::Unknown => "未知", UsageCostSource::Native => "原生", UsageCostSource::Calculated => "计算", UsageCostSource::Estimated => "估算" };
    format!("{} USD（{source}）", unknown(value.amount_usd.as_deref()))
}
pub fn metric(value: Option<&UsageMeasurement>, unit: &str) -> String {
    value.map(|v| format!("{} {unit}（{}）", v.value, match v.kind { UsageMeasurementKind::Measured => "实测", UsageMeasurementKind::Estimated => "估算" })).unwrap_or_else(|| "未知".into())
}
fn action(label: impl Into<String>, event: Event) -> Element<'static, Message> {
    button(text(label.into()).size(13)).padding([6, 10])
        .style(crate::app::components::system_settings_common::rounded_action_btn_style).on_press(message(event)).into()
}
fn scroll<'a>(content: iced::widget::Column<'a, Message>) -> Element<'a, Message> {
    scrollable(container(content).padding([0, 7])).direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4))).height(Length::Fixed(320.0)).into()
}
pub fn view(app: &App) -> Element<'_, Message> {
    let state = &app.usage_center.logs;
    let mut columns = row![text("显示列")].spacing(8);
    for (index, label) in ["供应商", "模型", "费用", "延迟"].iter().enumerate() {
        columns = columns.push(action(format!("{} {label}", if state.columns[index] { "✓" } else { "○" }), Event::Toggle(index)));
    }
    let mut rows = column![].spacing(8);
    if state.loading { rows = rows.push(text("正在加载请求…")); }
    match &state.result {
        Some(Err(error)) => rows = rows.push(text(format!("请求加载失败：{error}"))),
        Some(Ok(result)) => {
            rows = rows.push(text(format!("共 {} 条请求 · 第 {} 页 · 每页20条 · 时间倒序", result.counts.total_requests, state.cursor_history.len() + 1)));
            if result.page.items.is_empty() { rows = rows.push(text("暂无请求")); }
            for detail in &result.page.items {
                let record = &detail.record;
                let mut values = vec![format!("{} · {:?}", record.started_at_unix_seconds.0, record.status)];
                if state.columns[0] { values.push(match &record.provider { UsageProvider::Unknown => "未知供应商".into(), UsageProvider::Known { provider_id } => provider_id.clone() }); }
                if state.columns[1] { values.push(unknown(record.models.response_model.as_deref().or(record.models.request_model.as_deref()))); }
                if state.columns[2] { values.push(cost(&record.cost)); }
                if state.columns[3] { values.push(metric(record.performance.latency_ms.as_ref(), "ms")); }
                rows = rows.push(action(values.join(" · "), Event::Open(record.id.clone())));
            }
        }
        None => {}
    }
    let base = column![text("请求列表").size(18), columns, scroll(rows), row![action("上一页", Event::Previous), action("下一页", Event::Next), action("刷新 / 重试", Event::Reload)].spacing(8)].spacing(8);
    if !state.detail_open { return base.into(); }
    use crate::app::components::system_settings_common::{settings_modal_overlay, settings_modal_card, settings_close_button};
    let body = match &state.detail {
        None => "正在加载详情…".into(),
        Some(Err(error)) => format!("请求详情加载失败：{error}"),
        Some(Ok(detail)) => detail_text(detail),
    };
    let card = settings_modal_card(column![row![text("请求详情").size(20), settings_close_button(message(Event::CloseDetail))].spacing(16), scroll(column![text(body)]), action("关闭", Event::CloseDetail)].spacing(12)).width(Length::Fixed(720.0));
    settings_modal_overlay(Some(base.into()), message(Event::CloseDetail), card)
}
fn detail_text(detail: &UsageLogDetail) -> String {
    let r = &detail.record;
    let tokens = r.tokens.as_ref().map(|t| {
        let count = |v: Option<u64>| v.map(|n| n.to_string()).unwrap_or_else(|| "未知".into());
        format!("输入 {} · 输出 {} · 推理 {}\n缓存读取 {} · 创建 {} · 创建1h子集 {}\n输入语义 {:?} · 推理语义 {:?}", count(t.input), count(t.output), count(t.reasoning), count(t.cache_read), count(t.cache_creation), count(t.cache_creation_1h), t.input_semantics, t.reasoning_semantics)
    }).unwrap_or_else(|| "Token 未知".into());
    format!("请求 {}\n时间 {}（UTC Unix秒）\n供应商 {:?}\n状态 {:?} · HTTP {} · 计费 {}\n错误 {}\n请求模型 {}\n响应模型 {}\n定价模型 {}\n倍率 {} · 依据 {}\n费用 {}\n{}\n首字 {}\n延迟 {}\n速度 {}\n来源 {:?}\n疑似重复 {}", r.id, r.started_at_unix_seconds.0, r.provider, r.status, detail.status_code.map(|n| n.to_string()).unwrap_or_else(|| "未知".into()), r.billable, unknown(detail.error_message.as_deref()), unknown(r.models.request_model.as_deref()), unknown(r.models.response_model.as_deref()), unknown(r.models.pricing_model.as_deref()), unknown(r.models.pricing_multiplier.as_deref()), unknown(r.models.multiplier_basis.as_deref()), cost(&r.cost), tokens, metric(r.performance.first_token_ms.as_ref(), "ms"), metric(r.performance.latency_ms.as_ref(), "ms"), metric(r.performance.output_tokens_per_second.as_ref(), "token/s"), r.sources, unknown(r.suspected_duplicate_of.as_deref()))
}
