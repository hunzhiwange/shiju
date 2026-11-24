//! Presentation of server aggregates; never recompute billing or performance.
use super::{
    log_columns::{Column, Preferences, column_menu},
    logs::{COLUMN_GAP, TABLE_INSET, log_row_style, table_header_style, usage_detail_drawer},
    metrics,
};
use crate::Identity;

use desktop_i18n::I18n;
use desktop_ui_controls::{
    selectable_text::markdown_selection::SelectionMessage,
    system_settings_common::{help::settings_selectable_help_label, settings_right_drawer_overlay},
    text_editor_context_menu::TextContextEffect,
};

use gateway_usage_read::{GroupStats, ModelStats, ProviderStats};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, mouse_area, responsive, row, rule, scrollable, text},
};
use vw_api_usage::{UsageProvider, UsageQuery};

#[derive(Default)]
pub struct State {
    pub result: Option<Result<(Vec<ProviderStats>, Vec<ModelStats>), String>>,
    pub loading: bool,
    pub sort: Sort,
    columns: [Preferences; 2],
    columns_loaded: bool,
    columns_menu: Option<Table>,
    detail: Option<(Table, String, GroupStats)>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Loaded(u64, Identity, Result<(Vec<ProviderStats>, Vec<ModelStats>), String>),
    Sort(SortColumn),
    Columns(Table),
    CloseColumns,
    ToggleColumn(Table, usize),
    ColumnError(String),
    Open(GroupKey),
    CloseDetail,
    Help(TextContextEffect),
    None,
}

impl SelectionMessage for Event {
    fn none() -> Self {
        Self::None
    }

    fn copy(text: String) -> Self {
        Self::Help(TextContextEffect::Copy(text))
    }

    fn context(effect: TextContextEffect) -> Self {
        Self::Help(effect)
    }

    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Providers,
    Models,
}

const MODEL_COLUMNS: [Column; 7] = [
    ("model", "模型", true, 185.0),
    ("requests", "请求数", true, 105.0),
    ("cost", "成本", true, 125.0),
    ("tokens", "Tokens", true, 100.0),
    ("success", "成功率", true, 90.0),
    ("speed", "速度", true, 100.0),
    ("firstToken", "首字", true, 90.0),
];
const PROVIDER_COLUMNS: [Column; 7] = {
    let mut columns = MODEL_COLUMNS;
    columns[0] = ("provider", "供应商", true, 185.0);
    columns
};

impl Table {
    fn columns(self) -> &'static [Column] {
        match self {
            Self::Providers => &PROVIDER_COLUMNS,
            Self::Models => &MODEL_COLUMNS,
        }
    }

    fn preferences_file(self) -> &'static str {
        match self {
            Self::Providers => "usage-provider-columns.json",
            Self::Models => "usage-model-columns.json",
        }
    }
}

#[derive(Clone, Debug)]
pub enum GroupKey {
    Provider { app: Option<String>, provider: UsageProvider },
    Model(Option<String>),
}

impl State {
    pub(super) fn close_table_controls(&mut self) {
        self.columns_menu = None;
        self.detail = None;
    }

    fn load_columns(&mut self) -> Task<Event> {
        if self.columns_loaded {
            return Task::none();
        }
        self.columns_loaded = true;
        Task::batch([Table::Providers, Table::Models].map(|table| {
            match Preferences::load_from(table.columns(), table.preferences_file()) {
                Ok(preferences) => {
                    self.columns[table as usize] = preferences;
                    Task::none()
                }
                Err(error) => Task::done(Event::ColumnError(error)),
            }
        }))
    }

    fn open_group(&mut self, key: GroupKey) {
        let Some(Ok((providers, models))) = &self.result else { return };
        self.detail = match key {
            GroupKey::Provider { app, provider } => providers
                .iter()
                .find(|group| group.app == app && group.provider == provider)
                .map(|group| (Table::Providers, provider_title(group), group.stats.clone())),
            GroupKey::Model(model) => models
                .iter()
                .find(|group| group.model == model)
                .map(|group| (Table::Models, model_title(group), group.stats.clone())),
        };
    }
}

pub fn load(
    client: Result<vw_gateway_client::GatewayClient, String>,
    ticket: u64,
    scope: Identity,
    query: UsageQuery,
    time_zone: String,
) -> Task<Event> {
    let client = client
        .map(|client| gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone()));
    Task::perform(
        async move {
            let result = async {
                let client = client?;
                let providers = client
                    .usage_provider_stats(&time_zone, &query)
                    .await
                    .map_err(|e| e.to_string())?;
                let models = client
                    .usage_model_stats(&time_zone, &query)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((providers, models))
            }
            .await;
            (scope, result)
        },
        move |(scope, result)| Event::Loaded(ticket, scope, result),
    )
}

pub fn update(
    state: &mut State,
    current: Option<Identity>,
    visible: bool,
    open: bool,
    generation: u64,
    event: Event,
) -> Task<Event> {
    match event {
        Event::None | Event::Help(_) | Event::ColumnError(_) => {}
        Event::Loaded(ticket, scope, result) => {
            if open && generation == ticket && current.as_ref() == Some(&scope) && visible {
                state.loading = false;
                if result.is_ok() || !state.result.as_ref().is_some_and(Result::is_ok) {
                    state.result = Some(result);
                }
                return state.load_columns();
            }
        }
        Event::Sort(column) => {
            state.sort =
                Sort { column, descending: state.sort.column != column || !state.sort.descending };
        }
        Event::Columns(table) => {
            state.columns_menu = (state.columns_menu != Some(table)).then_some(table);
            return state.load_columns();
        }
        Event::CloseColumns => state.columns_menu = None,
        Event::ToggleColumn(table, index) => {
            let preferences = &mut state.columns[table as usize];
            preferences.toggle_in(table.columns(), index);
            if let Err(error) = preferences.save_to(table.preferences_file()) {
                return Task::done(Event::ColumnError(error));
            }
        }
        Event::Open(key) => state.open_group(key),
        Event::CloseDetail => state.detail = None,
    }
    Task::none()
}

pub(super) fn provider_label(provider: &UsageProvider, session_logs_only: bool) -> &str {
    match provider {
        UsageProvider::Unknown if session_logs_only => "会话日志",
        UsageProvider::Unknown => "未知供应商",
        UsageProvider::Known { provider_id } if provider_id.trim().is_empty() => "未知供应商",
        UsageProvider::Known { provider_id } => provider_id,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortColumn {
    #[default]
    Requests,
    Cost,
}
#[derive(Clone, Copy, Debug)]
pub struct Sort {
    pub column: SortColumn,
    pub descending: bool,
}
impl Default for Sort {
    fn default() -> Self {
        Self { column: SortColumn::Requests, descending: true }
    }
}
fn cost(stats: &GroupStats) -> Option<f64> {
    if stats.costs.is_empty() {
        return None;
    }
    let value =
        stats.costs.iter().try_fold(0.0, |sum, c| Some(sum + c.amount_usd.parse::<f64>().ok()?))?;
    value.is_finite().then_some(value)
}
fn compare(a: &GroupStats, b: &GroupStats, sort: Sort) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let order = match sort.column {
        SortColumn::Requests => a.counts.total_requests.cmp(&b.counts.total_requests),
        // Missing amounts always follow known subtotals, in either direction.
        SortColumn::Cost => match (cost(a), cost(b)) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            (None, None) => Ordering::Equal,
            (None, _) => return Ordering::Greater,
            (_, None) => return Ordering::Less,
        },
    };
    if sort.descending { order.reverse() } else { order }
}
const SPEED_HELP: &str = "输出 token 总数除以生成时间总和，不是逐条平均，免得短请求把数字带歪。只算经路由服务、输出不少于 100 token 的请求。带 ≈ 的是按会话日志估算的（含首字等待，只估输出不少于 200 token 的请求），这一行没有路由数据时才显示。";
const FIRST_HELP: &str = "从路由服务收到请求，到上游开始输出的时间。取这一行所有请求的中位数，不会被个别排队、冷启动的慢请求拉高。只算经路由服务、成功的流式请求；超过 30 天、已并入日汇总的请求不计。带 ≈ 的是按会话日志推出来的（偏高，只有部分请求推得出），这一行没有路由数据时才显示。";
fn performance_label(value: Option<super::metrics::PerformanceValue>, first: bool) -> String {
    value.map_or_else(
        || "—".into(),
        |v| {
            let prefix =
                if v.kind == vw_api_usage::UsageMeasurementKind::Estimated { "≈" } else { "" };
            if first {
                format!("{prefix}{:.1}s", v.value / 1000.0)
            } else {
                format!("{prefix}{:.0} tok/s", v.value)
            }
        },
    )
}
fn provider_title(group: &ProviderStats) -> String {
    format!(
        "{} · {}",
        provider_label(&group.provider, group.session_logs_only),
        group.app.as_deref().unwrap_or("未知应用")
    )
}

fn model_title(group: &ModelStats) -> String {
    group.model.as_deref().filter(|model| !model.trim().is_empty()).unwrap_or("未知模型").into()
}

fn values(title: String, stats: &GroupStats) -> [String; 7] {
    let tokens = if stats.tokens.iter().map(|group| group.requests).sum::<u64>()
        < stats.counts.total_requests
    {
        None
    } else {
        gateway_usage_read::UsageTokenTotals::from_groups(
            stats.tokens.iter().map(|group| &group.tokens),
        )
        .real_tokens
    };
    let rate = (stats.counts.total_requests > 0)
        .then(|| stats.succeeded as f64 / stats.counts.total_requests as f64);
    [
        title,
        metrics::format_count(stats.counts.total_requests),
        cost(stats).map_or_else(|| "—".into(), |value| format!("${value:.2}")),
        tokens.map_or_else(|| "—".into(), |value| super::hero::compact_count(value as f64)),
        metrics::format_hit_rate(rate),
        performance_label(metrics::generation_speed(stats), false),
        performance_label(metrics::first_token(stats), true),
    ]
}

pub(super) fn toolbar(i18n: desktop_i18n::I18n, state: &State, table: Table) -> Element<'_, Event> {
    let columns = table.columns();
    column_menu(i18n,
        columns.iter().enumerate().skip(1).map(|(index, (_, label, _, _))| (index, *label)),
        |index| state.columns[table as usize].visible_in(columns, index),
        state.columns_menu == Some(table),
        Event::Columns(table),
        Event::CloseColumns,
        |index| Event::ToggleColumn(table, index),
    )
}

pub fn view(state: &State, i18n: I18n) -> Element<'_, Event> {
    let section = |table| {
        column![
            container(toolbar(i18n, state, table)).align_right(Length::Fill).padding([0.0, TABLE_INSET]),
            table_view(state, table, i18n),
        ]
    };
    let tables = column![section(Table::Providers), section(Table::Models)];
    settings_right_drawer_overlay(tables.into(), detail_drawer(i18n, state))
}

pub(super) fn table_view(state: &State, table: Table, i18n: I18n) -> Element<'_, Event> {
    responsive(move |size| table_content(state, table, i18n, size.width))
        .height(Length::Shrink)
        .into()
}

fn table_content(
    state: &State,
    table: Table,
    i18n: I18n,
    viewport_width: f32,
) -> Element<'_, Event> {
    let columns = table.columns();
    let mut visible: Vec<_> = columns
        .iter()
        .enumerate()
        .filter(|(index, _)| state.columns[table as usize].visible_in(columns, *index))
        .map(|(index, &(_, label, _, width))| (index, label, width))
        .collect();
    let content_width = visible.iter().map(|(_, _, width)| width).sum::<f32>()
        + COLUMN_GAP * visible.len().saturating_sub(1) as f32
        + TABLE_INSET * 2.0;
    let table_width =
        if viewport_width.is_finite() { content_width.max(viewport_width) } else { content_width };
    let extra_per_column = (table_width - content_width) / visible.len() as f32;
    for (_, _, width) in &mut visible {
        *width += extra_per_column;
    }
    let mut header = row![].spacing(COLUMN_GAP).align_y(iced::Alignment::Center);
    for &(index, label, width) in &visible {
        let label = i18n.__(label);
        let title: Element<'_, Event> = match index {
            1 | 2 => {
                let column = if index == 1 { SortColumn::Requests } else { SortColumn::Cost };
                let arrow = if state.sort.column == column {
                    if state.sort.descending { " ↓" } else { " ↑" }
                } else {
                    ""
                };
                mouse_area(text(format!("{label}{arrow}")).size(12))
                    .interaction(iced::mouse::Interaction::Pointer)
                    .on_press(Event::Sort(column))
                    .into()
            }
            5 => {
                settings_selectable_help_label(i18n, None, label, text(i18n.__(label)).size(12), i18n.__(SPEED_HELP))
                    .into()
            }
            6 => {
                settings_selectable_help_label(i18n, None, label, text(i18n.__(label)).size(12), i18n.__(FIRST_HELP))
                    .into()
            }
            _ => text(i18n.__(label)).size(12).into(),
        };
        header = header.push(container(title).width(width));
    }
    let mut rows = column![
        container(header).padding([10.0, TABLE_INSET]).width(table_width).style(table_header_style),
        rule::horizontal(1),
    ];
    let mut groups = match &state.result {
        Some(Ok((providers, _))) if table == Table::Providers => providers
            .iter()
            .map(|group| {
                let key =
                    GroupKey::Provider { app: group.app.clone(), provider: group.provider.clone() };
                (key, provider_title(group), &group.stats)
            })
            .collect::<Vec<_>>(),
        Some(Ok((_, models))) => models
            .iter()
            .map(|group| (GroupKey::Model(group.model.clone()), model_title(group), &group.stats))
            .collect(),
        _ => Vec::new(),
    };
    if state.loading {
        rows = rows.push(container(text(i18n.__("正在加载分组…"))).padding([10.0, TABLE_INSET]));
    }
    match &state.result {
        Some(Err(error)) => {
            rows = rows.push(
                container(text(i18n.format("分组加载失败：%s", &[(format!("{}", error)).into()]).expect("valid display template"))).padding([10.0, TABLE_INSET]),
            );
        }
        Some(Ok(_)) if groups.is_empty() && !state.loading => {
            rows = rows.push(container(text(i18n.__("暂无用量"))).padding([18.0, TABLE_INSET]));
        }
        _ => {}
    }
    groups.sort_by(|a, b| compare(a.2, b.2, state.sort));
    for (key, title, stats) in groups {
        let values = values(title, stats);
        let mut cells = row![].spacing(COLUMN_GAP).align_y(iced::Alignment::Center);
        for &(index, _, width) in &visible {
            cells = cells.push(
                text(values[index].clone())
                    .size(13)
                    .width(width)
                    .wrapping(iced::advanced::text::Wrapping::WordOrGlyph),
            );
        }
        rows = rows
            .push(
                button(cells)
                    .width(table_width)
                    .padding([10.0, TABLE_INSET])
                    .style(|theme, status| log_row_style(theme, status, false))
                    .on_press(Event::Open(key)),
            )
            .push(rule::horizontal(1));
    }
    scrollable(rows.width(table_width))
        .width(Length::Fill)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(3),
        ))
        .height(Length::Shrink)
        .into()
}

pub(super) fn detail_drawer(i18n: desktop_i18n::I18n, state: &State) -> Option<(Event, Element<'_, Event>)> {
    let (table, title, stats) = state.detail.as_ref()?;
    let mut rows = column![].spacing(12);
    let entries = table
        .columns()
        .iter()
        .map(|(_, label, _, _)| *label)
        .zip(values(title.clone(), stats))
        .chain([(i18n.__("未知费用请求"), metrics::format_count(stats.counts.unknown_cost_requests))]);
    for (label, value) in entries {
        rows = rows.push(
            row![
                text(i18n.__(label)).size(13).style(text::secondary).width(105),
                text(value)
                    .size(14)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .wrapping(iced::advanced::text::Wrapping::WordOrGlyph)
            ]
            .spacing(16),
        );
    }
    let title = match table {
        Table::Providers => i18n.__("供应商详情"),
        Table::Models => i18n.__("模型详情"),
    };
    let body = container(rows).padding(14).width(Length::Fill).style(super::view::panel_style);
    Some((Event::CloseDetail, usage_detail_drawer(i18n, title, Event::CloseDetail, body.into())))
}

#[cfg(test)]
#[path = "../usage_replica_05_tests.rs"]
mod usage_replica_05_tests;
