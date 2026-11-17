use super::{event::Event, state::DetailTab, scan, display, pricing, overview};
use crate::models;
use iced::{Element, Length, widget::{button, column, container, row, scrollable, text, Space}};
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

pub fn view<'a>(state: &'a super::state::State, status: String, known_cost: Option<String>) -> Element<'a, Event> {
    let action = |label: &'static str, event| button(text(label).size(13)).padding([6, 10])
        .style(rounded_action_btn_style).on_press(event);
    if !state.open {
        return action("第三方智能体用量统计 ›", Event::Load(true)).into();
    }
    let header = row![text("用量统计").size(24), Space::new().width(Length::Fill),
        action("刷新", Event::Refresh),
        action("同步", Event::Sync),
        action("▾", Event::Scan(scan::Event::Open)),
        action("设置", Event::Scan(scan::Event::Open)),
        action("返回当前会话", Event::Close),
    ].spacing(8).align_y(iced::Alignment::Center);
    let mut options = vec![overview::ScopeOption { label: "全部供应商 / 模型".into(), provider: None, model: None }];
    if let Some(Ok((providers, models))) = &state.groups.result {
        for provider in providers {
            let label = match &provider.provider { vw_api_usage::UsageProvider::Unknown => "未知供应商".into(), vw_api_usage::UsageProvider::Known { provider_id } => provider_id.clone() };
            options.push(overview::ScopeOption { label, provider: Some(provider.provider.clone()), model: None });
        }
        for model in models {
            if let Some(name) = &model.model {
                options.push(overview::ScopeOption { label: name.clone(), provider: state.query.filter.provider.clone(), model: Some(name.clone()) });
            }
        }
    }
    // Own the option vector in the picker; do not return a borrow of local data.
    let filters = overview::view_with_options(&state.query, &options).map(Event::Filter);
    let summary = state.result.as_ref().and_then(|r| r.as_ref().ok());
    let hero = super::hero::view_with_cost(&state.hero, summary, state.loading, known_cost.as_deref()).map(Event::Hero);
    let notice = if state.loading { "正在加载用量…".into() } else {
        match &state.result {
            Some(Err(error)) => format!("用量加载失败：{error}"),
            Some(Ok(summary)) => format!("已知费用 {} · 未知费用 {} 请求", known_cost.unwrap_or_else(|| "—".into()), summary.counts.unknown_cost_requests),
            None => "请选择用量范围".into(),
        }
    };
    let overview_card = container(column![hero, text(notice).size(12), super::charts::view_range(&state.charts, state.hero.metric, Some(matches!(&state.query.range, vw_api_usage::UsageRange::All))).map(Event::Charts)].spacing(12))
        .padding(16).style(container::rounded_box);
    let mut tabs = row![].spacing(8);
    for (tab, label) in [(DetailTab::Logs, "请求日志"), (DetailTab::Providers, "供应商"), (DetailTab::Models, "模型")] {
        let selected = state.tab == tab;
        tabs = tabs.push(button(text(label).size(14)).padding([8, 12]).style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            if selected { style.background = Some(theme.extended_palette().background.strong.color.into()); }
            style
        }).on_press(Event::Tab(tab)));
    }
    let detail = match state.tab {
        DetailTab::Logs => super::logs::view(&state.logs).map(Event::Logs),
        DetailTab::Providers => super::groups::provider_view(&state.groups).map(Event::Groups),
        DetailTab::Models => super::groups::model_view(&state.groups).map(Event::Groups),
    };
    let details = container(column![tabs, detail].spacing(12)).padding(16).style(container::rounded_box);
    let tools = row![action("模型同步选择", Event::Models(models::Event::Open)), action("币种与显示", Event::Display(display::Event::Open)), action("模型定价", Event::Pricing(pricing::Event::Open))].spacing(8);
    let body = column![header, text(status).size(12), state.range_picker.view().map(Event::Range), filters, overview_card, details, tools].spacing(16);
    scrollable(container(body).padding([7, 7]).max_width(1280).center_x(Length::Fill))
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4))).into()
}
