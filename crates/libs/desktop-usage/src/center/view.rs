use super::event::Event;
use super::{scan, display, pricing};
use crate::models;
use iced::{Element, widget::{button, column, row, text}};
pub fn view<'a>(state: &'a super::state::State, status: String, known_cost: Option<String>) -> Element<'a, Event> {
    use desktop_ui_controls::system_settings_common::rounded_action_btn_style;
    let action = |label: &'static str, event| button(text(label).size(13)).padding([6, 10])
        .style(rounded_action_btn_style).on_press(event);
    let mut content = column![row![
        action("刷新用量", Event::Refresh),
        action("扫描与修复", Event::Scan(scan::Event::Open)),
        action("模型同步选择", Event::Models(models::Event::Open)),
        action("币种与显示", Event::Display(display::Event::Open)),
        action("模型定价", Event::Pricing(pricing::Event::Open)),
        action("最近24小时（UTC）", Event::Load(false)),
        action("关闭用量中心", Event::Close),
    ].spacing(8)].spacing(8);
    if state.open {
        let overview = super::overview::view(&state.query).map(Event::Filter);
        let charts = super::charts::view(&state.charts).map(Event::Charts);
        let groups = super::groups::view(&state.groups).map(Event::Groups);
        let logs = super::logs::view(&state.logs).map(Event::Logs);
        content = content.push(overview);
        let label = if state.loading { "正在加载用量…".into() } else {
            match &state.result {
                Some(Ok(summary)) if summary.counts.total_requests == 0 => "暂无用量".into(),
                Some(Ok(summary)) => format!("请求 {} · 计费请求 {} · 成功 {} · 失败 {} · 已知费用 {} · 未知费用请求 {}",
                    summary.counts.total_requests, summary.counts.billable_requests, summary.succeeded_requests, summary.failed_requests, known_cost.as_deref().unwrap_or_default(), summary.counts.unknown_cost_requests),
                Some(Err(error)) => format!("用量加载失败：{error}"),
                None => "请选择用量范围".into(),
            }
        };
        content = content.push(text(status).size(13)).push(text(label).size(14)).push(charts).push(groups).push(logs);
    }
    content.into()
}
