//! Charts consume server buckets verbatim, preserving empty periods and cost provenance.
use super::{hero::Metric, metrics::format_count};
use crate::Identity;
use desktop_ui_controls::system_settings_common::{rounded_action_btn_style, theme::is_dark_theme};
use gateway_usage_read::{UsageReadScope, UsageSeries, UsageSeriesBucket, UsageSeriesInterval};
use iced::{
    Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, Task, Theme,
    advanced::text::Alignment,
    alignment::Vertical,
    border, mouse,
    widget::{Space, button, canvas, column, container, row, scrollable, stack, text, tooltip},
};
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
const PLOT_TOP: f32 = 12.0;
const PLOT_LEFT: f32 = 58.0;
const PLOT_RIGHT: f32 = 22.0;
const PLOT_BOTTOM: f32 = 28.0;
const CHART_HEIGHT: f32 = 200.0;

fn accent(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        Color::from_rgb8(70, 199, 217)
    } else {
        Color::from_rgb8(13, 132, 155)
    }
}

fn bucket_tooltip(
    mark: impl Into<Element<'static, Event>>,
    bucket: &UsageSeriesBucket,
    metric: Metric,
    value: Option<f64>,
) -> Element<'static, Event> {
    let summary = &bucket.summary;
    let (label, amount) = match metric {
        Metric::Requests => ("请求数", format_count(summary.counts.total_requests)),
        Metric::Tokens => (
            "Tokens",
            summary.token_totals().real_tokens.map_or_else(
                || metric.format(value),
                format_count,
            ),
        ),
        Metric::Cost => ("已知费用", format!("{} USD", summary.known_cost_usd)),
        Metric::HitRate => ("缓存命中率", metric.format(value)),
    };
    let swatch = container(Space::new().width(8).height(8)).style(|theme: &Theme| {
        container::Style::default().background(accent(theme)).border(border::rounded(2))
    });
    let content = column![
        text(bucket.label.replace('-', "/")).size(13),
        row![swatch, text(label).size(13).style(text::secondary), text(amount).size(14)]
            .spacing(8)
            .align_y(iced::Alignment::Center),
    ]
    .spacing(7);
    tooltip(mark, content, tooltip::Position::FollowCursor)
        .padding(11)
        .style(super::view::metric_tooltip_style)
        .snap_within_viewport(true)
        .into()
}

struct Trend<'a> {
    series: &'a UsageSeries,
    values: Vec<Option<f64>>,
    metric: Metric,
    line: bool,
}

impl canvas::Program<Event> for Trend<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let right = bounds.width - PLOT_RIGHT;
        let bottom = bounds.height - PLOT_BOTTOM;
        let width = right - PLOT_LEFT;
        if width <= 0.0 || bottom <= PLOT_TOP || self.values.is_empty() {
            return Vec::new();
        }

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let step = width / self.values.len() as f32;
        let height = bottom - PLOT_TOP;
        let x_at = |index: usize| PLOT_LEFT + (index as f32 + 0.5) * step;
        let (floor, mut ceiling) = axis(&self.values, self.metric);
        // Round count axes so the midpoint is a whole number.
        if matches!(self.metric, Metric::Requests | Metric::Tokens) {
            let half = ceiling / 2.0;
            let unit = 10_f64.powf(half.log10().floor()).max(1.0);
            ceiling = (half / unit).ceil() * unit * 2.0;
        }

        let ink = theme.palette().text;
        let grid = ink.scale_alpha(0.14);
        let muted = ink.scale_alpha(0.75);
        let color = accent(theme);
        let hovered = cursor
            .position_in(bounds)
            .filter(|point| {
                (PLOT_LEFT..right).contains(&point.x) && (PLOT_TOP..bottom).contains(&point.y)
            })
            .map(|point| (((point.x - PLOT_LEFT) / step) as usize).min(self.values.len() - 1));
        if let Some(index) = hovered {
            frame.fill_rectangle(
                Point::new(x_at(index) - step / 2.0, PLOT_TOP),
                Size::new(step, height),
                color.scale_alpha(0.09),
            );
        }
        for fraction in [0.0, 0.5, 1.0] {
            let y = bottom - height * fraction as f32;
            frame.fill_rectangle(Point::new(PLOT_LEFT, y), Size::new(width, 1.0), grid);
            let value = floor + fraction * (ceiling - floor);
            let label = match self.metric {
                Metric::Cost if value >= 1e6 || (value > 0.0 && value < 0.0001) => {
                    format!("{value:.1e}")
                }
                Metric::Cost => {
                    format!("{value:.4}").trim_end_matches('0').trim_end_matches('.').into()
                }
                Metric::HitRate => self.metric.format(Some(value)),
                _ => super::hero::compact_count(value),
            };
            frame.fill_text(canvas::Text {
                content: label,
                position: Point::new(PLOT_LEFT - 8.0, y),
                align_x: Alignment::Right,
                align_y: Vertical::Center,
                color: muted,
                size: Pixels(11.0),
                ..Default::default()
            });
        }

        let mut previous = None;
        for (index, value) in self.values.iter().enumerate() {
            let Some(value) = value else {
                previous = None;
                continue;
            };
            let bar_height =
                (((value - floor) / (ceiling - floor)) as f32).clamp(0.0, 1.0) * height;
            let point = Point::new(x_at(index), bottom - bar_height);
            if self.line {
                if let Some(previous) = previous {
                    frame.stroke(
                        &canvas::Path::line(previous, point),
                        canvas::Stroke::default().with_color(color).with_width(2.0),
                    );
                }
                frame.fill(&canvas::Path::circle(point, 2.25), color);
                previous = Some(point);
            } else if bar_height > 0.0 {
                let bar_width = (step * 0.76).min(36.0);
                let bar_height = bar_height.max(1.0);
                frame.fill(
                    &canvas::Path::rounded_rectangle(
                        Point::new(point.x - bar_width / 2.0, bottom - bar_height),
                        Size::new(bar_width, bar_height),
                        border::top(4.0_f32.min(bar_width / 2.0).min(bar_height)),
                    ),
                    color,
                );
            }
        }

        let count = self.values.len();
        let ticks = count.min(((width / 56.0) as usize).max(1));
        for tick in 0..ticks {
            let index = tick * (count - 1) / (ticks - 1).max(1);
            let label = &self.series.buckets[index].label;
            // Labels are generated by the server in the series timezone.
            let label = match self.series.interval {
                UsageSeriesInterval::Day => label.get(5..10).map(|date| date.replace('-', "/")),
                UsageSeriesInterval::Hour => label.get(11..16).map(str::to_owned),
            };
            if let Some(content) = label {
                frame.fill_text(canvas::Text {
                    content,
                    position: Point::new(x_at(index), bottom + 8.0),
                    align_x: Alignment::Center,
                    color: muted,
                    size: Pixels(11.0),
                    ..Default::default()
                });
            }
        }
        vec![frame.into_geometry()]
    }
}

fn trend_chart(series: &UsageSeries, metric: Metric, line: bool) -> Element<'_, Event> {
    let values: Vec<_> =
        series.buckets.iter().map(|bucket| metric.value(&bucket.summary)).collect();
    let mut targets = row![].width(Length::Fill);
    for (bucket, value) in series.buckets.iter().zip(&values) {
        let mark = Space::new().width(Length::Fill).height(CHART_HEIGHT - PLOT_TOP - PLOT_BOTTOM);
        targets = targets.push(bucket_tooltip(mark, bucket, metric, *value));
    }
    stack![
        canvas(Trend { series, values, metric, line }).width(Length::Fill).height(CHART_HEIGHT),
        container(targets).padding(iced::Padding {
            top: PLOT_TOP,
            right: PLOT_RIGHT,
            bottom: PLOT_BOTTOM,
            left: PLOT_LEFT,
        }),
    ]
    .into()
}

fn heatmap_chart(series: &UsageSeries, metric: Metric) -> Element<'_, Event> {
    let Some(first) = series.buckets.first() else {
        return text("暂无用量记录").style(text::secondary).into();
    };
    let values: Vec<_> =
        series.buckets.iter().map(|bucket| metric.value(&bucket.summary)).collect();
    let levels = thresholds(&values, metric);
    let first_weekday = (first.start_unix_seconds.0.div_euclid(86400) + 3).rem_euclid(7) as usize;
    let mut weeks = row![].spacing(3);
    let mut week = column![].spacing(3);
    for _ in 0..first_weekday {
        week = week.push(Space::new().width(12).height(12));
    }
    for (index, (bucket, value)) in series.buckets.iter().zip(values).enumerate() {
        let level = value
            .filter(|v| *v > 0.0 || metric == Metric::HitRate)
            .map_or(0, |v| 1 + levels.iter().filter(|threshold| v > **threshold).count());
        let mark = container(Space::new().width(12).height(12)).style(move |theme: &Theme| {
            let color = if level == 0 {
                theme.extended_palette().background.strong.color
            } else {
                accent(theme).scale_alpha(0.2 + level as f32 * 0.2)
            };
            container::Style::default().background(color).border(border::rounded(3))
        });
        week = week.push(bucket_tooltip(mark, bucket, metric, value));
        if (first_weekday + index + 1) % 7 == 0 {
            weeks = weeks.push(week);
            week = column![].spacing(3);
        }
    }
    if (first_weekday + series.buckets.len()) % 7 != 0 {
        weeks = weeks.push(week);
    }
    let mut days = column![].spacing(3);
    for label in ["一", "", "三", "", "五", "", "日"] {
        days = days.push(text(label).size(10).height(12).style(text::secondary));
    }
    scrollable(container(row![days, weeks].spacing(7)).padding(7))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
}

pub fn view(state: &State) -> Element<'_, Event> {
    view_metric(state, state.metric)
}

pub fn view_metric(state: &State, metric: Metric) -> Element<'_, Event> {
    view_range(state, metric, None)
}

/// The global page uses summary metrics as controls and shows the relevant chart.
pub fn view_range(state: &State, metric: Metric, all: Option<bool>) -> Element<'_, Event> {
    let mut body = column![].spacing(12).width(Length::Fill);
    if all.is_none() {
        let mut controls = row![].spacing(8);
        for item in Metric::ALL {
            controls = controls.push(
                button(text(item.label()).size(12))
                    .padding([6, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Event::Metric(item)),
            );
        }
        body = body.push(controls);
    }

    if state.loading {
        return body.push(text("正在加载图表…").style(text::secondary)).into();
    }
    match &state.result {
        None => body.push(text("请选择用量范围")).into(),
        Some(Err(error)) => body.push(text(format!("图表加载失败：{error}"))).into(),
        Some(Ok((trend, heatmap))) => {
            if all != Some(true) {
                let interval = match trend.interval {
                    UsageSeriesInterval::Day => "按天",
                    UsageSeriesInterval::Hour => "按小时",
                };
                let unit = if metric == Metric::Cost { " · USD" } else { "" };
                let header = row![
                    text(format!("{} · {interval}{unit}", metric.label())).size(14),
                    text(&trend.range.time_zone).size(11).style(text::secondary),
                    Space::new().width(Length::Fill),
                    button(text(if state.line { "柱状图" } else { "折线图" }).size(12))
                        .padding([5, 10])
                        .style(rounded_action_btn_style)
                        .on_press(Event::Line(!state.line)),
                ]
                .spacing(8)
                .align_y(Vertical::Center);
                body = body.push(header);
                if trend.buckets.is_empty() {
                    body = body.push(text("暂无用量记录").style(text::secondary));
                } else {
                    body = body.push(trend_chart(trend, metric, state.line));
                }
            }
            if all != Some(false) {
                body = body
                    .push(text(format!("每日热力图 · {}", metric.label())).size(14))
                    .push(heatmap_chart(heatmap, metric));
            }
            body.into()
        }
    }
}
fn message(event: Event) -> Event {
    event
}

#[cfg(test)]
#[path = "../usage_replica_03_tests.rs"]
mod usage_replica_03_tests;
