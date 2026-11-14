//! Host-resolved display preferences; conversions never mutate ledger costs.
use super::{identity, Identity, Event as CenterEvent};
use crate::app::{App, Message, message::ViewMessage};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task, widget::{button, column, row, text}};
use vw_gateway_client::client::{Currency, RateMode, UsageDisplayPatch, UsageRatesState, UsageScanScope};

#[derive(Default)]
pub struct State {
    pub open: bool,
    sequence: u64,
    scope: Option<Identity>,
    busy: bool,
    result: Option<UsageRatesState>,
    currency: Currency,
    mode: RateMode,
    rate: String,
    error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open, Close, AutoLoad, Reload, Refresh, Currency(Currency), Mode(RateMode), Rate(String), Save,
    Loaded(u64, Identity, Result<UsageRatesState, String>),
}
impl State {
    pub fn close(&mut self) { self.open = false; self.sequence = self.sequence.wrapping_add(1); self.busy = false; }
}
fn message(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(CenterEvent::Display(event))) }
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = identity(app);
    let visible = app.screen == crate::app::Screen::Usage;
    let state = &mut app.usage_center.display;
    match event {
        Event::Close => state.close(),
        Event::Loaded(ticket, scope, result) => {
            if state.sequence != ticket || state.scope.as_ref() != Some(&scope) { return Task::none(); }
            state.busy = false;
            if current.as_ref() != Some(&scope) || !visible { state.result = None; return Task::none(); }
            match result {
                Ok(result) => {
                    state.currency = result.settings.currency;
                    state.mode = result.settings.mode;
                    state.rate = result.display.rate.to_string();
                    state.error = None;
                    state.result = Some(result);
                }
                Err(error) => { state.error = Some(error.clone()); return app.show_error_toast(format!("显示设置失败：{error}")); }
            }
        }
        _ if state.busy => {}
        Event::Currency(value) => { state.currency = value; state.rate = state.result.as_ref().and_then(|r| r.settings.manual_rates.get(&format!("{value:?}"))).map(ToString::to_string).unwrap_or_default(); }
        Event::Mode(value) => state.mode = value,
        Event::Rate(value) => state.rate = value,
        Event::AutoLoad if state.open => {}
        Event::AutoLoad | Event::Open | Event::Reload | Event::Refresh | Event::Save => {
            let saving = matches!(event, Event::Save);
            let refresh = matches!(event, Event::Refresh);
            if matches!(event, Event::Open) { state.open = true; }
            let Some(scope) = current else { state.result = None; state.error = Some("请选择项目与执行主机".into()); return Task::none(); };
            if saving && state.scope.as_ref() != Some(&scope) { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); }
            let mut patch = UsageDisplayPatch { currency: Some(state.currency), mode: Some(state.mode), ..Default::default() };
            if saving && state.mode == RateMode::Manual && state.currency != Currency::USD {
                let rate = state.rate.trim().parse::<f64>().ok().filter(|r| r.is_finite() && *r > 0.0);
                let Some(rate) = rate else { state.error = Some("汇率必须是有限正数".into()); return app.show_error_toast("汇率必须是有限正数"); };
                patch.manual_rates.insert(format!("{:?}", state.currency), rate);
            }
            state.result = None;
            state.scope = Some(scope.clone());
            state.sequence = state.sequence.wrapping_add(1);
            let ticket = state.sequence;
            state.busy = true;
            state.error = None;
            let client = crate::app::config::gateway_client();
            return Task::perform(async move {
                let result = async {
                    let client = client?;
                    let request = UsageScanScope { project_id: scope.project.clone().into(), target_host: scope.host.clone() };
                    if saving { client.usage_save_display(&request, &patch).await.map_err(|e| e.to_string())?; }
                    if refresh { client.usage_refresh_rates(&request).await } else { client.usage_display(&request).await }.map_err(|e| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Loaded(ticket, scope, result)));
        }
    }
    Task::none()
}

pub fn amount(app: &App, usd: &str) -> String {
    let state = &app.usage_center.display;
    if identity(app).as_ref() != state.scope.as_ref() { return format!("{usd} USD"); }
    let Some(result) = &state.result else { return format!("{usd} USD"); };
    let display = &result.display;
    if display.currency == Currency::USD { return format!("{usd} USD"); }
    match usd.parse::<f64>().ok().map(|v| v * display.rate).filter(|v| v.is_finite()) {
        Some(value) => format!("≈ {value:.4} {:?}（原始 {usd} USD）", display.currency),
        None => format!("{usd} USD（换算不可用）"),
    }
}
pub fn status(app: &App) -> String {
    let state = &app.usage_center.display;
    if identity(app).as_ref() != state.scope.as_ref() { return "汇率未加载；显示 USD".into(); }
    let Some(result) = &state.result else { return "汇率未加载；显示 USD".into(); };
    let d = &result.display;
    format!("{:?} · {:?} · 1 USD = {} {:?} · 来源 {} · 缓存{} · {}{} · 未知费用始终未知", result.settings.currency, result.settings.mode, d.rate, d.currency, d.source, if d.stale { "已过期" } else { "未过期" }, if d.degraded { "已降级" } else { "正常" }, d.error.as_ref().map(|e| format!(" · {e}")).unwrap_or_default())
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    button(text(label)).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(enabled.then(|| message(event))).into()
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.usage_center.display;
    if !state.open { return base; }
    let enabled = !state.busy;
    let mut currencies = row![].spacing(8);
    for currency in [Currency::USD, Currency::CNY, Currency::EUR, Currency::JPY, Currency::TWD, Currency::TRY] {
        currencies = currencies.push(button(text(format!("{currency:?}"))).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(enabled.then(|| message(Event::Currency(currency)))));
    }
    let mut body = column![text(format!("显示币种：{:?}；汇率模式：{:?}", state.currency, state.mode)), currencies,
        row![action("自动汇率", Event::Mode(RateMode::Auto), enabled), action("手工汇率", Event::Mode(RateMode::Manual), enabled)].spacing(8),
        text("手工汇率：1 USD 对应的目标币种金额"),
        settings_text_input("有限正数", &state.rate).on_input_maybe((enabled && state.mode == RateMode::Manual).then_some(|v| message(Event::Rate(v)))),
        text(status(app)), text("换算仅供显示，账本仍使用 USD。列表和图表中带 USD 的金额保留原币种。")].spacing(12);
    if let Some(error) = &state.error { body = body.push(settings_error_banner(error)); }
    let footer = row![action("保存", Event::Save, enabled), action("刷新自动汇率", Event::Refresh, enabled), action("重新加载", Event::Reload, enabled), action("关闭", Event::Close, true)].spacing(8);
    settings_modal_overlay(Some(base), message(Event::Close), settings_modal_layout(column![text("币种与显示设置").size(20)], body, Some(footer.into())).width(780).height(540))
}
