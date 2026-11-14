//! The host owns family selection and price conversion; the desktop edits intent only.
use super::{identity, Identity, Event as CenterEvent};
use crate::app::{App, Message, message::ViewMessage};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task, widget::{button, checkbox, column, row, text}};
use vw_gateway_client::client::{ModelsDevEntry, ModelsDevSyncConfig, ModelsDevSyncState, UsageScanScope};

#[derive(Default)]
pub struct State {
    pub open: bool,
    sequence: u64,
    revision: u64,
    scope: Option<Identity>,
    busy: bool,
    search: String,
    candidates: Vec<ModelsDevEntry>,
    draft: Option<ModelsDevSyncConfig>,
    result: Option<ModelsDevSyncState>,
    error: Option<String>,
    job: Option<String>,
    progress: String,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open, Close, Reload, Search(String), Auto(bool), Common(bool),
    Select(String, bool), Exclude(String, bool), Save, Sync, AutoSync, Poll,
    Loaded(u64, Identity, Result<(Vec<ModelsDevEntry>, ModelsDevSyncState), String>),
    Saved(u64, u64, Identity, Result<ModelsDevSyncState, String>),
    Synced(u64, Identity, Result<String, String>),
    Polled(u64, Identity, Result<(String, ModelsDevSyncState), String>),
}
fn message(event: Event) -> Message { Message::View(ViewMessage::UsageCenter(CenterEvent::Models(event))) }
fn scope(id: &Identity) -> UsageScanScope { UsageScanScope { project_id: id.project.clone().into(), target_host: id.host.clone() } }
fn set_key(keys: &mut Vec<String>, key: String, enabled: bool) {
    keys.retain(|value| value != &key);
    if enabled { keys.push(key); }
}
impl State {
    pub fn close(&mut self) { self.open = false; self.sequence = self.sequence.wrapping_add(1); self.busy = false; }
}
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = identity(app);
    let state = &mut app.usage_center.models;
    // Every completion is bound to this opening and authenticated destination.
    let valid = |ticket, id: &Identity| state.open && state.sequence == ticket && state.scope.as_ref() == Some(id) && current.as_ref() == Some(id);
    match event {
        Event::Close => { state.open = false; state.sequence = state.sequence.wrapping_add(1); state.busy = false; }
        Event::Loaded(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result { Ok((entries, result)) => { state.candidates = entries; state.draft = Some(result.config.clone()); state.result = Some(result); return update(app, Event::AutoSync); }, Err(e) => state.error = Some(e) }
        }
        Event::Saved(ticket, revision, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result {
                Ok(result) => {
                    if state.revision == revision { state.draft = Some(result.config.clone()); }
                    state.result = Some(result);
                    return Task::batch([app.show_success_toast("模型同步选择已保存"), update(app, Event::AutoSync)]);
                }
                Err(e) => state.error = Some(e),
            }
        }
        Event::Synced(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result { Ok(job) => { state.job = Some(job); state.progress = "同步任务已接受，请刷新结果".into(); }, Err(e) => state.error = Some(e) }
        }
        Event::Polled(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            // Polling never replaces the editable configuration.
            match result { Ok((progress, result)) => { state.progress = progress; state.result = Some(result); }, Err(e) => state.error = Some(e) }
        }
        Event::Search(value) => state.search = value,
        Event::Auto(value) => { if let Some(draft) = &mut state.draft { draft.auto_sync_enabled = value; state.revision += 1; } }
        Event::Common(value) => { if let Some(draft) = &mut state.draft { draft.selection.include_common_models = value; state.revision += 1; } }
        Event::Select(key, enabled) => { if let Some(draft) = &mut state.draft { set_key(&mut draft.selection.selected_model_keys, key, enabled); state.revision += 1; } }
        Event::Exclude(key, enabled) => { if let Some(draft) = &mut state.draft { set_key(&mut draft.selection.excluded_common_model_keys, key, enabled); state.revision += 1; } }
        Event::Open | Event::Reload | Event::Save | Event::Sync | Event::AutoSync | Event::Poll => {
            if state.busy { return Task::none(); }
            if matches!(event, Event::Open | Event::Reload) { state.open = true; }
            let Some(id) = current else { state.error = Some("请选择项目与执行主机".into()); return Task::none(); };
            if matches!(event, Event::AutoSync) && !state.result.as_ref().is_some_and(|s| s.config.auto_sync_enabled) { return Task::none(); }
            let loading = matches!(event, Event::Open | Event::Reload);
            if !loading && state.scope.as_ref() != Some(&id) { state.error = Some("项目或主机已切换，请重新加载".into()); return Task::none(); }
            if loading {
                if state.scope.as_ref() != Some(&id) { state.draft = None; state.result = None; state.candidates.clear(); state.job = None; state.progress.clear(); }
                state.scope = Some(id.clone());
                state.sequence = state.sequence.wrapping_add(1);
            }
            let ticket = state.sequence;
            let revision = state.revision;
            let draft = state.draft.clone();
            let job = state.job.clone();
            state.busy = true;
            state.error = None;
            let client = crate::app::config::gateway_client();
            return Task::perform(async move {
                let request_scope = scope(&id);
                let response = match client {
                    Err(e) => Event::Synced(ticket, id, Err(e)),
                    Ok(client) => match event {
                        Event::Open | Event::Reload => {
                            let result = async {
                                let config = client.usage_models_dev_config(&request_scope).await.map_err(|e| e.to_string())?;
                                let entries = client.usage_models_dev_candidates(&request_scope).await.map_err(|e| e.to_string())?;
                                Ok((entries, config))
                            }.await;
                            Event::Loaded(ticket, id, result)
                        }
                        Event::Save => {
                            let result = match draft { Some(config) => client.usage_save_models_dev_config(&request_scope, &config).await.map_err(|e| e.to_string()), None => Err("请先加载配置".into()) };
                            Event::Saved(ticket, revision, id, result)
                        }
                        Event::AutoSync => Event::Synced(ticket, id, client.usage_auto_sync_models_dev(&request_scope).await.map(|r| r.task_id).map_err(|e| e.to_string())),
                        Event::Sync => Event::Synced(ticket, id, client.usage_sync_models_dev(&request_scope).await.map(|r| r.task_id).map_err(|e| e.to_string())),
                        _ => {
                            let result = async {
                                let job = job.ok_or_else(|| "尚无同步任务".to_string())?;
                                let snapshot = client.usage_job(&request_scope, &job).await.map_err(|e| e.to_string())?;
                                let config = client.usage_models_dev_config(&request_scope).await.map_err(|e| e.to_string())?;
                                Ok((serde_json::to_string(&snapshot.progress).map_err(|e| e.to_string())?, config))
                            }.await;
                            Event::Polled(ticket, id, result)
                        }
                    }
                };
                response
            }, message);
        }
    }
    Task::none()
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    button(text(label)).style(rounded_action_btn_style).padding([8, 14]).on_press_maybe(enabled.then(|| message(event))).into()
}
pub fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.usage_center.models;
    if !state.open { return base; }
    let mut body = column![text("家族规则及价格转换由执行主机处理；选择修改后请保存，再同步。")].spacing(10);
    if let Some(draft) = &state.draft {
        body = body.push(checkbox(draft.auto_sync_enabled).label("自动同步").style(settings_checkbox_style).on_toggle(|v| message(Event::Auto(v))))
            .push(checkbox(draft.selection.include_common_models).label("包含常用模型").style(settings_checkbox_style).on_toggle(|v| message(Event::Common(v))));
        body = body.push(text(format!("显式选择 {} · 排除常用 {}", draft.selection.selected_model_keys.len(), draft.selection.excluded_common_model_keys.len())));
        body = body.push(settings_text_input("搜索模型或供应商", &state.search).on_input(|v| message(Event::Search(v))));
        let search = state.search.to_lowercase();
        for entry in state.candidates.iter().filter(|e| format!("{} {} {} {}", e.key, e.model_name, e.provider_name, e.normalized_id).to_lowercase().contains(&search)) {
            let selected = draft.selection.selected_model_keys.contains(&entry.key);
            let excluded = draft.selection.excluded_common_model_keys.contains(&entry.key);
            let key = entry.key.clone();
            let exclude_key = key.clone();
            body = body.push(column![text(format!("{} · {} · {}", entry.model_name, entry.provider_name, entry.normalized_id)), row![
                checkbox(selected).label("显式选择").style(settings_checkbox_style).on_toggle(move |v| message(Event::Select(key.clone(), v))),
                checkbox(excluded).label("排除常用").style(settings_checkbox_style).on_toggle(move |v| message(Event::Exclude(exclude_key.clone(), v))),
            ].spacing(12)].spacing(4));
        }
    }
    let mut header = column![text("模型价格同步选择").size(20)].spacing(8);
    if state.busy { header = header.push(text("正在请求执行主机…")); }
    if let Some(result) = &state.result {
        header = header.push(text(format!("最近同步（Unix 毫秒）：{}", result.last_sync_at.map(|v| v.to_string()).unwrap_or_else(|| "从未同步".into()))));
        if let Some(error) = &result.last_error { header = header.push(settings_error_banner(error)); }
    }
    if let Some(error) = &state.error { header = header.push(settings_error_banner(error)); }
    if !state.progress.is_empty() { body = body.push(text(&state.progress)); }
    let footer = row![action("重新加载", Event::Reload, !state.busy), action("保存选择", Event::Save, !state.busy && state.draft.is_some()), action("强制同步", Event::Sync, !state.busy && state.draft.is_some()), action("刷新同步结果", Event::Poll, !state.busy && state.job.is_some()), action("关闭", Event::Close, true)].spacing(8);
    settings_modal_overlay(Some(base), message(Event::Close), settings_modal_layout(header, body, Some(footer.into())).width(900).height(680))
}
