//! Presentation of server aggregates; never recompute billing or performance.
use super::{Identity, identity, Event as CenterEvent};
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task, Length, widget::{column, row, text, checkbox, scrollable, container}};
use vw_gateway_client::client::{ProviderStats, ModelStats, GroupStats, MetricSamples, UsageReadScope};
use vw_gateway_client::vw_api_types::usage::{UsageProvider, UsageQuery};

#[derive(Default)]
pub struct State {
    pub result: Option<Result<(Vec<ProviderStats>, Vec<ModelStats>), String>>,
    pub loading: bool,
    pub hide_cost: bool,
    pub hide_tokens: bool,
    pub hide_performance: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Loaded(u64, Identity, Result<(Vec<ProviderStats>, Vec<ModelStats>), String>),
    Cost(bool),
    Tokens(bool),
    Performance(bool),
}

fn message(event: Event) -> Message {
    Message::View(ViewMessage::UsageCenter(CenterEvent::Groups(event)))
}

pub fn load(ticket: u64, scope: Identity, query: UsageQuery, time_zone: String) -> Task<Message> {
    let client = crate::app::config::gateway_client();
    Task::perform(async move {
        let result = async {
            let client = client?;
            let read_scope = UsageReadScope {
                project_id: scope.project.clone().into(), target_host: scope.host.clone(), time_zone,
            };
            let providers = client.usage_provider_stats(&read_scope, &query).await.map_err(|e| e.to_string())?;
            let models = client.usage_model_stats(&read_scope, &query).await.map_err(|e| e.to_string())?;
            Ok((providers, models))
        }.await;
        (scope, result)
    }, move |(scope, result)| message(Event::Loaded(ticket, scope, result)))
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Loaded(ticket, scope, result) => {
            if app.usage_center.open && app.usage_center.sequence == ticket
                && identity(app).as_ref() == Some(&scope)
                && app.screen == crate::app::Screen::Usage {
                app.usage_center.groups.loading = false;
                app.usage_center.groups.result = Some(result);
            }
        }
        Event::Cost(show) => app.usage_center.groups.hide_cost = !show,
        Event::Tokens(show) => app.usage_center.groups.hide_tokens = !show,
        Event::Performance(show) => app.usage_center.groups.hide_performance = !show,
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

fn metric(samples: &MetricSamples, unit: &str) -> String {
    match samples.mean.filter(|v| v.is_finite()) {
        Some(value) if samples.samples > 0 => format!("{value:.2} {unit}（{} 样本）", samples.samples),
        _ => "—（无样本）".into(),
    }
}

fn stats_row<'a>(label: &'a str, stats: &'a GroupStats, state: &State) -> Element<'a, Message> {
    let mut cells = row![text(label).width(180), text(stats.counts.total_requests.to_string()).width(100)].spacing(12);
    if !state.hide_cost {
        let costs = stats.costs.iter().map(|cost| format!("{:?}: {} USD / {} 请求", cost.source, cost.amount_usd, cost.requests)).collect::<Vec<_>>().join("\n");
        cells = cells.push(text(format!("{costs}\n未知费用：{} 请求", stats.counts.unknown_cost_requests)).width(240));
    }
    if !state.hide_tokens {
        // Preserve semantic groups and missing counters, including cache subsets.
        let tokens = stats.tokens.iter().map(|group| {
            let t = &group.tokens;
            let count = |v: Option<u64>| v.map_or_else(|| "—".into(), |v| v.to_string());
            format!("{} 请求 · 输入 {} / 输出 {} / 推理 {}\n缓存读 {} / 创建 {} / 其中1h {}\n输入 {:?} · 推理 {:?}", group.requests, count(t.input), count(t.output), count(t.reasoning), count(t.cache_read), count(t.cache_creation), count(t.cache_creation_1h), t.input_semantics, t.reasoning_semantics)
        }).collect::<Vec<_>>().join("\n");
        cells = cells.push(text(if tokens.is_empty() { "—（无 Token 数据）".into() } else { tokens }).width(340));
    }
    if !state.hide_performance {
        let measured = &stats.measured;
        let estimated = &stats.estimated;
        cells = cells.push(text(format!("成功率 {} · 未知状态 {}\n实测速度 {}\n≈估算速度 {}\n实测首字 {}\n≈估算首字 {}\n实测延迟 {}\n≈估算延迟 {}",
            stats.success_rate.map_or_else(|| "—".into(), |rate| format!("{:.1}%", rate * 100.0)), stats.unknown_status,
            metric(&measured.output_tokens_per_second, "tok/s"), metric(&estimated.output_tokens_per_second, "tok/s"),
            metric(&measured.first_token_ms, "ms"), metric(&estimated.first_token_ms, "ms"),
            metric(&measured.latency_ms, "ms"), metric(&estimated.latency_ms, "ms"))).width(320));
    }
    cells.into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::components::system_settings_common::settings_checkbox_style;
    let state = &app.usage_center.groups;
    let controls = row![
        checkbox("费用", !state.hide_cost).on_toggle(|v| message(Event::Cost(v))).style(settings_checkbox_style),
        checkbox("Token / 缓存", !state.hide_tokens).on_toggle(|v| message(Event::Tokens(v))).style(settings_checkbox_style),
        checkbox("性能", !state.hide_performance).on_toggle(|v| message(Event::Performance(v))).style(settings_checkbox_style),
    ].spacing(16);
    let mut body = column![text("供应商与模型汇总 · 请求数 ↓（降序）"), controls].spacing(10);
    if state.loading {
        return body.push(text("正在加载分组…")).into();
    }
    match &state.result {
        None => body.push(text("请选择用量范围")).into(),
        Some(Err(error)) => body.push(text(format!("分组加载失败：{error}"))).into(),
        Some(Ok((providers, models))) => {
            let mut rows = column![text("供应商 · 请求数 · 所选统计列")].spacing(12);
            let mut providers = providers.iter().collect::<Vec<_>>();
            providers.sort_by_key(|p| std::cmp::Reverse(p.stats.counts.total_requests));
            if providers.is_empty() { rows = rows.push(text("暂无供应商用量")); }
            for provider in providers { rows = rows.push(stats_row(provider_label(&provider.provider), &provider.stats, state)); }
            rows = rows.push(text("模型 · 请求数 · 所选统计列"));
            let mut models = models.iter().collect::<Vec<_>>();
            models.sort_by_key(|m| std::cmp::Reverse(m.stats.counts.total_requests));
            if models.is_empty() { rows = rows.push(text("暂无模型用量")); }
            for model in models { rows = rows.push(stats_row(model.model.as_deref().filter(|m| !m.trim().is_empty()).unwrap_or("未知模型"), &model.stats, state)); }
            let bar = || scrollable::Scrollbar::new().width(4).scroller_width(4);
            body = body.push(scrollable(container(rows).padding([7, 7])).height(Length::Fixed(420.0))
                .direction(scrollable::Direction::Both { vertical: bar(), horizontal: bar() }));
            body.into()
        }
    }
}
