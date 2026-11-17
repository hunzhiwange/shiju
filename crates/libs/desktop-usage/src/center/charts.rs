//! Charts consume server buckets verbatim, preserving empty periods and cost provenance.
use crate::Identity;
use super::hero::Metric;
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

use iced::{widget::{column, container, row, scrollable, text, tooltip, Space, button, svg, stack}, Element, Length, Task};
use gateway_usage_read::{UsageReadScope, UsageSeries, UsageSeriesBucket};
use vw_api_usage::UsageQuery;

#[derive(Default)]
pub struct State {
    pub metric: Metric,
    pub line: bool,
    pub loading: bool,
    pub result: Option<Result<(UsageSeries, UsageSeries), String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Metric(Metric),
    Line(bool),
    Loaded(u64, Identity, Result<(UsageSeries, UsageSeries), String>),
}

pub fn load(client: Result<vw_gateway_client::GatewayClient, String>, ticket: u64, scope: Identity, query: UsageQuery) -> Task<Event> {
    let client = client.map(|client| gateway_usage_read::GatewayUsageReadClient::new(client.transport().clone()));
    Task::perform(async move {
        let result = async {
            let client = client?;
            let read_scope = UsageReadScope {
                project_id: scope.project.clone().into(),
                target_host: scope.host.clone(),
                time_zone: "UTC".into(),
            };
            let trend = client.usage_trends(&read_scope, &query, None).await.map_err(|e| e.to_string())?;
            let heatmap = client.usage_heatmap(&read_scope, &query).await.map_err(|e| e.to_string())?;
            Ok((trend, heatmap))
        }.await;
        (scope, result)
    }, move |(scope, result)| message(Event::Loaded(ticket, scope, result)))
}

pub fn update(state: &mut State, current: Option<Identity>, visible: bool, open: bool, generation: u64, event: Event) -> Task<Event> {
    let (ticket, scope, result) = match event {
        Event::Metric(metric) => { state.metric = metric; return Task::none(); }
        Event::Line(line) => { state.line = line; return Task::none(); }
        Event::Loaded(ticket, scope, result) => (ticket, scope, result),
    };
    if open && generation == ticket
        && current.as_ref() == Some(&scope)
        && visible {
        state.loading = false;
        state.result = Some(result);
    }
    Task::none()
}

fn detail(bucket: &UsageSeriesBucket) -> String {
    let summary = &bucket.summary;
    let tokens = summary.token_groups.iter().map(|tokens| {
        let count = |value: Option<u64>| value.map_or_else(|| "—".into(), |v| v.to_string());
        format!("输入 {} · 输出 {} · 缓存读 {} · 写 {}\n输入 {:?} · 推理 {:?}",
            count(tokens.input), count(tokens.output), count(tokens.cache_read), count(tokens.cache_creation),
            tokens.input_semantics, tokens.reasoning_semantics)
    }).collect::<Vec<_>>().join("\n");
    format!("{}\n请求 {} · 计费 {}\n成功 {} · 失败 {}\n已知费用 {} USD\n未知费用 {} 请求\n{}\nUTC 秒 [{}，{})",
        bucket.label, summary.counts.total_requests, summary.counts.billable_requests,
        summary.succeeded_requests, summary.failed_requests, summary.known_cost_usd,
        summary.counts.unknown_cost_requests,
        if tokens.is_empty() { "无 Token 数据" } else { &tokens }, bucket.start_unix_seconds.0, bucket.end_unix_seconds.0)
}

/// Source heatmap uses quartiles so isolated peaks do not flatten every other day.
pub fn thresholds(values: &[Option<f64>], metric: Metric) -> [f64; 3] {
    let mut values: Vec<_> = values.iter().flatten().copied().filter(|v| v.is_finite() && (*v > 0.0 || metric == Metric::HitRate)).collect();
    values.sort_by(f64::total_cmp);
    if values.is_empty() { return [f64::INFINITY; 3]; }
    [values[values.len()/4], values[values.len()/2], values[values.len()*3/4]]
}
pub fn axis(values: &[Option<f64>], metric: Metric) -> (f64, f64) {
    if metric == Metric::HitRate {
        if values.iter().all(Option::is_none) { return (0.0, 1.0); }
        let min = values.iter().flatten().copied().fold(1.0, f64::min);
        let floor = ((min - ((1.0-min)/2.0).max(0.02))*20.0+1e-9).floor()/20.0;
        return (if floor < 0.5 { 0.0 } else { floor }, 1.0);
    }
    (0.0, values.iter().flatten().copied().fold(0.0, f64::max).max(1.0))
}
fn chart(series: &UsageSeries, heatmap: bool, metric: Metric, line: bool) -> Element<'_, Event> {
    if series.buckets.is_empty() { return text("暂无用量桶").into(); }
    let values: Vec<_> = series.buckets.iter().map(|b| metric.value(&b.summary)).collect();
    let levels = thresholds(&values, metric);
    let (floor, ceiling) = axis(&values, metric);
    let mut cells = row![].spacing(if heatmap { 3 } else { 0 });
    let mut week = column![].spacing(3);
    let first_weekday = (series.buckets[0].start_unix_seconds.0.div_euclid(86400) + 3).rem_euclid(7) as usize;
    if heatmap { for _ in 0..first_weekday { week = week.push(Space::new().height(12).width(12)); } }
    let width = (series.buckets.len() as f64 * 18.0).max(600.0);
    let step = width / series.buckets.len() as f64;
    let mut drawing = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="108" viewBox="0 0 {width} 108">"##);
    for y in [4, 54, 104] { drawing.push_str(&format!(r##"<path d="M0 {y} H{width}" stroke="#82949d" stroke-opacity="0.2"/>"##)); }
    let mut previous = None;
    for (index, bucket) in series.buckets.iter().enumerate() {
        let value = values[index];
        let ratio = value.map_or(0.0, |v| ((v-floor)/(ceiling-floor)).clamp(0.0,1.0));
        let card = container(text(format!("{} · {}\n{}", metric.label(), metric.format(value), detail(bucket))).size(12)).padding(10).style(container::rounded_box);
        if heatmap {
            let level = value.filter(|v| *v > 0.0 || metric == Metric::HitRate).map_or(0, |v| 1 + levels.iter().filter(|threshold| v > **threshold).count());
            let mark = container(Space::new().width(12).height(12)).style(move |theme: &iced::Theme| {
                let color = if level == 0 { theme.extended_palette().background.strong.color } else { iced::Color::from_rgba(0.0, 0.68, 0.73, 0.2 + level as f32 * 0.2) };
                container::Style::default().background(color).border(iced::Border { radius: 3.0.into(), ..Default::default() })
            });
            week = week.push(tooltip(mark, card, tooltip::Position::Top).snap_within_viewport(true));
            if (first_weekday+index+1)%7 == 0 { cells = cells.push(week); week = column![].spacing(3); }
        } else {
            let x = (index as f64 + 0.5)*step;
            let y = 104.0-ratio*100.0;
            if value.is_some() {
                if line {
                    if let Some((px, py)) = previous { drawing.push_str(&format!(r##"<path d="M{px} {py} L{x} {y}" fill="none" stroke="#00adb8" stroke-width="2"/>"##)); }
                    drawing.push_str(&format!(r##"<circle cx="{x}" cy="{y}" r="2" fill="#00adb8"/>"##));
                } else {
                    let bar = step.min(36.0)*0.75;
                    drawing.push_str(&format!(r##"<rect x="{}" y="{y}" width="{bar}" height="{}" rx="3" fill="#00adb8"/>"##,x-bar/2.0,ratio*100.0));
                }
                previous = Some((x,y));
            } else { previous = None; }
            cells = cells.push(tooltip(container(Space::new().height(108)).width(Length::Fill),card,tooltip::Position::Top).snap_within_viewport(true));
        }
    }
    if heatmap && (first_weekday+series.buckets.len())%7 != 0 { cells = cells.push(week); }
    if heatmap {
        return scrollable(container(row![column![text("一").size(10), Space::new().height(14), text("三").size(10), Space::new().height(14), text("五").size(10), Space::new().height(14), text("日").size(10)], cells].spacing(7)).padding([7,7]))
            .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4).scroller_width(4))).into();
    }
    drawing.push_str("</svg>");
    let plot = stack![svg(svg::Handle::from_memory(drawing.into_bytes())).width(Length::Fill).height(108).content_fit(iced::ContentFit::Fill), cells];
    let mut labels = row![].spacing(0);
    let stride = series.buckets.len().div_ceil(6);
    for (index, bucket) in series.buckets.iter().enumerate() {
        if index % stride == 0 { labels = labels.push(text(bucket.label.clone()).size(10).width(Length::Fill)); }
    }
    let ticks = column![text(metric.format(Some(ceiling))).size(10), Space::new().height(30), text(metric.format(Some((floor+ceiling)/2.0))).size(10), Space::new().height(30), text(metric.format(Some(floor))).size(10)].width(90);
    container(row![ticks, column![plot, labels].spacing(6).width(Length::Fill)].spacing(8)).padding([7, 7]).into()
}

pub fn view(state: &State) -> Element<'_, Event> { view_metric(state, state.metric) }
/// Center integration can supply the hero selection without reloading server buckets.
pub fn view_metric(state: &State, metric: Metric) -> Element<'_, Event> {
    let mut controls = row![].spacing(8);
    for item in Metric::ALL { controls = controls.push(button(text(item.label()).size(12)).padding([6,10]).style(rounded_action_btn_style).on_press(Event::Metric(item))); }
    controls = controls.push(button(text(if state.line { "切换柱状图" } else { "切换折线图" }).size(12)).padding([6,10]).style(rounded_action_btn_style).on_press(Event::Line(!state.line)));
    let body = column![controls].spacing(12);
    if state.loading { return body.push(text("正在加载图表…")).into(); }
    match &state.result {
        None => body.push(text("请选择用量范围")).into(),
        Some(Err(error)) => body.push(text(format!("图表加载失败：{error}"))).into(),
        Some(Ok((trend, heatmap))) => body.push(text(format!("{}趋势 · {:?} · UTC",metric.label(),trend.interval)).size(12))
            .push(chart(trend,false,metric,state.line)).push(text(format!("每日热力图 · {}",metric.label())).size(12))
            .push(chart(heatmap,true,metric,false)).push(text("少 ░ ▒ ▓ 多 · 悬停查看日期与数值；未知数据保留为空").size(11)).into(),
    }
}
fn message(event: Event) -> Event { event }

#[cfg(test)]
#[path = "../usage_replica_03_tests.rs"]
mod usage_replica_03_tests;
