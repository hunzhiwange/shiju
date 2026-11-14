//! Charts consume server buckets verbatim, preserving empty periods and cost provenance.
use super::{identity, Event as CenterEvent, Identity};
use crate::app::{message::ViewMessage, App, Message};
use iced::{widget::{column, container, row, scrollable, text, tooltip, Space}, Element, Length, Task};
use vw_gateway_client::client::{UsageReadScope, UsageSeries, UsageSeriesBucket};
use vw_gateway_client::vw_api_types::usage::UsageQuery;

#[derive(Default)]
pub struct State {
    pub loading: bool,
    pub result: Option<Result<(UsageSeries, UsageSeries), String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Loaded(u64, Identity, Result<(UsageSeries, UsageSeries), String>),
}

pub fn load(ticket: u64, scope: Identity, query: UsageQuery) -> Task<Message> {
    let client = crate::app::config::gateway_client();
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
    }, move |(scope, result)| Message::View(ViewMessage::UsageCenter(
        CenterEvent::Charts(Event::Loaded(ticket, scope, result)))))
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let Event::Loaded(ticket, scope, result) = event;
    if app.usage_center.open && app.usage_center.sequence == ticket
        && identity(app).as_ref() == Some(&scope)
        && app.screen == crate::app::Screen::Usage {
        app.usage_center.charts.loading = false;
        app.usage_center.charts.result = Some(result);
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

fn chart(series: &UsageSeries, heatmap: bool) -> Element<'_, Message> {
    if series.buckets.is_empty() {
        return text("暂无用量桶").into();
    }
    let maximum = series.buckets.iter().map(|b| b.summary.counts.total_requests).max().unwrap_or(0).max(1);
    let mut cells = row![].spacing(if heatmap { 4 } else { 8 });
    let mut week = column![].spacing(4);
    // UTC calendar alignment: Unix epoch was Thursday, Monday is row zero.
    let first_weekday = (series.buckets[0].start_unix_seconds.0.div_euclid(86400) + 3).rem_euclid(7) as usize;
    if heatmap {
        for _ in 0..first_weekday { week = week.push(Space::new().height(28).width(28)); }
    }
    for (index, bucket) in series.buckets.iter().enumerate() {
        let count = bucket.summary.counts.total_requests;
        let ratio = count as f32 / maximum as f32;
        let height = if heatmap { 28.0 } else { (ratio * 100.0).max(2.0) };
        let mark = container(Space::new().width(Length::Fill).height(height))
            .width(if heatmap { 28.0 } else { 64.0 })
            .style(move |theme: &iced::Theme| {
                let palette = theme.extended_palette();
                let mut color = palette.primary.strong.color;
                if heatmap && count > 0 { color.a = 0.35 + ratio * 0.65; }
                container::Style::default()
                    .background(if count == 0 { palette.background.strong.color } else { color })
                    .border(iced::Border { color: palette.background.strong.text, width: 1.0, radius: 3.0.into() })
            });
        let card = container(text(detail(bucket)).size(13)).padding(10).style(container::rounded_box);
        if heatmap {
            week = week.push(tooltip(mark, card, tooltip::Position::Top).snap_within_viewport(true));
            if (first_weekday + index + 1) % 7 == 0 {
                cells = cells.push(week);
                week = column![].spacing(4);
            }
        } else {
            let cell = column![
                container(mark).height(104.0).align_y(iced::alignment::Vertical::Bottom),
                text(count.to_string()).size(11),
                text(bucket.label.clone()).size(11).width(100),
            ].spacing(4);
            cells = cells.push(tooltip(cell, card, tooltip::Position::Top).snap_within_viewport(true));
        }
    }
    if heatmap && (first_weekday + series.buckets.len()) % 7 != 0 { cells = cells.push(week); }

    scrollable(container(cells).padding([7, 7]))
        .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4).scroller_width(4)))
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let state = &app.usage_center.charts;
    let mut body = column![text("请求趋势与每日热力图 · UTC"), text("周一至周日由上至下，颜色越深请求越多；空桶保留，悬浮查看费用与完整时间。窄窗口可横向滚动。").size(12)].spacing(8);
    if state.loading { return body.push(text("正在加载图表…")).into(); }
    match &state.result {
        None => body.push(text("请选择用量范围")).into(),
        Some(Err(error)) => body.push(text(format!("图表加载失败：{error}"))).into(),
        Some(Ok((trend, heatmap))) => {
            body = body.push(text(format!("趋势 · {:?}", trend.interval))).push(chart(trend, false))
                .push(text("每日热力图 · 请求数")).push(chart(heatmap, true));
            body.into()
        }
    }
}
