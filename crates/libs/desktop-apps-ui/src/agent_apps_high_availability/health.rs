use desktop_ui_controls::system_settings_common::*;
use vw_api_app_failover::{FailoverHealthResponse, FailoverRouteResponse};
#[derive(Clone, Debug)]
pub enum Event {
    Refresh,
    Reset,
}
pub struct State<'a> {
    pub health: &'a Option<FailoverHealthResponse>,
    pub route: &'a Option<FailoverRouteResponse>,
    pub error: &'a Option<String>,
    pub busy: bool,
}
use iced::{
    Element,
    widget::{button, column, row, text},
};
use vw_api_app_failover::FailoverHealthState;

pub fn view<'a>(state: State<'a>, visible: bool) -> Element<'a, Event> {
    let health = state.health.as_ref().filter(|_| visible);
    let route = state.route.as_ref().filter(|_| visible);
    let label = match health.map(|h| h.state) {
        Some(FailoverHealthState::Closed) => "闭合",
        Some(FailoverHealthState::Open) => "打开（熔断）",
        Some(FailoverHealthState::HalfOpen) => "半开（恢复观察）",
        _ => "未知，请刷新",
    };
    let configured = route
        .and_then(|r| r.configured_provider_id.as_ref())
        .map(|id| id.as_ref())
        .unwrap_or("未知");
    let actual =
        route.and_then(|r| r.actual_provider_id.as_ref()).map(|id| id.as_ref()).unwrap_or("未知");
    let mut body = column![
        text(format!("生产健康：{label}")),
        text(format!("配置候选：{configured}")),
        text(format!("实际命中：{actual}")),
    ]
    .spacing(7);
    if let Some(ms) = health.and_then(|h| h.retry_after_ms) {
        body = body.push(text(format!("本次观测恢复等待：{ms} 毫秒")));
    }
    if let Some(reason) = route.and_then(|r| r.failure_reason) {
        body = body.push(text(format!("失败原因：{reason:?}")));
    }
    if let Some(error) = state.error {
        body = body.push(text(error));
    }
    let mut refresh = button(text("刷新健康")).padding([7, 12]).style(rounded_action_btn_style);
    let mut reset = button(text("恢复此供应商")).padding([7, 12]).style(primary_action_btn_style);
    if visible && !state.busy {
        refresh = refresh.on_press(Event::Refresh);
        reset = reset.on_press(Event::Reset);
    }
    body.push(row![refresh, reset].spacing(8)).into()
}
