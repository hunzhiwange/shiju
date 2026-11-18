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
    CloseColumns,
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
        Event::CloseColumns => state.columns_open = false,
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
                state.result = Some(Err("网关不可用，请检查连接".into()));
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
    let status = state.query.as_ref().and_then(|q| q.filter.status);
    let statuses = ["全部状态", "成功", "失败", "取消", "未知"];
    let selected = match status {
        None => statuses[0],
        Some(UsageRequestStatus::Succeeded) => statuses[1],
        Some(UsageRequestStatus::Failed) => statuses[2],
        Some(UsageRequestStatus::Cancelled) => statuses[3],
        Some(UsageRequestStatus::Unknown) => statuses[4],
    };
    let mut options = column![].spacing(2);
    for (index, (_, label, _, _)) in COLUMNS.iter().enumerate().skip(1) {
        options = options.push(action(
            format!("{}  {label}", if state.columns.visible(index) { "✓" } else { "　" }),
            Event::Toggle(index),
        ));
    }
    let columns = desktop_ui_controls::overlays::BelowOverlay::new(
        action("显示列 ▾", Event::Columns),
        desktop_ui_controls::system_settings_common::settings_modal_card(scroll(options))
            .width(220),
    )
    .show(state.columns_open)
    .gap(8.0)
    .on_close(Event::CloseColumns)
    .capture_outside_click(true);
    let mut top = column![
        row![
            iced::widget::Space::new().width(Length::Fill),
            iced::widget::pick_list(statuses, Some(selected), |label| Event::Status(match label {
                "成功" => Some(UsageRequestStatus::Succeeded),
                "失败" => Some(UsageRequestStatus::Failed),
                "取消" => Some(UsageRequestStatus::Cancelled),
                "未知" => Some(UsageRequestStatus::Unknown),
                _ => None,
            }))
            .padding([6, 10]),
            desktop_ui_controls::system_settings_common::settings_text_input(
                "状态码",
                &state.status_code_draft
            )
            .on_input(Event::StatusCode)
            .on_submit(Event::ApplyStatusCode)
            .width(90),
            action("筛选", Event::ApplyStatusCode),
            columns
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    ]
    .spacing(8);
    if let Some(error) = &state.status_code_error {
        top = top.push(text(error));
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
            let title = text(if matches!(index, 5 | 9 | 10 | 11 | 12) {
                format!("{label}  ⓘ")
            } else {
                (*label).into()
            })
            .size(12)
            .width(Length::Fixed(widths[index]));
            let help = match index {
                5 => Some("新增输入按来源语义扣除缓存；缺少必要数据时显示 —"),
                9 => Some("缓存读取 /（新增输入 + 缓存读取 + 缓存创建）"),
                10 => Some(
                    "速度怎么算：每秒输出的 token 数。≈ 表示根据会话日志时间戳估算，可能包含等待首字的时间；来源未提供时显示 —。",
                ),
                11 | 12 => Some(
                    "时间怎么算：单位为秒。首字表示开始输出前的等待时间，总耗时表示请求持续时间。≈ 为日志时间戳估算，可能包含客户端准备时间；来源未提供时显示 —。",
                ),
                _ => None,
            };
            header = if let Some(help) = help {
                header.push(iced::widget::tooltip(
                    title,
                    container(text(help).size(13))
                        .width(360)
                        .padding(14)
                        .style(container::bordered_box),
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
    let page_size = if state.page_size == 0 { 20 } else { state.page_size };
    let pages = state
        .result
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|r| r.counts.total_requests.div_ceil(u64::from(page_size)).max(1))
        .unwrap_or(1);
    let footer = container(
        row![
            text(format!("共 {total} 条记录")).size(13).style(text::secondary),
            iced::widget::pick_list(vec![10u32, 20, 50, 100], Some(page_size), Event::PageSize)
                .padding([6, 10]),
            text("条 / 页").size(13).style(text::secondary),
            iced::widget::Space::new().width(Length::Fill),
            nav("‹", Event::Previous, !state.loading && !state.cursor_history.is_empty()),
            text(format!("{} / {pages}", state.cursor_history.len() + 1)).size(13),
            nav("›", Event::Next, !state.loading && next)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(container::rounded_box);
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
    let body: Element<'_, Event> = match &state.detail {
        None => text("正在加载详情…").into(),
        Some(Err(error)) => text(format!("请求详情加载失败：{error}")).into(),
        Some(Ok(detail)) => detail_view(detail),
    };
    let card = settings_modal_card(
        column![
            row![
                text("请求详情").size(20),
                iced::widget::Space::new().width(Length::Fill),
                settings_close_button(message(Event::CloseDetail))
            ]
            .spacing(16),
            scroll(column![body]),
            action("关闭", Event::CloseDetail)
        ]
        .spacing(12),
    )
    .width(Length::Fixed(720.0));
    settings_modal_overlay(Some(base.into()), message(Event::CloseDetail), card)
}
fn detail_view(detail: &UsageLogDetail) -> Element<'_, Event> {
    fn section(
        title: &'static str,
        values: Vec<(&'static str, String)>,
    ) -> Element<'static, Event> {
        let mut rows = column![].spacing(12);
        for (label, value) in values {
            rows = rows.push(
                row![
                    text(label).size(13).style(text::secondary).width(110),
                    text(value)
                        .size(14)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                ]
                .spacing(16),
            );
        }
        column![
            text(title).size(17),
            container(rows).padding(18).width(Length::Fill).style(container::rounded_box)
        ]
        .spacing(10)
        .into()
    }
    let r = &detail.record;
    let exact = |n: Option<u64>| n.map_or_else(|| "—".into(), |n| n.to_string());
    let t = r.tokens.as_ref();
    let time = chrono::DateTime::from_timestamp(r.started_at_unix_seconds.0, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y/%m/%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".into());
    let mut body = column![
        section(
            "基本信息",
            vec![
                ("时间", time),
                ("应用", cell(detail, 1)),
                ("供应商", cell(detail, 2)),
                ("模型", cell(detail, 3)),
                ("请求 ID", r.id.clone())
            ]
        ),
        section(
            "Tokens 用量",
            vec![
                ("新增输入", exact(t.and_then(fresh))),
                ("原始输入", exact(t.and_then(|t| t.input))),
                ("输出", exact(t.and_then(|t| t.output))),
                ("缓存命中", exact(t.and_then(|t| t.cache_read))),
                ("缓存写入", exact(t.and_then(|t| t.cache_creation)))
            ]
        ),
        section(
            "成本（USD）",
            vec![
                ("合计", cell(detail, 4)),
                ("计价说明", cost(&r.cost)),
                ("定价模型", unknown(r.models.pricing_model.as_deref()))
            ]
        ),
        section(
            "性能",
            vec![
                ("速度", cell(detail, 10)),
                ("首字", cell(detail, 11)),
                ("总耗时", cell(detail, 12)),
                ("状态码", cell(detail, 13))
            ]
        ),
        text("≈ 表示来源提供的估算值；— 表示暂无数据。费用分项尚未由账本提供。")
            .size(12)
            .style(text::secondary)
    ]
    .spacing(24);
    if let Some(error) = &detail.error_message {
        body = body.push(section("错误信息", vec![("详情", error.clone())]));
    }
    body.into()
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
