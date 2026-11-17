//! Host-owned prices: mutations are acknowledged before refreshing the ledger.
use crate::Identity;
use super::Outcome;

use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, row, text, text_editor}};
use gateway_usage_price::{ModelPricingInfo, StoredModelPricing, BillingDefaults, BillingModelSource, UsagePricingStorage};
use gateway_usage_scan::UsageScanScope;

#[derive(Default)]
pub struct State {
    pub open: bool,
    sequence: u64,
    scope: Option<Identity>,
    busy: bool,
    prices: Vec<StoredModelPricing>,
    error: Option<String>,
    editor: Option<ModelPricingInfo>,
    creating: bool,
    pending_delete: Option<String>,
    source: String,
    import: text_editor::Content,
    models: crate::models::State,
    files_open: bool,
    page: usize,
    billing: Option<BillingDefaults>,
    billing_draft: Option<BillingDefaults>,
    billing_busy: bool,
    storage: Option<UsagePricingStorage>,
    storage_busy: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Models(crate::models::Event), Files, CancelDelete, Page(usize),
    BillingEdit, BillingCancel, BillingField(String, BillingModelSource), BillingSave,
    BillingLoaded(u64, Identity, Result<BillingDefaults, String>),
    StorageOpen, StorageReload,
    StorageLoaded(u64, Identity, Result<UsagePricingStorage, String>),
    Open, Close, Reload, RefreshPrices, New, Edit(usize), CancelEdit,
    Field(usize, String), Source(String), ImportEdit(text_editor::Action),
    Save, Delete(String), ConfirmDelete(String), Import,
    Loaded(u64, Identity, Result<Vec<StoredModelPricing>, String>),
    Written(u64, Identity, Result<(), String>),
}
impl State {
    pub fn close(&mut self) {
        let sequence = self.sequence.wrapping_add(1);
        *self = Self::default();
        self.sequence = sequence;
    }
}

fn message(event: Event) -> Outcome<Event> { Outcome::Event(event) }
fn validate(price: &ModelPricingInfo) -> Result<(), String> {
    if price.model_id.as_ref().trim().is_empty() { return Err("模型 ID 不能为空".into()); }
    for value in [&price.input_cost_per_million, &price.output_cost_per_million,
        &price.cache_read_cost_per_million, &price.cache_creation_cost_per_million] {
        let parts: Vec<_> = value.split('.').collect();
        if parts.len() > 2 || parts.iter().any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit())) {
            return Err("价格必须为非负十进制金额".into());
        }
    }
    Ok(())
}
#[derive(Clone)]
enum Write { Save(ModelPricingInfo, bool), Delete(String), Import(String, Vec<ModelPricingInfo>) }

pub fn update(state: &mut State, current: Option<Identity>, visible: bool, client: Result<vw_gateway_client::GatewayClient, String>, event: Event) -> Task<Outcome<Event>> {

    match event {
        Event::Models(event) => {
            let refresh = matches!(&event, crate::models::Event::Polled(_, id, Ok(_)) if current.as_ref() == Some(id) && state.open && state.scope.as_ref() == Some(id));
            let task = crate::models::update_with_client(&mut state.models, current.clone(), client.clone(), event)
                .map(|event| message(Event::Models(event)));
            if refresh { return Task::batch([task, update(state, current, visible, client, Event::RefreshPrices)]); }
            return task;
        }
        Event::BillingLoaded(ticket, id, result) => {
            if !state.open || state.sequence != ticket || current.as_ref() != Some(&id) || state.scope.as_ref() != Some(&id) { return Task::none(); }
            state.billing_busy = false;
            match result { Ok(value) => { state.billing = Some(value); state.billing_draft = None; }, Err(error) => state.error = Some(error) }
        }
        Event::StorageLoaded(ticket, id, result) => {
            if !state.open || state.sequence != ticket || current.as_ref() != Some(&id) || state.scope.as_ref() != Some(&id) { return Task::none(); }
            state.storage_busy = false;
            match result { Ok(value) => state.storage = Some(value), Err(error) => state.error = Some(error) }
        }
        Event::BillingEdit if !state.billing_busy && !state.busy => state.billing_draft = state.billing.clone(),
        Event::BillingCancel if !state.billing_busy => state.billing_draft = None,
        Event::BillingField(app, source) if !state.billing_busy && !state.busy => {
            if let Some(draft) = &mut state.billing_draft { if let Some(value) = draft.get_mut(&app) { *value = source; } }
        }
        Event::BillingSave | Event::Files | Event::StorageOpen | Event::StorageReload => {
            let billing = matches!(event, Event::BillingSave);
            if state.busy || billing && state.billing_busy || !billing && state.storage_busy { return Task::none(); }
            if matches!(event, Event::Files) { state.files_open = !state.files_open; if !state.files_open { return Task::none(); } }
            let Some(id) = current.filter(|id| state.open && state.scope.as_ref() == Some(id)) else { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); };
            let ticket = state.sequence;
            let draft = state.billing_draft.clone();
            if billing && draft.is_none() { return Task::none(); }
            if billing { state.billing_busy = true; } else { state.storage_busy = true; }
            let open = matches!(event, Event::StorageOpen);
            return Task::perform(async move {
                let client = client.map(|client| gateway_usage_price::GatewayUsagePriceClient::new(client.transport().clone()));
                let scope = UsageScanScope { project_id: id.project.clone().into(), target_host: id.host.clone() };
                if billing {
                    let result = async { client?.usage_save_billing_defaults(&scope, &draft.ok_or("请先编辑计费配置")?).await.map_err(|error| error.to_string()) }.await;
                    Event::BillingLoaded(ticket, id, result)
                } else {
                    let result = async { client?.usage_pricing_storage(&scope, open).await.map_err(|error| error.to_string()) }.await;
                    Event::StorageLoaded(ticket, id, result)
                }
            }, message);
        }
        Event::BillingEdit | Event::BillingCancel | Event::BillingField(..) => {},
        Event::CancelDelete => state.pending_delete = None,
        Event::Page(page) => state.page = page.min(state.prices.len().saturating_sub(1) / 20),
        Event::Feedback(..) => {},
        Event::Close if !state.busy => { state.close(); }
        Event::Open | Event::Reload | Event::RefreshPrices => {
            if matches!(event, Event::Reload) && (state.billing_busy || state.storage_busy) { return Task::none(); }
            if state.busy { return Task::none(); }
            let load_models = matches!(event, Event::Open);
            let load_billing = !matches!(event, Event::RefreshPrices);
            state.open = true;
            let Some(scope) = current else { state.error = Some("请选择项目与执行主机".into()); return Task::none(); };
            if state.scope.as_ref() != Some(&scope) {
                state.editor = None;
                state.pending_delete = None;
                state.page = 0;
                state.models.close();
                state.billing = None; state.billing_draft = None; state.storage = None; state.billing_busy = false; state.storage_busy = false;
                state.prices.clear();
                state.import = text_editor::Content::new();
            }
            state.scope = Some(scope.clone());
            if load_billing { state.sequence = state.sequence.wrapping_add(1); }
            let ticket = state.sequence;
            state.busy = true;
            state.error = None;

            let models = if load_models { crate::models::update_with_client(&mut state.models, Some(scope.clone()), client.clone(), crate::models::Event::Open)
                .map(|event| message(Event::Models(event))) } else { Task::none() };
            if load_billing { state.billing_busy = true; }
            let billing_client = client.clone();
            let billing_scope = scope.clone();
            let billing = if load_billing { Task::perform(async move {
                let id = billing_scope;
                let scope = UsageScanScope { project_id: id.project.clone().into(), target_host: id.host.clone() };
                let result = async { gateway_usage_price::GatewayUsagePriceClient::new(billing_client?.transport().clone()).usage_billing_defaults(&scope).await.map_err(|error| error.to_string()) }.await;
                Event::BillingLoaded(ticket, id, result)
            }, message) } else { Task::none() };
            let prices = Task::perform(async move {
                let result = async {
                    gateway_usage_price::GatewayUsagePriceClient::new(client?.transport().clone()).usage_prices(&UsageScanScope { project_id: scope.project.clone().into(), target_host: scope.host.clone() }).await.map_err(|e| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Loaded(ticket, scope, result)));
            return Task::batch([models, prices, billing]);
        }
        Event::Loaded(ticket, scope, result) => {
            if state.open && state.sequence == ticket && state.scope.as_ref() == Some(&scope) {
                state.busy = false;
                if !visible || current.as_ref() != Some(&scope) { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); }
                match result { Ok(prices) => { state.prices = prices; state.page = state.page.min(state.prices.len().saturating_sub(1) / 20); }, Err(e) => state.error = Some(e) }
            }
        }
        Event::Written(ticket, scope, result) => {
            if state.open && state.sequence == ticket && state.scope.as_ref() == Some(&scope) {
                state.busy = false;
                if !visible || current.as_ref() != Some(&scope) { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); }
                match result {
                    Err(e) => state.error = Some(e),
                    Ok(()) => {
                        state.editor = None;
                        state.pending_delete = None;
                        state.import = text_editor::Content::new();
                        return Task::batch([update(state, current, visible, client, Event::Reload), Task::done(Outcome::Refresh), Task::done(Outcome::Success("价格已更新".into()))]);
                    }
                }
            }
        }
        _ if state.busy => {}
        Event::New => {
            state.creating = true;
            state.editor = Some(ModelPricingInfo { model_id: "".into(), display_name: String::new(), input_cost_per_million: "0".into(), output_cost_per_million: "0".into(), cache_read_cost_per_million: "0".into(), cache_creation_cost_per_million: "0".into() });
        }
        Event::Edit(index) => {
            if let Some(price) = state.prices.get(index) { state.editor = Some(price.price.clone()); state.creating = false; }
        }
        Event::CancelEdit => state.editor = None,
        Event::Field(field, value) => if let Some(price) = &mut state.editor {
            match field {
                0 if state.creating => price.model_id = value.into(),
                1 => price.display_name = value,
                2 => price.input_cost_per_million = value,
                3 => price.output_cost_per_million = value,
                4 => price.cache_read_cost_per_million = value,
                5 => price.cache_creation_cost_per_million = value,
                _ => {}
            }
        },
        Event::Source(source) => state.source = source,
        Event::ImportEdit(action) => state.import.perform(action),
        Event::Delete(id) => { state.pending_delete = Some(id); return Task::none(); }
        Event::Save | Event::ConfirmDelete(_) | Event::Import => {
            if state.billing_busy || state.storage_busy { return Task::none(); }
            let operation = match event {
                Event::Save => match state.editor.clone() {
                    Some(price) => validate(&price).map(|()| Write::Save(price, state.creating)),
                    None => Err("请先选择价格".into()),
                },
                Event::ConfirmDelete(id) => {
                    if state.pending_delete.as_ref() != Some(&id) { return Task::none(); }
                    Ok(Write::Delete(id))
                },
                _ => (|| {
                    let prices: Vec<ModelPricingInfo> = serde_json::from_str(&state.import.text()).map_err(|e| format!("导入 JSON 无效：{e}"))?;
                    if state.source.trim().is_empty() || state.source.len() > 256 || prices.is_empty() || prices.len() > 256 { return Err("请填写来源并提供 1–256 条价格".into()); }
                    for price in &prices { validate(price)?; }
                    Ok(Write::Import(state.source.clone(), prices))
                })(),
            };
            let operation = match operation { Ok(op) => op, Err(e) => { state.error = Some(e); return Task::none(); } };
            let Some(scope) = current.filter(|s| state.scope.as_ref() == Some(s)) else {
                state.error = Some("项目或主机缺失或已切换，请重新加载".into()); return Task::none();
            };
            state.busy = true;
            state.error = None;
            state.sequence = state.sequence.wrapping_add(1);
            let ticket = state.sequence;

            return Task::perform(async move {
                let result = async {
                    let client = gateway_usage_price::GatewayUsagePriceClient::new(client?.transport().clone());
                    let scope = UsageScanScope { project_id: scope.project.clone().into(), target_host: scope.host.clone() };
                    match operation {
                        Write::Save(price, true) => client.usage_create_price(&scope, &price).await.map(|_| ()).map_err(|e| e.to_string()),
                        Write::Save(price, false) => client.usage_update_price(&scope, &price).await.map(|_| ()).map_err(|e| e.to_string()),
                        Write::Delete(id) => {
                            let result = client.usage_delete_price(&scope, &id).await.map_err(|e| e.to_string())?;
                            if result.deleted { Ok(()) } else { Err("价格不存在，未删除".into()) }
                        }
                        Write::Import(source, prices) => client.usage_import_prices(&scope, &source, &prices).await.map(|_| ()).map_err(|e| e.to_string()),
                    }
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Written(ticket, scope, result)));
        }
        Event::Close => {}
    }
    Task::none()
}

fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Outcome<Event>> {
    button(text(label)).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event))).into()
}
pub fn overlay<'a, M: Clone + 'static>(state: &'a State, _current: Option<Identity>, base: Element<'a, M>, map: fn(Outcome<Event>) -> M) -> Element<'a, M> {

    if !state.open { return base; }
    let enabled = !state.busy && !state.billing_busy && !state.storage_busy;
    let mut body = column![crate::models::card(&state.models).map(|event| message(Event::Models(event))),
        billing_card(state),
        row![text(format!("模型定价（{}）", state.prices.len())).size(18), text("USD / 百万 Tokens"), action("新增定价", Event::New, enabled), action("文件…", Event::Files, enabled)].spacing(12),
        row![text("模型").width(Length::FillPortion(3)), text("输入").width(Length::Fill), text("输出").width(Length::Fill), text("缓存读取").width(Length::Fill), text("缓存写入").width(Length::Fill), text("操作").width(130)].spacing(8)
    ].spacing(12);
    if state.files_open {
        let mut files = column![text("执行主机定价存储（SQLite）"), text("在所选主机打开目录；重新加载会读取该主机数据库中的定价。")].spacing(6);
        if let Some(storage) = &state.storage {
            files = files.push(text(&storage.path));
            if let Some(reason) = &storage.reason { files = files.push(text(reason)); }
            files = files.push(action("在执行主机打开目录", Event::StorageOpen, !state.storage_busy && storage.can_open));
        }
        if state.storage_busy { files = files.push(text("正在请求执行主机…")); }
        files = files.push(row![action("刷新文件信息", Event::StorageReload, !state.storage_busy), action("重新加载定价", Event::Reload, enabled)].spacing(8));
        body = body.push(settings_modal_card(files));
    }
    if state.prices.is_empty() && !state.busy { body = body.push(text("暂无模型定价")); }
    for (index, price) in state.prices.iter().enumerate().skip(state.page * 20).take(20) {
        body = body.push(row![
            column![text(&price.price.display_name), text(price.price.model_id.as_ref()).size(12), text(if price.is_custom { "自定义" } else { "自动" }).size(12)].width(Length::FillPortion(3)),
            text(format!("${}", price.price.input_cost_per_million)).width(Length::Fill),
            text(format!("${}", price.price.output_cost_per_million)).width(Length::Fill),
            text(format!("${}", price.price.cache_read_cost_per_million)).width(Length::Fill),
            text(format!("${}", price.price.cache_creation_cost_per_million)).width(Length::Fill),
            row![action("编辑", Event::Edit(index), enabled), action("删除", Event::Delete(price.price.model_id.0.clone()), enabled)].spacing(6).width(130)
        ].spacing(8));
        if state.pending_delete.as_deref() == Some(price.price.model_id.as_ref()) {
            body = body.push(row![text("确认删除此模型定价？"), action("确认删除", Event::ConfirmDelete(price.price.model_id.0.clone()), enabled), action("取消", Event::CancelDelete, enabled)].spacing(8));
        }
    }
    body = body.push(row![action("上一页", Event::Page(state.page.saturating_sub(1)), enabled && state.page > 0), text(format!("{} / {}", state.page + 1, state.prices.len().saturating_sub(1) / 20 + 1)), action("下一页", Event::Page(state.page + 1), enabled && (state.page + 1) * 20 < state.prices.len())].spacing(10));
    if let Some(price) = &state.editor {
        for (index, (label, value)) in [("模型 ID", price.model_id.as_ref()), ("显示名称", price.display_name.as_str()), ("输入", price.input_cost_per_million.as_str()), ("输出", price.output_cost_per_million.as_str()), ("缓存读取", price.cache_read_cost_per_million.as_str()), ("缓存创建", price.cache_creation_cost_per_million.as_str())].into_iter().enumerate() {
            body = body.push(column![text(label), settings_text_input(label, value).on_input_maybe((enabled && (index != 0 || state.creating)).then_some(move |value| message(Event::Field(index, value))))].spacing(4));
        }
        body = body.push(row![action("保存价格", Event::Save, enabled), action("取消编辑", Event::CancelEdit, enabled)].spacing(8));
    }
    body = body.push(text("批量导入：ModelPricingInfo JSON 数组（model_id、display_name 和四项 *_cost_per_million 字符串）"))
        .push(settings_text_input("明确导入来源", &state.source).on_input_maybe(enabled.then_some(|v| message(Event::Source(v)))))
        .push(settings_text_editor(&state.import).on_action(move |a| message(Event::ImportEdit(a))).height(160))
        .push(action("批量导入", Event::Import, enabled));
    let mut header = column![text("模型定价").size(20)].spacing(8);
    if state.busy { header = header.push(text("正在处理…")); }
    if let Some(error) = &state.error { header = header.push(settings_error_banner(error)); }
    let footer = row![action("重新加载", Event::Reload, enabled), action("返回用量统计", Event::Close, enabled)].spacing(8);
    let panel = settings_modal_overlay(Some(base), map(message(Event::Close)), Element::from(settings_modal_layout(header, body, Some(footer.into())).width(1080).height(720)).map(map));
    if crate::models::picker_open(&state.models) {
        // Map after rendering so callers keep their existing Outcome boundary.
        let picker = crate::models::overlay_with_base(&state.models, iced::widget::Space::new().into(), |event| message(Event::Models(if matches!(event, crate::models::Event::Close) { crate::models::Event::Picker(false) } else { event })));
        return iced::widget::stack![panel, picker.map(map)].into();
    }
    panel
}

#[cfg(test)]
#[path = "../usage_replica_08_tests.rs"]
mod tests;

fn billing_card(state: &State) -> Element<'_, Outcome<Event>> {
    let mut body = column![text("计费默认配置").size(16), text("用于后续路由请求；原生会话费用保持来源报告值，历史记录不重新计费。")].spacing(8);
    if let Some(draft) = &state.billing_draft {
        for (app, source) in draft {
            let key = app.clone();
            let next = if *source == BillingModelSource::Response { BillingModelSource::Request } else { BillingModelSource::Response };
            body = body.push(row![text(app).width(Length::Fill), text(if *source == BillingModelSource::Response { "按返回模型计价" } else { "按请求模型计价" }),
                button(text("切换")).style(rounded_action_btn_style).padding([8, 14]).on_press_maybe((!state.billing_busy && !state.busy).then(|| message(Event::BillingField(key, next))))].spacing(8));
        }
        body = body.push(row![action("保存配置", Event::BillingSave, !state.billing_busy && !state.busy), action("取消", Event::BillingCancel, !state.billing_busy)].spacing(8));
    } else if let Some(saved) = &state.billing {
        for (app, source) in saved { body = body.push(text(format!("{} · {}", app, if *source == BillingModelSource::Response { "按返回模型计价" } else { "按请求模型计价" }))); }
        body = body.push(action("修改…", Event::BillingEdit, !state.billing_busy && !state.busy));
    } else { body = body.push(text(if state.billing_busy { "正在加载计费配置…" } else { "计费配置加载失败，请重新加载" })); }
    settings_modal_card(body).into()
}
