//! Host-owned prices: mutations are acknowledged before refreshing the ledger.
use crate::Identity;
use super::Outcome;

use desktop_ui_controls::{
    selectable_text::markdown_selection::SelectionMessage,
    text_editor_context_menu::TextContextEffect,
};
use iced::{Task, widget::text_editor};
use gateway_usage_price::{ModelPricingInfo, StoredModelPricing, BillingDefaults, BillingModelSource, UsagePricingStorage};

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
    import_view: Option<ImportView>,
    import_search: String,
    models: crate::models::State,
    files_open: bool,
    page: usize,
    billing: Option<BillingDefaults>,
    billing_draft: Option<BillingDefaults>,
    billing_busy: bool,
    storage: Option<UsagePricingStorage>,
    storage_busy: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ImportView { Catalog, Json }

#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Help(TextContextEffect),
    None,
    Models(crate::models::Event), Files, CloseFiles, CancelDelete, Page(usize),
    BillingEdit, BillingCancel, BillingField(String, BillingModelSource), BillingSave,
    BillingLoaded(u64, Identity, Result<BillingDefaults, String>),
    StorageOpen, StorageReload,
    StorageLoaded(u64, Identity, Result<UsagePricingStorage, String>),
    Open, Close, Reload, RefreshPrices, New, Edit(usize), CancelEdit,
    Field(usize, String), Source(String), ImportEdit(text_editor::Action),
    ShowImport(Option<ImportView>), ImportSearch(String), ImportModel(String),
    Save, Delete(String), ConfirmDelete(String), Import,
    Loaded(u64, Identity, Result<Vec<StoredModelPricing>, String>),
    Written(u64, Identity, Result<(), String>),
}

impl SelectionMessage for Outcome<Event> {
    fn none() -> Self {
        message(Event::None)
    }

    fn copy(text: String) -> Self {
        Self::context(TextContextEffect::Copy(text))
    }

    fn context(effect: TextContextEffect) -> Self {
        message(Event::Help(effect))
    }

    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::none()
    }
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
            if matches!(event, crate::models::Event::SavedNotice) {
                return Task::done(Outcome::Success("模型同步设置已保存".into()));
            }
            let refresh = matches!(&event, crate::models::Event::Polled(_, id, Ok((progress, _)))
                if current.as_ref() == Some(id) && state.open && state.scope.as_ref() == Some(id)
                    && crate::models::sync_finished(progress));
            let task = crate::models::update_with_client(&mut state.models, current.clone(), client.clone(), event)
                .map(|event| message(Event::Models(event)));
            if refresh { return Task::batch([task, update(state, current, visible, client, Event::RefreshPrices)]); }
            return task;
        }
        Event::BillingLoaded(ticket, id, result) => {
            if !state.open || state.sequence != ticket || current.as_ref() != Some(&id) || state.scope.as_ref() != Some(&id) { return Task::none(); }
            state.billing_busy = false;
            match result {
                Ok(value) => {
                    state.billing = Some(value);
                    if state.billing_draft.take().is_some() {
                        return Task::done(Outcome::Success("计费默认配置已保存".into()));
                    }
                }
                Err(error) => state.error = Some(error),
            }
        }
        Event::StorageLoaded(ticket, id, result) => {
            if !state.open || state.sequence != ticket || current.as_ref() != Some(&id) || state.scope.as_ref() != Some(&id) { return Task::none(); }
            state.storage_busy = false;
            match result { Ok(value) => state.storage = Some(value), Err(error) => state.error = Some(error) }
        }
        Event::BillingEdit if !state.billing_busy && !state.busy => {
            state.error = None;
            state.billing_draft = state.billing.clone();
        }
        Event::BillingCancel if !state.billing_busy => {
            state.billing_draft = None;
            state.error = None;
        }
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
            state.error = None;
            let open = matches!(event, Event::StorageOpen);
            return Task::perform(async move {
                let client = client.map(|client| gateway_usage_price::GatewayUsagePriceClient::new(client.transport().clone()));
                if billing {
                    let result = async { client?.usage_save_billing_defaults(&draft.ok_or("请先编辑计费配置")?).await.map_err(|error| error.to_string()) }.await;
                    Event::BillingLoaded(ticket, id, result)
                } else {
                    let result = async { client?.usage_pricing_storage(open).await.map_err(|error| error.to_string()) }.await;
                    Event::StorageLoaded(ticket, id, result)
                }
            }, message);
        }
        Event::BillingEdit | Event::BillingCancel | Event::BillingField(..) => {},
        Event::CloseFiles => state.files_open = false,
        Event::CancelDelete => state.pending_delete = None,
        Event::Page(page) => state.page = page.min(state.prices.len().saturating_sub(1) / 20),
        Event::Feedback(..) | Event::Help(_) | Event::None => {},
        Event::Close if !state.busy && !state.billing_busy && !state.storage_busy => { state.close(); }
        Event::Open | Event::Reload | Event::RefreshPrices => {
            if matches!(event, Event::Reload) && (state.billing_busy || state.storage_busy) { return Task::none(); }
            if state.busy { return Task::none(); }
            let load_models = matches!(event, Event::Open);
            let load_billing = !matches!(event, Event::RefreshPrices);
            state.open = true;
            let Some(scope) = current else { state.error = Some("网关不可用，请检查连接".into()); return Task::none(); };
            if state.scope.as_ref() != Some(&scope) {
                state.editor = None;
                state.pending_delete = None;
                state.page = 0;
                state.models.close();
                state.billing = None; state.billing_draft = None; state.storage = None; state.billing_busy = false; state.storage_busy = false;
                state.prices.clear();
                state.import = text_editor::Content::new();
                state.import_view = None;
                state.import_search.clear();
                state.files_open = false;
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
                let result = async { gateway_usage_price::GatewayUsagePriceClient::new(billing_client?.transport().clone()).usage_billing_defaults().await.map_err(|error| error.to_string()) }.await;
                Event::BillingLoaded(ticket, id, result)
            }, message) } else { Task::none() };
            let prices = Task::perform(async move {
                let result = async {
                    gateway_usage_price::GatewayUsagePriceClient::new(client?.transport().clone()).usage_prices().await.map_err(|e| e.to_string())
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
                        state.import_view = None;
                        return Task::batch([update(state, current, visible, client, Event::Reload), Task::done(Outcome::Refresh), Task::done(Outcome::Success("价格已更新".into()))]);
                    }
                }
            }
        }
        _ if state.busy => {}
        Event::New => {
            state.creating = true;
            state.error = None;
            state.editor = Some(ModelPricingInfo { model_id: "".into(), display_name: String::new(), input_cost_per_million: "0".into(), output_cost_per_million: "0".into(), cache_read_cost_per_million: "0".into(), cache_creation_cost_per_million: "0".into() });
        }
        Event::Edit(index) => {
            if let Some(price) = state.prices.get(index) {
                state.editor = Some(price.price.clone());
                state.creating = false;
                state.error = None;
            }
        }
        Event::CancelEdit => {
            state.editor = None;
            state.error = None;
        }
        Event::ShowImport(view) => {
            state.files_open = false;
            state.import_view = view;
            state.import_search.clear();
            state.error = None;
        }
        Event::ImportSearch(value) => state.import_search = value,
        Event::ImportModel(key) => {
            let Some(entry) = state.models.candidates().iter().find(|entry| entry.key == key) else {
                return Task::none();
            };
            state.editor = Some(ModelPricingInfo {
                model_id: entry.normalized_id.clone().into(),
                display_name: entry.model_name.clone(),
                input_cost_per_million: entry.input.to_string(),
                output_cost_per_million: entry.output.to_string(),
                cache_read_cost_per_million: entry.cache_read.to_string(),
                cache_creation_cost_per_million: entry.cache_write.to_string(),
            });
            state.creating = true;
            state.import_view = None;
        }
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
                    match operation {
                        Write::Save(price, true) => client.usage_create_price(&price).await.map(|_| ()).map_err(|e| e.to_string()),
                        Write::Save(price, false) => client.usage_update_price(&price).await.map(|_| ()).map_err(|e| e.to_string()),
                        Write::Delete(id) => {
                            let result = client.usage_delete_price(&id).await.map_err(|e| e.to_string())?;
                            if result.deleted { Ok(()) } else { Err("价格不存在，未删除".into()) }
                        }
                        Write::Import(source, prices) => client.usage_import_prices(&source, &prices).await.map(|_| ()).map_err(|e| e.to_string()),
                    }
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Written(ticket, scope, result)));
        }
        Event::Close => {}
    }
    Task::none()
}


#[path = "pricing_view.rs"]
mod view;
pub use view::overlay;

#[cfg(test)]
#[path = "../usage_replica_08_tests.rs"]
mod tests;
