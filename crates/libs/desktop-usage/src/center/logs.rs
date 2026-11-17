//! Server-paged ledger requests. Column preferences never change the query.
use crate::Identity;

use gateway_usage_read::{UsageLogDetail, UsageLogResult, UsageReadScope};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, scrollable, text},
};
use vw_api_usage::*;

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
    columns: super::log_columns::Preferences,
    columns_open: bool,
    preference_error: Option<String>,
    page_size: u32,
    status_code_draft: String,
    status_code_error: Option<String>,
    initialized: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Reload,
    Next,
    Previous,
    Toggle(usize),
    Columns,
    StatusCode(String),
    ApplyStatusCode,
    Status(Option<UsageRequestStatus>),
    PageSize(u32),
    Open(String),
    CloseDetail,
    Loaded(u64, u64, Identity, UsageQuery, Result<UsageLogResult, String>),
    DetailLoaded(u64, u64, Identity, UsageQuery, Result<UsageLogDetail, String>),
}
fn message(event: Event) -> Event {
    event
}

/// Called by the shared filter loader, with its exact query snapshot.
pub fn reset(
    state: &mut State,
    current: Option<Identity>,
    visible: bool,
    open: bool,
    generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    query: UsageQuery,
) -> Task<Event> {
    state.status_code_draft =
        query.filter.status_code.map(|code| code.to_string()).unwrap_or_default();
    state.query = Some(query);
    state.cursor = None;
    state.cursor_history.clear();
    state.detail_open = false;
    state.detail_ticket = state.detail_ticket.wrapping_add(1);
    state.detail = None;
    update(state, current, visible, open, generation, client, Event::Reload)
}

pub fn update(
    state: &mut State,
    current: Option<Identity>,
    visible: bool,
    open: bool,
    generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    event: Event,
) -> Task<Event> {
    let scope = current.clone();
    let active = open && visible;

    match event {
        Event::Toggle(index) => {
            state.columns.toggle(index);
            state.preference_error = state.columns.save().err();
        }
        Event::StatusCode(value) => state.status_code_draft = value,
        Event::ApplyStatusCode => {
            let value = state.status_code_draft.trim();
            let code = if value.is_empty() {
                None
            } else {
                match value.parse::<u16>().ok().filter(|code| (100..=599).contains(code)) {
                    Some(code) => Some(code),
                    None => {
                        state.status_code_error = Some("状态码须为100–599，留空查询全部".into());
                        return Task::none();
                    }
                }
            };
            state.status_code_error = None;
            let Some(mut query) = state.query.clone() else {
                return Task::none();
            };
            query.filter.status_code = code;
            return reset(state, current, visible, open, generation, client, query);
        }
        Event::Columns => state.columns_open = !state.columns_open,
        Event::Status(status) => {
            let Some(mut query) = state.query.clone() else {
                return Task::none();
            };
            query.filter.status = status;
            return reset(state, current, visible, open, generation, client, query);
        }
        Event::PageSize(size) => {
            if ![10, 20, 50, 100].contains(&size) {
                return Task::none();
            }
            state.page_size = size;
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            return reset(state, current, visible, open, generation, client, query);
        }
        Event::CloseDetail => {
            state.detail_open = false;
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
        }
        Event::Loaded(ticket, seq, expected, query, result) => {
            if active
                && generation == seq
                && scope.as_ref() == Some(&expected)
                && state.ticket == ticket
                && state.query.as_ref() == Some(&query)
            {
                state.loading = false;
                state.result = Some(result);
            }
        }
        Event::DetailLoaded(ticket, seq, expected, query, result) => {
            if active
                && generation == seq
                && scope.as_ref() == Some(&expected)
                && state.detail_ticket == ticket
                && state.detail_open
                && state.query.as_ref() == Some(&query)
            {
                state.detail = Some(result);
            }
        }
        Event::Open(id) => {
            if !active {
                return Task::none();
            }
            let Some(scope) = scope else {
                return Task::none();
            };
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
            let ticket = state.detail_ticket;
            state.detail_open = true;
            state.detail = None;
            let client = client.map(|client| {
                gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone())
            });
            return Task::perform(
                async move {
                    let result = match client {
                        Ok(client) => client
                            .usage_log_detail(&read_scope(&scope), &query, &id)
                            .await
                            .map_err(|e| e.to_string()),
                        Err(error) => Err(error),
                    };
                    (scope, query, result)
                },
                move |(scope, query, result)| {
                    message(Event::DetailLoaded(ticket, generation, scope, query, result))
                },
            );
        }
        Event::Reload | Event::Next | Event::Previous => {
            if !active {
                return Task::none();
            }
            if state.loading && !matches!(event, Event::Reload) {
                return Task::none();
            }
            if matches!(event, Event::Next) {
                let next = state
                    .result
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .and_then(|r| r.page.next_cursor.clone());
                let Some(next) = next else {
                    return Task::none();
                };
                state.cursor_history.push(state.cursor.clone());
                state.cursor = Some(next);
            } else if matches!(event, Event::Previous) {
                let Some(cursor) = state.cursor_history.pop() else {
                    return Task::none();
                };
                state.cursor = cursor;
            }
            if !state.initialized {
                match super::log_columns::Preferences::load() {
                    Ok(columns) => state.columns = columns,
                    Err(error) => state.preference_error = Some(error),
                }
                state.page_size = 20;
                state.initialized = true;
            }
            state.detail_open = false;
            state.detail = None;
            state.detail_ticket = state.detail_ticket.wrapping_add(1);
            state.ticket = state.ticket.wrapping_add(1);
            let ticket = state.ticket;
            state.result = None;
            let Some(scope) = scope else {
                state.loading = false;
                state.result = Some(Err("请选择项目与执行主机".into()));
                return Task::none();
            };
            let Some(query) = state.query.clone() else {
                return Task::none();
            };
            let mut page_query = query.clone();
            page_query.cursor = state.cursor.clone();
            page_query.limit = Some(state.page_size);
            state.loading = true;
            let client = client.map(|client| {
                gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone())
            });
            return Task::perform(
                async move {
                    let result = match client {
                        Ok(client) => client
                            .usage_logs(&read_scope(&scope), &page_query)
                            .await
                            .map_err(|e| e.to_string()),
                        Err(error) => Err(error),
                    };
                    (scope, query, result)
                },
                move |(scope, query, result)| {
                    message(Event::Loaded(ticket, generation, scope, query, result))
                },
            );
        }
    }
    Task::none()
}
fn read_scope(scope: &Identity) -> UsageReadScope {
    UsageReadScope {
        project_id: scope.project.clone().into(),
        target_host: scope.host.clone(),
        time_zone: "UTC".into(),
    }
}
fn unknown(value: Option<&str>) -> String {
    value.unwrap_or("未知").to_owned()
}
pub fn cost(value: &UsageCost) -> String {
    let source = match value.source {
        UsageCostSource::Unknown => "未知",
        UsageCostSource::Native => "原生",
        UsageCostSource::Calculated => "计算",
        UsageCostSource::Estimated => "估算",
    };
    format!("{} USD（{source}）", unknown(value.amount_usd.as_deref()))
}
pub fn metric(value: Option<&UsageMeasurement>, unit: &str) -> String {
    value
        .filter(|v| v.value.is_finite() && v.value >= 0.0)
        .map(|v| {
            format!(
                "{}{:.1} {unit}",
                if v.kind == UsageMeasurementKind::Estimated { "≈" } else { "" },
                v.value
            )
        })
        .unwrap_or_else(|| "—".into())
}
fn action(label: impl Into<String>, event: Event) -> Element<'static, Event> {
    button(text(label.into()).size(13))
        .padding([6, 10])
        .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
        .on_press(message(event))
        .into()
}
fn scroll<'a>(content: iced::widget::Column<'a, Event>) -> Element<'a, Event> {
    scrollable(container(content).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fixed(320.0))
        .into()
}
pub fn view(state: &State) -> Element<'_, Event> {
    use super::log_columns::COLUMNS;
    let mut controls = row![action("显示列 ▾", Event::Columns)].spacing(8);
    let status = state.query.as_ref().and_then(|q| q.filter.status);
    for (label, value) in [
        ("全部状态", None),
        ("成功", Some(UsageRequestStatus::Succeeded)),
        ("失败", Some(UsageRequestStatus::Failed)),
        ("取消", Some(UsageRequestStatus::Cancelled)),
        ("未知", Some(UsageRequestStatus::Unknown)),
    ] {
        controls = controls.push(action(
            format!("{}{label}", if status == value { "✓ " } else { "" }),
            Event::Status(value),
        ));
    }
    let mut top = column![
        controls,
        row![
            desktop_ui_controls::system_settings_common::settings_text_input(
                "HTTP 状态码（留空为全部）",
                &state.status_code_draft
            )
            .on_input(Event::StatusCode)
            .on_submit(Event::ApplyStatusCode)
            .width(260),
            action("筛选状态码", Event::ApplyStatusCode)
        ]
        .spacing(8)
    ]
    .spacing(8);
    if let Some(error) = &state.status_code_error {
        top = top.push(text(error));
    }
    if state.columns_open {
        let mut options = column![text("显示的列（时间列始终显示）").size(13)].spacing(4);
        for (index, (_, label, _, _)) in COLUMNS.iter().enumerate().skip(1) {
            options = options.push(action(
                format!("{} {label}", if state.columns.visible(index) { "✓" } else { "○" }),
                Event::Toggle(index),
            ));
        }
        top = top.push(scroll(options));
    }
    if let Some(error) = &state.preference_error {
        top = top.push(text(format!("列设置保存/加载失败：{error}")));
    }
    let widths: Vec<f32> = COLUMNS
        .iter()
        .enumerate()
        .map(|(index, (_, _, _, minimum))| {
            state
                .result
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .map(|r| {
                    r.page
                        .items
                        .iter()
                        .map(|d| cell(d, index).chars().count() as f32 * 9.0 + 24.0)
                        .fold(*minimum, f32::max)
                })
                .unwrap_or(*minimum)
        })
        .collect();
    let mut rows = column![].spacing(0);
    let mut header = row![].spacing(12);
    for (index, (_, label, _, _)) in COLUMNS.iter().enumerate() {
        if state.columns.visible(index) {
            let title = text(*label).size(12).width(Length::Fixed(widths[index]));
            let help = match index {
                5 => Some("新增输入按来源语义扣除缓存；缺少必要数据时显示 —"),
                9 => Some("缓存读取 /（新增输入 + 缓存读取 + 缓存创建）"),
                10 => Some("输出 token / 秒；≈ 表示来源标记的估算值"),
                11 | 12 => Some("单位为秒；≈ 表示估算；来源未提供时显示 —"),
                _ => None,
            };
            header = if let Some(help) = help {
                header.push(iced::widget::tooltip(
                    title,
                    container(text(help).size(12)).padding(8).style(container::bordered_box),
                    iced::widget::tooltip::Position::Top,
                ))
            } else {
                header.push(title)
            };
        }
    }
    rows = rows.push(container(header).padding([10, 12]).style(container::bordered_box));
    if state.loading {
        rows = rows.push(text("正在加载请求…"));
    }
    match &state.result {
        Some(Err(error)) => rows = rows.push(text(format!("请求加载失败：{error}"))),
        Some(Ok(result)) => {
            if result.page.items.is_empty() {
                rows = rows.push(container(text("暂无请求")).padding(24));
            }
            for detail in &result.page.items {
                let mut cells = row![].spacing(12);
                for (index, (_, _, _, _)) in COLUMNS.iter().enumerate() {
                    if state.columns.visible(index) {
                        let value = if index == 0 && state.columns.visible(13) {
                            cell(detail, index).split(" · ").next().unwrap_or("—").to_owned()
                        } else {
                            cell(detail, index)
                        };
                        if index == 1 {
                            let app = detail
                                .record
                                .sources
                                .first()
                                .map(|s| s.app.as_str())
                                .unwrap_or("agent");
                            cells = cells.push(
                                row![
                                    iced::widget::svg(desktop_assets::assets::get_provider_icon(
                                        app
                                    ))
                                    .width(14)
                                    .height(14),
                                    text(value).size(13)
                                ]
                                .spacing(6)
                                .width(Length::Fixed(widths[index])),
                            );
                        } else {
                            cells = cells
                                .push(text(value).size(13).width(Length::Fixed(widths[index])));
                        }
                    }
                }
                rows = rows.push(
                    button(cells)
                        .padding([10, 12])
                        .style(|theme: &iced::Theme, status| {
                            let mut style = button::text(theme, status);
                            style.border = iced::Border {
                                width: 0.5,
                                color: theme.extended_palette().background.strong.color,
                                ..Default::default()
                            };
                            style
                        })
                        .on_press(Event::Open(detail.record.id.clone())),
                );
            }
        }
        None => {}
    }
    let next = state
        .result
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .is_some_and(|r| r.page.next_cursor.is_some());
    let nav = |label: &'static str, event: Event, enabled: bool| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(event))
    };
    let total = state
        .result
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|r| r.counts.total_requests.to_string())
        .unwrap_or_else(|| "—".into());
    let mut footer = row![
        text(format!("共 {total} 条 · 第 {} 页", state.cursor_history.len() + 1)).size(13),
        nav("上一页", Event::Previous, !state.loading && !state.cursor_history.is_empty()),
        nav("下一页", Event::Next, !state.loading && next),
        nav("刷新 / 重试", Event::Reload, !state.loading)
    ]
    .spacing(8);
    for size in [10, 20, 50, 100] {
        footer = footer.push(action(
            format!("{}{size}条", if size == state.page_size { "✓ " } else { "" }),
            Event::PageSize(size),
        ));
    }
    let table = scrollable(container(rows).padding([0, 7]))
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::new().width(4).scroller_width(4),
            horizontal: scrollable::Scrollbar::new().width(4).scroller_width(4),
        })
        .height(Length::Fixed(360.0));
    let base = column![top, table, footer].spacing(8);
    if !state.detail_open {
        return base.into();
    }
    use desktop_ui_controls::system_settings_common::{
        settings_close_button, settings_modal_card, settings_modal_overlay,
    };
    let body = match &state.detail {
        None => "正在加载详情…".into(),
        Some(Err(error)) => format!("请求详情加载失败：{error}"),
        Some(Ok(detail)) => detail_text(detail),
    };
    let card = settings_modal_card(
        column![
            row![text("请求详情").size(20), settings_close_button(message(Event::CloseDetail))]
                .spacing(16),
            scroll(column![text(body)]),
            action("关闭", Event::CloseDetail)
        ]
        .spacing(12),
    )
    .width(Length::Fixed(720.0));
    settings_modal_overlay(Some(base.into()), message(Event::CloseDetail), card)
}
fn detail_text(detail: &UsageLogDetail) -> String {
    let r = &detail.record;
    let tokens = r.tokens.as_ref().map(|t| {
        let count = |v: Option<u64>| v.map(|n| n.to_string()).unwrap_or_else(|| "未知".into());
        format!("输入 {} · 输出 {} · 推理 {}\n缓存读取 {} · 创建 {} · 创建1h子集 {}\n输入语义 {:?} · 推理语义 {:?}", count(t.input), count(t.output), count(t.reasoning), count(t.cache_read), count(t.cache_creation), count(t.cache_creation_1h), t.input_semantics, t.reasoning_semantics)
    }).unwrap_or_else(|| "Token 未知".into());
    format!(
        "请求 {}\n时间 {}（UTC Unix秒）\n供应商 {:?}\n状态 {:?} · HTTP {} · 计费 {}\n错误 {}\n请求模型 {}\n响应模型 {}\n定价模型 {}\n倍率 {} · 依据 {}\n费用 {}\n{}\n首字 {}\n延迟 {}\n速度 {}\n来源 {:?}\n疑似重复 {}",
        r.id,
        r.started_at_unix_seconds.0,
        r.provider,
        r.status,
        detail.status_code.map(|n| n.to_string()).unwrap_or_else(|| "未知".into()),
        r.billable,
        unknown(detail.error_message.as_deref()),
        unknown(r.models.request_model.as_deref()),
        unknown(r.models.response_model.as_deref()),
        unknown(r.models.pricing_model.as_deref()),
        unknown(r.models.pricing_multiplier.as_deref()),
        unknown(r.models.multiplier_basis.as_deref()),
        cost(&r.cost),
        tokens,
        metric(r.performance.first_token_ms.as_ref(), "ms"),
        metric(r.performance.latency_ms.as_ref(), "ms"),
        metric(r.performance.output_tokens_per_second.as_ref(), "token/s"),
        r.sources,
        unknown(r.suspected_duplicate_of.as_deref())
    )
}

fn count(value: Option<u64>) -> String {
    match value {
        Some(n) if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        Some(n) if n >= 1000 => format!("{:.1}K", n as f64 / 1000.0),
        Some(n) => n.to_string(),
        None => "—".into(),
    }
}
fn fresh(tokens: &UsageTokens) -> Option<u64> {
    match tokens.input_semantics {
        UsageInputSemantics::Fresh => tokens.input,
        UsageInputSemantics::Total => {
            tokens.input?.checked_sub(tokens.cache_read?.checked_add(tokens.cache_creation?)?)
        }
        UsageInputSemantics::Unknown => None,
    }
}
fn seconds(value: Option<&UsageMeasurement>) -> String {
    let converted = value.map(|v| UsageMeasurement { value: v.value / 1000.0, kind: v.kind });
    metric(converted.as_ref(), "s")
}
fn cell(detail: &UsageLogDetail, index: usize) -> String {
    let r = &detail.record;
    let t = r.tokens.as_ref();
    match index {
        0 => {
            let time = chrono::DateTime::from_timestamp(r.started_at_unix_seconds.0, 0)
                .map(|t| t.with_timezone(&chrono::Local).format("%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "未知时间".into());
            if r.status == UsageRequestStatus::Succeeded {
                time
            } else {
                format!(
                    "{time} · {}",
                    detail
                        .status_code
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{:?}", r.status))
                )
            }
        }
        1 => {
            let mut apps: Vec<_> = r.sources.iter().map(|s| s.app.as_str()).collect();
            apps.sort_unstable();
            apps.dedup();
            if apps.is_empty() { "未知".into() } else { apps.join(" / ") }
        }
        2 => match &r.provider {
            UsageProvider::Unknown => "未知供应商".into(),
            UsageProvider::Known { provider_id } => provider_id.clone(),
        },
        3 => unknown(r.models.response_model.as_deref().or(r.models.request_model.as_deref())),
        4 => r
            .cost
            .amount_usd
            .as_ref()
            .map(|v| {
                let amount = v
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .map(|v| format!("${v:.4}"))
                    .unwrap_or_else(|| format!("{v} USD"));
                format!(
                    "{}{amount}",
                    if r.cost.source == UsageCostSource::Estimated { "≈" } else { "" }
                )
            })
            .unwrap_or_else(|| "未定价".into()),
        5 => count(t.and_then(fresh)),
        6 => count(t.and_then(|t| t.output)),
        7 => count(t.and_then(|t| t.cache_read)),
        8 => count(t.and_then(|t| t.cache_creation)),
        9 => t
            .and_then(|t| {
                Some((
                    t.cache_read?,
                    fresh(t)?.checked_add(t.cache_read?)?.checked_add(t.cache_creation?)?,
                ))
            })
            .filter(|(_, total)| *total > 0)
            .map(|(read, total)| format!("{:.1}%", read as f64 / total as f64 * 100.0))
            .unwrap_or_else(|| "—".into()),
        10 => metric(r.performance.output_tokens_per_second.as_ref(), "tok/s"),
        11 => seconds(r.performance.first_token_ms.as_ref()),
        12 => seconds(r.performance.latency_ms.as_ref()),
        13 => detail.status_code.map(|s| s.to_string()).unwrap_or_else(|| "—".into()),
        _ => String::new(),
    }
}
