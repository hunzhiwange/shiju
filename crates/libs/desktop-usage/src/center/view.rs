use super::{display, event::Event, overview, pricing, scan, state::DetailTab};
use crate::models;
use desktop_assets::assets::{Icon, get_icon};
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
use iced::{
    Element, Length, Theme,
    widget::{Space, button, column, container, row, scrollable, svg, text},
};

pub fn view<'a>(
    state: &'a super::state::State,
    status: String,
    known_cost: Option<String>,
) -> Element<'a, Event> {
    let action = |label: &'static str, event| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press(event)
    };
    if !state.open {
        return action("第三方智能体用量统计 ›", Event::Load(super::range_picker::Preset::Month))
            .into();
    }
    let header = row![
        svg(get_icon(Icon::BarChartFill))
            .width(22)
            .height(22)
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) }),
        text("用量统计").size(25),
        Space::new().width(Length::Fill),
        action("刷新", Event::Refresh),
        action("同步", Event::Sync),
        action("设置", Event::Scan(scan::Event::Open)),
        action("返回当前会话", Event::Close),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);
    let mut options = vec![overview::ScopeOption {
        label: "全部供应商 / 模型".into(),
        provider: None,
        model: None,
    }];
    if let Some(Ok((providers, models))) = &state.groups.result {
        for provider in providers {
            let label = match &provider.provider {
                vw_api_usage::UsageProvider::Unknown => "未知供应商".into(),
                vw_api_usage::UsageProvider::Known { provider_id } => provider_id.clone(),
            };
            options.push(overview::ScopeOption {
                label,
                provider: Some(provider.provider.clone()),
                model: None,
            });
        }
        for model in models {
            if let Some(name) = &model.model {
                options.push(overview::ScopeOption {
                    label: name.clone(),
                    provider: state.query.filter.provider.clone(),
                    model: Some(name.clone()),
                });
            }
        }
    }
    // Own the option vector in the picker; do not return a borrow of local data.
    let filters = overview::view_with_options(&state.query, &options).map(Event::Filter);
    let summary = state.result.as_ref().and_then(|r| r.as_ref().ok());
    let hero =
        super::hero::view_with_cost(&state.hero, summary, state.loading, known_cost.as_deref())
            .map(Event::Hero);
    let mut chart_section = column![].width(Length::Fill).spacing(8);
    match &state.result {
        Some(Err(error)) => {
            chart_section = chart_section.push(text(format!("用量加载失败：{error}")).size(12));
        }
        Some(Ok(summary)) if summary.counts.unknown_cost_requests > 0 => {
            chart_section = chart_section.push(
                text(format!("{} 个请求的费用未知", summary.counts.unknown_cost_requests))
                    .size(12)
                    .style(text::secondary),
            );
        }
        _ => {}
    }
    let chart = super::charts::view_range(
        &state.charts,
        state.hero.metric,
        Some(matches!(&state.query.range, vw_api_usage::UsageRange::All)),
    )
    .map(Event::Charts);
    let overview_card = container(column![hero, chart_section.push(chart).padding([14, 16])])
        .width(Length::Fill)
        .clip(true)
        .style(panel_style);
    let mut tabs = row![].spacing(8);
    for (tab, label) in [
        (DetailTab::Logs, "请求日志"),
        (DetailTab::Providers, "供应商"),
        (DetailTab::Models, "模型"),
    ] {
        let selected = state.tab == tab;
        tabs = tabs.push(
            button(text(label).size(14))
                .padding([8, 12])
                .style(move |theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    if selected {
                        let palette = theme.extended_palette();
                        style.background = Some(palette.background.strong.color.into());
                        style.text_color = palette.background.base.text;
                    }
                    style
                })
                .on_press(Event::Tab(tab)),
        );
    }
    let detail = match state.tab {
        DetailTab::Logs => super::logs::view(&state.logs).map(Event::Logs),
        DetailTab::Providers => super::groups::provider_view(&state.groups).map(Event::Groups),
        DetailTab::Models => super::groups::model_view(&state.groups).map(Event::Groups),
    };
    let details = container(column![tabs, detail].spacing(12))
        .padding(20)
        .width(Length::Fill)
        .style(panel_style);
    let tools = row![
        action("模型同步选择", Event::Models(models::Event::Open)),
        action("币种与显示", Event::Display(display::Event::Open)),
        action("模型定价", Event::Pricing(pricing::Event::Open))
    ]
    .spacing(8);
    let filter_card = container(
        row![state.range_picker.view().map(Event::Range), filters]
            .spacing(16)
            .align_y(iced::Alignment::Start),
    )
    .width(Length::Fill);
    let body = column![
        header,
        filter_card,
        overview_card,
        details,
        column![tools, text(status).size(12).style(text::secondary)].spacing(8)
    ]
    .spacing(16);
    scrollable(container(body).padding([7, 7]).max_width(1280).center_x(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .into()
}

fn panel_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style::default().background(palette.background).border(iced::Border {
        color: palette.text.scale_alpha(0.14),
        width: 1.0,
        radius: 14.0.into(),
    })
}
