//! Shared metric selection and summary strip; callers pass the same metric to charts.
use gateway_usage_read::UsageSummary;
use iced::{widget::{button, column, container, row, text, tooltip, Space}, Element, Length};
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metric { #[default] Cost, Requests, Tokens, HitRate }
impl Metric {
    pub const ALL: [Self; 4] = [Self::Cost, Self::Requests, Self::Tokens, Self::HitRate];
    pub fn label(self) -> &'static str {
        match self { Self::Cost => "总成本", Self::Requests => "总请求数", Self::Tokens => "真实消耗 Tokens", Self::HitRate => "缓存命中率" }
    }
    pub fn value(self, summary: &UsageSummary) -> Option<f64> {
        if summary.counts.total_requests == 0 { return if self == Self::HitRate { None } else { Some(0.0) }; }
        let tokens = summary.token_totals();
        match self {
            Self::Cost => summary.known_cost_usd.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0),
            Self::Requests => Some(summary.counts.total_requests as f64),
            Self::Tokens => tokens.real_tokens.map(|n| n as f64),
            Self::HitRate => {
                let denominator = tokens.fresh_input?.checked_add(tokens.cache_read?)?.checked_add(tokens.cache_creation?)?;
                (denominator > 0).then(|| tokens.cache_read.unwrap_or(0) as f64 / denominator as f64)
            }
        }
    }
    pub fn format(self, value: Option<f64>) -> String {
        let Some(value) = value.filter(|v| v.is_finite()) else { return "—".into(); };
        match self { Self::Cost => format!("{value:.4} USD"), Self::HitRate => super::metrics::format_hit_rate(Some(value)), _ => format!("{value:.0}") }
    }
}
#[derive(Clone, Debug)]
pub enum Event { Select(Metric), ToggleDetails }
#[derive(Default)]
pub struct State { pub metric: Metric, pub expanded: bool }
pub fn update(state: &mut State, event: Event) {
    match event { Event::Select(metric) => state.metric = metric, Event::ToggleDetails => state.expanded = !state.expanded }
}
pub fn view<'a>(state: &State, summary: Option<&UsageSummary>, loading: bool) -> Element<'a, Event> {
    view_with_cost(state, summary, loading, None)
}
pub fn view_with_cost<'a>(state: &State, summary: Option<&UsageSummary>, loading: bool, cost: Option<&str>) -> Element<'a, Event> {
    let mut cells = row![].spacing(0);
    for metric in Metric::ALL {
        let value = if loading { "…".into() } else if summary.is_none_or(|s| s.counts.total_requests == 0) { "—".into() }
            else if metric == Metric::Cost { cost.map(str::to_owned).unwrap_or_else(|| metric.format(summary.and_then(|s| metric.value(s)))) }
            else { metric.format(summary.and_then(|s| metric.value(s))) };
        let selected = state.metric == metric;
        let cell = column![text(metric.label()).size(12), text(value).size(24),
            container(Space::new().height(2)).width(Length::Fill).style(move |theme: &iced::Theme| {
                container::Style::default().background(if selected { theme.palette().text } else { iced::Color::TRANSPARENT })
            })].spacing(4);
        let help = match metric {
            Metric::Cost => format!("已知费用小计；未知费用 {} 请求", summary.map_or(0, |s| s.counts.unknown_cost_requests)),
            Metric::Tokens => "新增输入 + 输出 + 缓存写入 + 缓存命中；未知计数显示 —，不重复计算推理或缓存子集。".into(),
            Metric::HitRate => "缓存命中 ÷（新增输入 + 缓存写入 + 缓存命中）；无输入时显示 —。".into(),
            Metric::Requests => "当前范围内全部请求，包含失败与未知状态。".into(),
        };
        cells = cells.push(tooltip(button(cell).padding([10, 16]).width(Length::Fill).style(rounded_action_btn_style).on_press(Event::Select(metric)), container(text(help).size(12)).padding(10).style(container::rounded_box), tooltip::Position::Top));
    }
    let mut body = column![cells].spacing(4);
    if state.expanded {
        let totals = summary.map(UsageSummary::token_totals).unwrap_or_default();
        let mut details = row![].spacing(0);
        for (label, value) in [("新增输入", totals.fresh_input), ("输出", totals.output), ("缓存写入", totals.cache_creation), ("缓存命中", totals.cache_read)] {
            let value = if loading { "…".into() } else { value.map_or_else(|| "—".into(), |n| n.to_string()) };
            details = details.push(container(column![text(label).size(12), text(value).size(16)].spacing(4)).padding([10,16]).width(Length::Fill));
        }
        body = body.push(details);
    }
    body.push(container(button(text(if state.expanded { "收起明细 ∧" } else { "展开明细 ∨" }).size(11)).padding([4, 10]).style(rounded_action_btn_style).on_press(Event::ToggleDetails)).center_x(Length::Fill)).into()
}
