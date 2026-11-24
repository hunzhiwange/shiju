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

pub fn view<'a>(i18n: desktop_i18n::I18n, state: State<'a>, visible: bool) -> Element<'a, Event> {
    let health = state.health.as_ref().filter(|_| visible);
    let route = state.route.as_ref().filter(|_| visible);
    let label = match health.map(|h| h.state) {
        Some(FailoverHealthState::Closed) => i18n.__("闭合"),
        Some(FailoverHealthState::Open) => i18n.__("打开（熔断）"),
        Some(FailoverHealthState::HalfOpen) => i18n.__("半开（恢复观察）"),
        _ => i18n.__("未知，请刷新"),
    };
    let configured = route
        .and_then(|r| r.configured_provider_id.as_ref())
        .map(|id| id.as_ref())
        .unwrap_or(i18n.__("未知"));
    let actual =
        route.and_then(|r| r.actual_provider_id.as_ref()).map(|id| id.as_ref()).unwrap_or(i18n.__("未知"));
    let mut body = column![
        text(i18n.sprintf("生产健康：%s", &[label.to_string().into()])),
        text(i18n.sprintf("配置候选：%s", &[configured.to_string().into()])),
        text(i18n.sprintf("实际命中：%s", &[actual.to_string().into()])),
    ]
    .spacing(7);
    if let Some(ms) = health.and_then(|h| h.retry_after_ms) {
        body = body.push(text(i18n.sprintf("本次观测恢复等待：%s 毫秒", &[format!("{}", ms).into()])));
    }
    if let Some(reason) = route.and_then(|r| r.failure_reason) {
        body = body.push(text(i18n.sprintf("失败原因：%s", &[format!("{:?}", reason).into()])));
    }
    if let Some(error) = state.error {
        body = body.push(text(error));
    }
    let mut refresh = button(text(i18n.__("刷新健康"))).padding([7, 12]).style(rounded_action_btn_style);
    let mut reset = button(text(i18n.__("恢复此供应商"))).padding([7, 12]).style(primary_action_btn_style);
    if visible && !state.busy {
        refresh = refresh.on_press(Event::Refresh);
        reset = reset.on_press(Event::Reset);
    }
    body.push(row![refresh, reset].spacing(8)).into()
}
