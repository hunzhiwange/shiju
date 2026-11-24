//! Shared metric selection and summary strip; callers pass the same metric to charts.
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{icon_svg, rounded_action_btn_style};
use gateway_usage_read::UsageSummary;
use iced::{
    Element, Font, Length,
    font::Weight,
    widget::{Space, button, column, container, row, rule, stack, text, tooltip},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metric {
    #[default]
    Cost,
    Requests,
    Tokens,
    HitRate,
}
impl Metric {
    pub const ALL: [Self; 4] = [Self::Cost, Self::Requests, Self::Tokens, Self::HitRate];
    pub fn label(self) -> &'static str {
        match self {
            Self::Cost => "总成本",
            Self::Requests => "总请求数",
            Self::Tokens => "真实消耗 Tokens",
            Self::HitRate => "缓存命中率",
        }
    }
    pub fn value(self, summary: &UsageSummary) -> Option<f64> {
        if summary.counts.total_requests == 0 {
            return if self == Self::HitRate { None } else { Some(0.0) };
        }
        let tokens = summary.token_totals();
        match self {
            Self::Cost => {
                summary.known_cost_usd.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
            }
            Self::Requests => Some(summary.counts.total_requests as f64),
            Self::Tokens => tokens.real_tokens.map(|n| n as f64),
            Self::HitRate => {
                let denominator = tokens
                    .fresh_input?
                    .checked_add(tokens.cache_read?)?
                    .checked_add(tokens.cache_creation?)?;
                (denominator > 0)
                    .then(|| tokens.cache_read.unwrap_or(0) as f64 / denominator as f64)
            }
        }
    }
    pub fn format(self, value: Option<f64>) -> String {
        let Some(value) = value.filter(|v| v.is_finite()) else {
            return "—".into();
        };
        match self {
            Self::Cost => format!("{value:.4} USD"),
            Self::HitRate => super::metrics::format_hit_rate(Some(value)),
            _ => format!("{value:.0}"),
        }
    }
}
pub(super) fn compact_count(value: f64) -> String {
    for (divisor, suffix) in
        [(1e18, "E"), (1e15, "P"), (1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")]
    {
        if value >= divisor {
            let number = format!("{:.2}", value / divisor);
            return format!("{}{suffix}", number.trim_end_matches('0').trim_end_matches('.'));
        }
    }
    format!("{value:.0}")
}

#[derive(Clone, Debug)]
pub enum Event {
    Select(Metric),
    ToggleDetails,
}
pub struct State {
    pub metric: Metric,
    pub expanded: bool,
}
impl Default for State {
    fn default() -> Self {
        Self { metric: Metric::Tokens, expanded: false }
    }
}

pub fn update(state: &mut State, event: Event) {
    match event {
        Event::Select(metric) => state.metric = metric,
        Event::ToggleDetails => state.expanded = !state.expanded,
    }
}
pub fn view<'a>(i18n: desktop_i18n::I18n,
    state: &State,
    summary: Option<&UsageSummary>,
    loading: bool,
) -> Element<'a, Event> {
    view_with_cost(i18n, state, summary, loading, None)
}
pub fn view_with_cost<'a>(i18n: desktop_i18n::I18n,
    state: &State,
    summary: Option<&UsageSummary>,
    loading: bool,
    cost: Option<&str>,
) -> Element<'a, Event> {
    let mut cells = row![];
    for metric in Metric::ALL {
        let value = if loading {
            "…".into()
        } else if summary.is_none_or(|s| s.counts.total_requests == 0) {
            "—".into()
        } else if metric == Metric::Cost {
            cost.map(str::to_owned)
                .unwrap_or_else(|| metric.format(summary.and_then(|s| metric.value(s))))
        } else if matches!(metric, Metric::Requests | Metric::Tokens) {
            summary.and_then(|s| metric.value(s)).map_or_else(|| "—".into(), compact_count)
        } else {
            metric.format(summary.and_then(|s| metric.value(s)))
        };
        let display_value = if metric == Metric::Cost {
            value.split_once('（').map_or(value.as_str(), |(amount, _)| amount)
        } else {
            &value
        };
        let selected = state.metric == metric;
        let label: Element<'_, Event> = if selected {
            text(i18n.__(metric.label())).size(13).into()
        } else {
            text(i18n.__(metric.label())).size(13).style(text::secondary).into()
        };
        let cell = column![
            column![
                label,
                text(display_value.to_owned())
                    .size(28)
                    .font(Font { weight: Weight::Semibold, ..Font::DEFAULT })
            ]
            .spacing(5)
            .padding([12, 14]),
            container(Space::new().height(2)).width(Length::Fill).style(
                move |theme: &iced::Theme| {
                    container::Style::default().background(if selected {
                        theme.palette().text
                    } else {
                        iced::Color::TRANSPARENT
                    })
                }
            )
        ];
        let help = match metric {
            Metric::Cost => i18n.format("已知费用 %s\n未知费用 %s 请求", &[(value.to_string()).into(), (format!("{}", summary.map_or(0, |s| s.counts.unknown_cost_requests))).into()]).expect("valid display template"),
            Metric::Tokens => {
                i18n.__("新增输入 + 输出 + 缓存写入 + 缓存命中；未知计数显示 —，不重复计算推理或缓存子集。")
                    .into()
            }
            Metric::HitRate => {
                i18n.__("缓存命中 ÷（新增输入 + 缓存写入 + 缓存命中）；无输入时显示 —。").into()
            }
            Metric::Requests => i18n.__("当前范围内全部请求，包含失败与未知状态。").into(),
        };
        let help = if matches!(metric, Metric::Requests | Metric::Tokens) && !loading {
            format!(
                "{} · {}\n{help}",
                metric.label(),
                metric.format(summary.and_then(|s| metric.value(s)))
            )
        } else {
            help
        };
        cells = cells.push(
            tooltip(
                button(cell)
                    .padding(0)
                    .width(Length::Fill)
                    .style(move |theme, status| {
                        let mut style = rounded_action_btn_style(theme, status);
                        style.border = iced::border::rounded(9);
                        let ink = theme.palette().text;
                        style.background = match status {
                            button::Status::Hovered => Some(ink.scale_alpha(0.025).into()),
                            button::Status::Pressed => Some(ink.scale_alpha(0.048).into()),
                            _ => None,
                        };
                        style.text_color = ink;
                        style
                    })
                    .on_press(Event::Select(metric)),
                text(help).size(12),
                tooltip::Position::Top,
            )
            .gap(5)
            .padding(10)
            .style(super::view::metric_tooltip_style)
            .snap_within_viewport(true),
        );
    }
    let disclosure_icon = if state.expanded { Icon::ChevronUp } else { Icon::ChevronDown };
    let disclosure = tooltip(
        button(icon_svg(disclosure_icon, 11.0))
            .padding(6)
            .style(|theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                let palette = theme.extended_palette();
                style.border.radius = 12.0.into();
                style.background = Some(match status {
                    button::Status::Hovered | button::Status::Pressed => palette.background.strong,
                    _ => palette.background.weak,
                }.color.into());
                style
            })
            .on_press(Event::ToggleDetails),
        container(text(i18n.__("更多指标")).size(12))
            .padding([6, 12])
            .style(|theme: &iced::Theme| {
                let palette = theme.palette();
                container::Style::default()
                    .background(palette.text)
                    .color(palette.background)
                    .border(iced::border::rounded(16))
            }),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .padding(0)
    .style(|_| container::Style::default())
    .snap_within_viewport(true);
    let mut body = column![cells];
    if state.expanded {
        let totals = summary.map(UsageSummary::token_totals).unwrap_or_default();
        let mut details = row![].spacing(0);
        for (label, value) in [
            (i18n.__("新增输入"), totals.fresh_input),
            (i18n.__("输出"), totals.output),
            (i18n.__("缓存写入"), totals.cache_creation),
            (i18n.__("缓存命中"), totals.cache_read),
        ] {
            let value = if loading {
                "…".into()
            } else {
                value.map_or_else(|| "—".into(), |n| n.to_string())
            };
            details = details.push(
                container(column![text(label).size(12), text(value).size(16)].spacing(4))
                    .padding([10, 14])
                    .width(Length::Fill),
            );
        }
        body = body.push(rule::horizontal(1)).push(details);
    }
    stack![
        body.push(rule::horizontal(1)).push(Space::new().height(11)),
        container(disclosure)
            .align_bottom(Length::Fill)
            .center_x(Length::Fill),
    ]
    .into()
}
