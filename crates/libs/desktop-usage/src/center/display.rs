//! Host-resolved display preferences; conversions never mutate ledger costs.
use crate::Identity;
use std::{path::{Path, PathBuf}, str::FromStr};
use rust_decimal::Decimal;
use super::Outcome;

use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text}};
use gateway_usage_price::{Currency, RateMode, UsageDisplayPatch, UsageRatesState};
use gateway_usage_scan::UsageScanScope;

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
    preferences: Preferences,
    preferences_error: Option<String>,
    auto_tried: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Trend(bool), Today(TodayItem),
    Open, Close, AutoLoad, Reload, Refresh, Currency(Currency), Mode(RateMode), Rate(String), Save,
    Loaded(u64, Identity, Result<UsageRatesState, String>),
}
impl State {
    /// Display-only local preferences, shared by the trend and sidebar renderers.
    pub fn preferences(&self) -> &Preferences { &self.preferences }
    /// Resolved host conversion for chart axes; callers keep ledger values in USD.
    pub fn money_display(&self, current: &Identity) -> Option<&gateway_usage_price::MoneyDisplay> {
        (self.scope.as_ref() == Some(current)).then_some(())?;
        self.result.as_ref().map(|result| &result.display)
    }
    pub(super) fn restore(&mut self) {
        if let Some(result) = &self.result {
            self.currency = result.settings.currency;
            self.mode = result.settings.mode;
            self.rate = result.display.rate.to_string();
        }
    }
    pub fn close(&mut self) { self.restore(); self.open = false; self.sequence = self.sequence.wrapping_add(1); self.busy = false; }
}
fn message(event: Event) -> Outcome<Event> { Outcome::Event(event) }
pub fn update(state: &mut State, current: Option<Identity>, visible: bool, client: Result<vw_gateway_client::GatewayClient, String>, event: Event) -> Task<Outcome<Event>> {

    match event {
        Event::Feedback(..) => {},
        Event::Trend(_) | Event::Today(_) if state.busy => {},
        Event::Trend(area) => {
            let mut next = state.preferences.clone(); next.area = area;
            match next.save() { Ok(()) => { state.preferences = next; state.preferences_error = None; }, Err(error) => state.preferences_error = Some(error) }
        }
        Event::Today(item) => {
            let mut next = state.preferences.clone(); next.toggle(item);
            match next.save() { Ok(()) => { state.preferences = next; state.preferences_error = None; }, Err(error) => state.preferences_error = Some(error) }
        }
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
                    let refresh = !state.auto_tried && result.settings.mode == RateMode::Auto
                        && result.settings.currency != Currency::USD
                        && (result.settings.cache.is_none() || result.display.stale);
                    state.result = Some(result);
                    if refresh {
                        state.auto_tried = true;
                        return update(state, current, visible, client, Event::Refresh);
                    }
                }
                Err(error) => { state.restore(); state.error = Some(error.clone()); return Task::done(Outcome::Error(format!("显示设置失败：{error}").into())); }
            }
        }
        _ if state.busy => {}
        Event::Currency(value) => { state.currency = value; state.rate = selected_rate(state).map(|r| r.to_string()).unwrap_or_default(); }
        Event::Mode(value) => {
            state.mode = value;
            state.rate = selected_rate(state).map(|r| r.to_string()).unwrap_or_default();
        },
        Event::Rate(value) => state.rate = value,
        Event::AutoLoad if state.open && state.scope == current => {}
        Event::AutoLoad | Event::Open | Event::Reload | Event::Refresh | Event::Save => {
            let saving = matches!(event, Event::Save);
            let refresh = matches!(event, Event::Refresh);
            if matches!(event, Event::Open) {
                state.open = true; state.auto_tried = false;
            }
            if matches!(event, Event::Open | Event::AutoLoad | Event::Reload) {
                match Preferences::load() {
                    Ok(value) => { state.preferences = value; state.preferences_error = None; }
                    Err(error) => state.preferences_error = Some(error),
                }
            }
            let Some(scope) = current else { state.result = None; state.error = Some("网关不可用，请检查连接".into()); return Task::none(); };
            if saving && state.scope.as_ref() != Some(&scope) { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); }
            let mut patch = UsageDisplayPatch { currency: Some(state.currency), mode: Some(state.mode), ..Default::default() };
            if saving && state.mode == RateMode::Manual && state.currency != Currency::USD {
                let rate = state.rate.trim().parse::<f64>().ok().filter(|r| r.is_finite() && *r > 0.0);
                let Some(rate) = rate else { state.error = Some("汇率必须是有限正数".into()); return Task::done(Outcome::Error("汇率必须是有限正数".into())); };
                patch.manual_rates.insert(format!("{:?}", state.currency), rate);
            }
            if state.scope.as_ref() != Some(&scope) { state.result = None; state.auto_tried = false; }
            state.scope = Some(scope.clone());
            state.sequence = state.sequence.wrapping_add(1);
            let ticket = state.sequence;
            state.busy = true;
            state.error = None;

            return Task::perform(async move {
                let result = async {
                    let client = gateway_usage_price::GatewayUsagePriceClient::new(client?.transport().clone());
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

pub fn amount(state: &State, current: Option<Identity>, usd: &str) -> String {

    if current.as_ref() != state.scope.as_ref() { return format!("{usd} USD"); }
    let Some(result) = &state.result else { return format!("{usd} USD"); };
    let display = &result.display;
    if display.currency == Currency::USD { return format!("{usd} USD"); }
    if !display.rate.is_finite() || display.rate <= 0.0 {
        return format!("{usd} USD（汇率未知）");
    }
    let converted = Decimal::from_str(usd).ok().and_then(|amount| {
        Decimal::from_str(&display.rate.to_string()).ok().and_then(|rate| amount.checked_mul(rate))
    });
    match converted {
        Some(value) => format!("≈ {:.4} {:?}（原始 {usd} USD）", value, display.currency),
        None => format!("{usd} USD（换算不可用）"),
    }
}
pub fn sidebar_today(
    state: &State,
    current: Option<Identity>,
    today: Option<&(Identity, gateway_usage_read::UsageSummary)>,
) -> Option<(String, String)> {
    use super::hero::{Metric, compact_count};

    let items = &state.preferences().today;
    let (scope, summary) = today?;
    if items.is_empty() || current.as_ref() != Some(scope) {
        return None;
    }

    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let (label, short, detail) = match item {
            TodayItem::Cost => {
                let total = summary.counts.total_requests;
                let unknown = summary.counts.unknown_cost_requests;
                let full = amount(state, current.clone(), &summary.known_cost_usd);
                let short = if total > 0 && unknown == total {
                    "—".into()
                } else if let Some(usd) = full.split('（').next().and_then(|v| v.strip_suffix(" USD")) {
                    usd.parse::<f64>().ok().filter(|v| v.is_finite())
                        .map_or_else(|| "—".into(), |v| {
                            if v >= 1000.0 { format!("${}", compact_count(v)) }
                            else { format!("${v:.2}") }
                        })
                } else {
                    full.split('（').next().unwrap_or(&full).to_owned()
                };
                let detail = if unknown == 0 { full } else { format!("{full} · {unknown} 条请求费用未知") };
                ("已知费用", short, detail)
            }
            TodayItem::Tokens | TodayItem::Requests | TodayItem::HitRate => {
                let metric = match item {
                    TodayItem::Tokens => Metric::Tokens,
                    TodayItem::Requests => Metric::Requests,
                    _ => Metric::HitRate,
                };
                let value = metric.value(summary);
                let detail = metric.format(value);
                let short = match metric {
                    Metric::HitRate => detail.clone(),
                    _ => value.map_or_else(|| "—".into(), compact_count),
                };
                (metric.label(), short, detail)
            }
        };
        values.push((short, format!("{label} {detail}")));
    }
    let badge = format!("今日 {}", values.first()?.0);
    let tooltip = values.into_iter().map(|(_, detail)| detail).collect::<Vec<_>>().join(" · ");
    Some((badge, format!("今日 · {tooltip}")))
}

pub fn status(state: &State, current: Option<Identity>) -> String {

    if current.as_ref() != state.scope.as_ref() { return "汇率未加载；显示 USD".into(); }
    let Some(result) = &state.result else { return "汇率未加载；显示 USD".into(); };
    let d = &result.display;
    if !d.rate.is_finite() || d.rate <= 0.0 { return "汇率未知；显示 USD".into(); }
    format!("{:?} · {:?} · 1 USD = {} {:?} · 来源 {} · 缓存{} · {}{} · 未知费用始终未知", result.settings.currency, result.settings.mode, d.rate, d.currency, d.source, if d.stale { "已过期" } else { "未过期" }, if d.degraded { "已降级" } else { "正常" }, d.error.as_ref().map(|e| format!(" · {e}")).unwrap_or_default())
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Outcome<Event>> {
    button(text(label).size(13))
        .padding([7, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}

fn save_button(label: &'static str, enabled: bool) -> Element<'static, Outcome<Event>> {
    button(text(label).size(14))
        .padding([9, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(enabled.then(|| message(Event::Save)))
        .into()
}

fn choice_button(
    label: &str,
    event: Option<Event>,
    selected: bool,
) -> iced::widget::Button<'static, Outcome<Event>> {
    let label = if selected {
        format!("✓ {label}")
    } else {
        label.to_owned()
    };
    button(text(label).size(13))
        .padding([7, 12])
        .style(move |theme, status| {
            if status == button::Status::Disabled && !selected {
                rounded_action_btn_style(theme, status)
            } else {
                settings_segment_button_style(theme, status, selected)
            }
        })
        .on_press_maybe(event.map(message))
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct CurrencyOption(Currency);

impl std::fmt::Display for CurrencyOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            Currency::USD => "USD · 美元",
            Currency::CNY => "CNY · 人民币",
            Currency::EUR => "EUR · 欧元",
            Currency::JPY => "JPY · 日元",
            Currency::TWD => "TWD · 新台币",
            Currency::TRY => "TRY · 土耳其里拉",
        })
    }
}

fn currency_picker(state: &State, enabled: bool) -> Element<'static, Outcome<Event>> {
    settings_select(
        [Currency::USD, Currency::CNY, Currency::EUR, Currency::JPY, Currency::TWD, Currency::TRY]
            .map(CurrencyOption),
        Some(CurrencyOption(state.currency)),
        |choice| message(Event::Currency(choice.0)),
    )
    .disabled(!enabled)
    .width(Length::Fill)
    .into()
}

pub fn quick_settings<'a>(
    state: &'a State,
    current: Option<&Identity>,
) -> Element<'a, Outcome<Event>> {
    let enabled = !state.busy && current.is_some() && state.scope.as_ref() == current;
    let saved = state.result.as_ref().filter(|_| state.scope.as_ref() == current);
    let currency_changed = saved.is_some_and(|result| result.settings.currency != state.currency);
    let mut controls = column![
        quick_row("币种", currency_picker(state, enabled && saved.is_some())),
        quick_row(
            "趋势图样式",
            settings_select(
                ["柱形图", "折线图"],
                Some(if state.preferences.area { "折线图" } else { "柱形图" }),
                |label| message(Event::Trend(label == "折线图")),
            )
            .disabled(state.busy)
            .width(Length::Fill)
            .into(),
        ),
        quick_row("侧栏「今日」", today_controls(state, !state.busy)),
    ]
    .spacing(8);
    if currency_changed {
        if state.mode == RateMode::Manual && state.currency != Currency::USD {
            controls = controls.push(quick_row(
                "手工汇率",
                settings_text_input("1 USD 对应金额", &state.rate)
                    .on_input_maybe(enabled.then_some(|v| message(Event::Rate(v))))
                    .into(),
            ));
        }
        controls = controls.push(save_button("保存币种", enabled));
    }
    if let Some(error) = &state.preferences_error {
        controls = controls.push(settings_error_banner(error));
    }
    if let Some(error) = &state.error {
        controls = controls.push(settings_error_banner(error));
    }
    controls.into()
}

fn quick_row<'a>(
    label: &'static str,
    control: Element<'a, Outcome<Event>>,
) -> Element<'a, Outcome<Event>> {
    row![
        text(label).size(13).width(98).style(text::secondary),
        iced::widget::container(control).width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    current: Option<Identity>,
    base: Element<'a, M>,
    map: fn(Outcome<Event>) -> M,
) -> Element<'a, M> {
    if !state.open {
        return base;
    }
    let enabled = !state.busy;
    let auto = state.mode == RateMode::Auto;
    let area = state.preferences.area;
    let editable_rate = enabled && !auto && state.currency != Currency::USD;
    let rate_modes = row![
        choice_button("自动", (enabled && !auto).then_some(Event::Mode(RateMode::Auto)), auto),
        choice_button("手工", (enabled && auto).then_some(Event::Mode(RateMode::Manual)), !auto),
    ]
    .spacing(6);
    let chart_styles = row![
        choice_button("柱形图", (enabled && area).then_some(Event::Trend(false)), !area),
        choice_button("曲线面积图", (enabled && !area).then_some(Event::Trend(true)), area),
    ]
    .spacing(6)
    .wrap();

    let currency = column![
        settings_group_heading("币种与汇率", "换算仅供显示，账本及标注 USD 的金额保留原币种。"),
        quick_row("显示币种", currency_picker(state, enabled)),
        quick_row("汇率模式", rate_modes.into()),
        quick_row(
            "1 USD 对应",
            settings_text_input("目标币种金额", &state.rate)
                .on_input_maybe(editable_rate.then_some(|value| message(Event::Rate(value))))
                .into(),
        ),
        text(status(state, current)).size(12).style(settings_muted_text_style),
        row![
            action("刷新自动汇率", Event::Refresh, enabled),
            action("重新加载", Event::Reload, enabled),
        ]
        .spacing(8)
        .wrap(),
    ]
    .spacing(10);
    let display = column![
        settings_group_heading("用量显示", "图表和侧栏偏好即时生效。"),
        quick_row("趋势图样式", chart_styles.into()),
        quick_row(
            "侧栏「今日」",
            settings_control_with_hint(
                today_controls(state, enabled),
                "最多选两项，全部取消后隐藏。",
            ),
        ),
    ]
    .spacing(10);
    let mut body = column![currency, settings_divider(), display].spacing(16);
    if let Some(error) = &state.preferences_error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error));
    }

    let close = message(Event::Close);
    let header = settings_modal_header(
        text("币种与显示设置").size(22),
        settings_close_button(close.clone()),
    );
    let footer = column![
        settings_divider(),
        container(save_button("保存币种与汇率", enabled && state.result.is_some()))
            .align_right(Length::Fill),
    ]
    .spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(580)
        .height(560)
        .style(|theme| {
            let mut style = settings_modal_card_style(theme).background(theme.palette().background);
            style.border.radius = 16.0.into();
            style
        });
    settings_modal_overlay(Some(base), map(close), Element::from(card).map(map))
}

fn selected_rate(state: &State) -> Option<f64> {
    if state.currency == Currency::USD { return Some(1.0); }
    let settings = &state.result.as_ref()?.settings;
    let key = format!("{:?}", state.currency);
    let rates = if state.mode == RateMode::Auto { &settings.cache.as_ref()?.rates } else { &settings.manual_rates };
    rates.get(&key).copied().filter(|rate| rate.is_finite() && *rate > 0.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodayItem { Cost, Tokens, Requests, HitRate }
const TODAY: [(TodayItem, &str, &str); 4] = [
    (TodayItem::Cost, "cost", "金额"), (TodayItem::Tokens, "tokens", "Token"),
    (TodayItem::Requests, "requests", "请求数"), (TodayItem::HitRate, "hitRate", "命中率"),
];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub area: bool,
    pub today: Vec<TodayItem>,
}
impl Default for Preferences {
    fn default() -> Self { Self { area: false, today: vec![TodayItem::Cost] } }
}
impl Preferences {
    fn toggle(&mut self, item: TodayItem) {
        if self.today.contains(&item) { self.today.retain(|value| *value != item); }
        else if self.today.len() < 2 { self.today.push(item); }
        self.today.sort_by_key(|item| TODAY.iter().position(|(value, _, _)| value == item));
    }
    fn path() -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).ok_or("无法获取用户目录")?;
        Ok(config_runtime_paths::home_config_dir(PathBuf::from(home)).join("usage-display.json"))
    }
    pub fn load() -> Result<Self, String> { Self::read(&Self::path()?) }
    fn read(path: &Path) -> Result<Self, String> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let today = value["sidebarTodayItems"].as_array().map(|items| {
            TODAY.iter().filter(|(_, key, _)| items.iter().any(|item| item.as_str() == Some(*key)))
                .take(2).map(|(item, _, _)| *item).collect()
        }).unwrap_or_else(|| vec![TodayItem::Cost]);
        Ok(Self { area: value["trendChartType"].as_str() == Some("area"), today })
    }
    fn save(&self) -> Result<(), String> { self.write(&Self::path()?) }
    fn write(&self, path: &Path) -> Result<(), String> {
        let today: Vec<_> = TODAY.iter().filter(|(item, _, _)| self.today.contains(item)).map(|(_, key, _)| *key).collect();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "trendChartType": if self.area { "area" } else { "bar" }, "sidebarTodayItems": today
        })).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.parent().ok_or("显示偏好路径无效")?).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    }
}
fn today_controls(state: &State, enabled: bool) -> Element<'static, Outcome<Event>> {
    let mut controls = row![].spacing(8);
    for (item, _, label) in TODAY {
        let selected = state.preferences.today.contains(&item);
        let available = enabled && (selected || state.preferences.today.len() < 2);
        controls = controls.push(choice_button(
            label,
            available.then_some(Event::Today(item)),
            selected,
        ));
    }
    controls.wrap().into()
}

#[cfg(test)]
#[path = "../usage_replica_07_tests.rs"]
mod tests;
