//! Presentation of server aggregates; never recompute billing or performance.
use crate::Identity;

use gateway_usage_read::{GroupStats, ModelStats, ProviderStats, UsageReadScope};
use iced::{
    Element, Length, Task,
    widget::{button, checkbox, column, container, row, scrollable, text, tooltip},
};
use vw_api_usage::{UsageProvider, UsageQuery};

#[derive(Default)]
pub struct State {
    pub result: Option<Result<(Vec<ProviderStats>, Vec<ModelStats>), String>>,
    pub loading: bool,
    pub sort: Sort,
    pub hide_cost: bool,
    pub hide_tokens: bool,
    pub hide_performance: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Loaded(u64, Identity, Result<(Vec<ProviderStats>, Vec<ModelStats>), String>),
    Sort(SortColumn),
    Cost(bool),
    Tokens(bool),
    Performance(bool),
}

fn message(event: Event) -> Event {
    event
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
                let read_scope = UsageReadScope {
                    project_id: scope.project.clone().into(),
                    target_host: scope.host.clone(),
                    time_zone,
                };
                let providers = client
                    .usage_provider_stats(&read_scope, &query)
                    .await
                    .map_err(|e| e.to_string())?;
                let models = client
                    .usage_model_stats(&read_scope, &query)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((providers, models))
            }
            .await;
            (scope, result)
        },
        move |(scope, result)| message(Event::Loaded(ticket, scope, result)),
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
        Event::Loaded(ticket, scope, result) => {
            if open && generation == ticket && current.as_ref() == Some(&scope) && visible {
                state.loading = false;
                state.result = Some(result);
            }
        }
        Event::Sort(column) => {
            state.sort =
                Sort { column, descending: state.sort.column != column || !state.sort.descending };
        }
        Event::Cost(show) => state.hide_cost = !show,
        Event::Tokens(show) => state.hide_tokens = !show,
        Event::Performance(show) => state.hide_performance = !show,
    }
    Task::none()
}

fn provider_label(provider: &UsageProvider) -> &str {
    match provider {
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
fn cell(value: String, width: u16) -> Element<'static, Event> {
    text(value).size(13).width(width).into()
}
fn stats_row<'a>(label: String, stats: &'a GroupStats, state: &State) -> Element<'a, Event> {
    use super::metrics;
    let mut cells =
        row![cell(label, 180), cell(stats.counts.total_requests.to_string(), 100)].spacing(24);
    if !state.hide_cost {
        let amount = cost(stats).map_or_else(|| "—".into(), |v| format!("${v:.2}"));
        let detail = format!("已知费用小计；未知费用 {} 请求", stats.counts.unknown_cost_requests);
        cells = cells.push(tooltip(
            cell(amount, 140),
            container(text(detail).size(12)).padding(8).style(container::rounded_box),
            tooltip::Position::Top,
        ));
    }
    if !state.hide_tokens {
        let value =
            if stats.tokens.iter().map(|g| g.requests).sum::<u64>() < stats.counts.total_requests {
                None
            } else {
                gateway_usage_read::UsageTokenTotals::from_groups(
                    stats.tokens.iter().map(|g| &g.tokens),
                )
                .real_tokens
            };
        let label = value.map_or_else(
            || "—".into(),
            |n| {
                if n >= 1_000_000_000 {
                    format!("{:.0}B", n as f64 / 1e9)
                } else if n >= 1_000_000 {
                    format!("{:.0}M", n as f64 / 1e6)
                } else if n >= 1000 {
                    format!("{:.0}K", n as f64 / 1e3)
                } else {
                    n.to_string()
                }
            },
        );
        cells = cells.push(cell(label, 110));
    }
    if !state.hide_performance {
        let rate = (stats.counts.total_requests > 0)
            .then(|| stats.succeeded as f64 / stats.counts.total_requests as f64);
        cells = cells
            .push(cell(metrics::format_hit_rate(rate), 100))
            .push(cell(performance_label(metrics::generation_speed(stats), false), 110))
            .push(cell(performance_label(metrics::first_token(stats), true), 100));
    }
    container(cells)
        .padding([10, 16])
        .style(|theme: &iced::Theme| container::Style {
            border: iced::Border {
                color: theme.extended_palette().background.strong.color,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
fn header<'a>(name: &'static str, state: &State) -> Element<'a, Event> {
    use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
    let sort_button = |label: &str, column, width| {
        let arrow = if state.sort.column == column {
            if state.sort.descending { " ↓" } else { " ↑" }
        } else {
            ""
        };
        button(text(format!("{label}{arrow}")).size(12))
            .width(width)
            .padding([4, 0])
            .style(rounded_action_btn_style)
            .on_press(Event::Sort(column))
    };
    let mut cells =
        row![text(name).size(12).width(180), sort_button("请求数", SortColumn::Requests, 100)]
            .spacing(24);
    if !state.hide_cost {
        cells = cells.push(sort_button("成本", SortColumn::Cost, 140));
    }
    if !state.hide_tokens {
        cells = cells.push(text("Tokens").size(12).width(110));
    }
    if !state.hide_performance {
        cells = cells.push(text("成功率").size(12).width(100));
        for (label, help, width) in [("速度 ⓘ", SPEED_HELP, 110), ("首字 ⓘ", FIRST_HELP, 100)]
        {
            cells = cells.push(tooltip(
                text(label).size(12).width(width),
                container(text(help).size(12).width(360)).padding(12).style(container::rounded_box),
                tooltip::Position::Top,
            ));
        }
    }
    container(cells).padding([8, 16]).style(container::rounded_box).into()
}

pub fn view(state: &State) -> Element<'_, Event> {
    table_view(state, None)
}
/// The parent owns tab selection; these entry points display a single table.
pub fn provider_view(state: &State) -> Element<'_, Event> {
    table_view(state, Some(false))
}
pub fn model_view(state: &State) -> Element<'_, Event> {
    table_view(state, Some(true))
}
fn table_view(state: &State, models_only: Option<bool>) -> Element<'_, Event> {
    use desktop_ui_controls::system_settings_common::settings_checkbox_style;

    let controls = row![
        checkbox(!state.hide_cost)
            .label("费用")
            .on_toggle(|v| message(Event::Cost(v)))
            .style(settings_checkbox_style),
        checkbox(!state.hide_tokens)
            .label("Token / 缓存")
            .on_toggle(|v| message(Event::Tokens(v)))
            .style(settings_checkbox_style),
        checkbox(!state.hide_performance)
            .label("性能")
            .on_toggle(|v| message(Event::Performance(v)))
            .style(settings_checkbox_style),
    ]
    .spacing(16);
    let mut body = column![text("供应商与模型汇总"), controls].spacing(10);
    if state.loading {
        return body.push(text("正在加载分组…")).into();
    }
    match &state.result {
        None => body.push(text("请选择用量范围")).into(),
        Some(Err(error)) => body.push(text(format!("分组加载失败：{error}"))).into(),
        Some(Ok((providers, models))) => {
            let mut rows = column![].spacing(0);
            if models_only != Some(true) {
                rows = rows
                    .push(text(format!("供应商 {}", providers.len())).size(13))
                    .push(header("供应商", state));
                let mut providers = providers.iter().collect::<Vec<_>>();
                providers.sort_by(|a, b| compare(&a.stats, &b.stats, state.sort));
                if providers.is_empty() {
                    rows = rows.push(text("暂无供应商用量"));
                }
                for provider in providers {
                    rows = rows.push(stats_row(
                        format!(
                            "{} · {}",
                            provider_label(&provider.provider),
                            provider.app.as_deref().unwrap_or("未知应用")
                        ),
                        &provider.stats,
                        state,
                    ));
                }
            }
            if models_only != Some(false) {
                rows = rows
                    .push(text(format!("模型 {}", models.len())).size(13))
                    .push(header("模型", state));
                let mut models = models.iter().collect::<Vec<_>>();
                models.sort_by(|a, b| compare(&a.stats, &b.stats, state.sort));
                if models.is_empty() {
                    rows = rows.push(text("暂无模型用量"));
                }
                for model in models {
                    rows = rows.push(stats_row(
                        model
                            .model
                            .as_deref()
                            .filter(|m| !m.trim().is_empty())
                            .unwrap_or("未知模型")
                            .to_owned(),
                        &model.stats,
                        state,
                    ));
                }
            }
            let bar = || scrollable::Scrollbar::new().width(4).scroller_width(4);
            body = body.push(
                scrollable(container(rows).padding([7, 7]))
                    .height(Length::Fixed(420.0))
                    .direction(scrollable::Direction::Both { vertical: bar(), horizontal: bar() }),
            );
            body.into()
        }
    }
}

#[cfg(test)]
#[path = "../usage_replica_05_tests.rs"]
mod usage_replica_05_tests;
