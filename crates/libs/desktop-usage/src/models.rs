//! The host owns family selection and price conversion; the desktop edits intent only.
use crate::Identity;
use vw_gateway_client::GatewayClient;
type Message = Event;
use desktop_ui_controls::system_settings_common::*;
use desktop_assets::assets::Icon;
use iced::{Element, Length, Task, Theme, widget::{button, checkbox, column, container, row, rule, text}};
use gateway_usage_price::{ModelsDevEntry, ModelsDevSyncConfig, ModelsDevSyncState};
use gateway_usage_scan::UsageScanScope;

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
    picker: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    SavedNotice, Open, Reload, Picker(bool), Search(String), Auto(bool), Common(bool),
    Continue(u64, Identity),
    Select(String, bool), Exclude(String, bool), Save, Sync, AutoSync, Poll,
    Loaded(u64, Identity, Result<(Vec<ModelsDevEntry>, ModelsDevSyncState), String>),
    Saved(u64, u64, Identity, Result<ModelsDevSyncState, String>),
    Synced(u64, Identity, Result<String, String>),
    Polled(u64, Identity, Result<(String, ModelsDevSyncState), String>),
}
fn message(event: Event) -> Message { event }
fn scope(id: &Identity) -> UsageScanScope { UsageScanScope { project_id: id.project.clone().into(), target_host: id.host.clone() } }
fn set_key(keys: &mut Vec<String>, key: String, enabled: bool) {
    keys.retain(|value| value != &key);
    if enabled { keys.push(key); }
}
impl State {
    pub fn close(&mut self) { self.open = false; self.picker = false; self.sequence = self.sequence.wrapping_add(1); self.busy = false; }
}
pub fn update_with_client(state: &mut State, current: Option<Identity>, client: Result<GatewayClient, String>, event: Event) -> Task<Message> {
    // Every completion is bound to this opening and authenticated destination.
    let valid = |ticket, id: &Identity| state.open && state.sequence == ticket && state.scope.as_ref() == Some(id) && current.as_ref() == Some(id);
    match event {
        Event::SavedNotice => {},
        Event::Loaded(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result { Ok((entries, result)) => { state.candidates = entries; state.draft = Some(result.config.clone()); state.result = Some(result); return update_with_client(state, current, client, Event::AutoSync); }, Err(e) => state.error = Some(e) }
        }
        Event::Saved(ticket, revision, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result {
                Ok(result) => {
                    if state.revision == revision {
                        state.draft = Some(result.config.clone());
                        state.picker = false;
                    }
                    state.result = Some(result);
                    return Task::batch([Task::done(Event::SavedNotice), update_with_client(state, current, client, Event::AutoSync)]);
                }
                Err(e) => state.error = Some(e),
            }
        }
        Event::Synced(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            match result { Ok(job) => { state.job = Some(job); state.progress = "同步任务已接受，请刷新结果".into(); return update_with_client(state, current, client, Event::Poll); }, Err(e) => state.error = Some(e) }
        }
        Event::Polled(ticket, id, result) => {
            if !valid(ticket, &id) { return Task::none(); }
            state.busy = false;
            // Polling never replaces the editable configuration.
            match result {
                Ok((progress, result)) => {
                    let terminal = sync_finished(&progress);
                    state.progress = if terminal { "同步任务已结束".into() } else { "正在同步模型价格…".into() };
                    state.result = Some(result);
                    if !terminal {
                        return Task::perform(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            Event::Continue(ticket, id)
                        }, message);
                    }
                }
                Err(e) => state.error = Some(e),
            }
        }
        Event::Continue(ticket, id) => {
            if !valid(ticket, &id) { return Task::none(); }
            return update_with_client(state, current, client, Event::Poll);
        }
        Event::Picker(value) => {
            if value {
                if state.busy || state.draft.is_none() { return Task::none(); }
                state.search.clear();
            } else if !state.busy {
                state.draft = state.result.as_ref().map(|result| result.config.clone());
            } else {
                return Task::none();
            }
            state.picker = value;
        }
        Event::Search(value) => state.search = value,
        Event::Auto(value) => {
            if state.busy { return Task::none(); }
            if let Some(draft) = &mut state.draft { draft.auto_sync_enabled = value; state.revision += 1; }
            return update_with_client(state, current, client, Event::Save);
        }
        Event::Common(value) => { if state.busy { return Task::none(); } if let Some(draft) = &mut state.draft { draft.selection.include_common_models = value; state.revision += 1; } }
        Event::Select(key, enabled) => { if state.busy { return Task::none(); } if let Some(draft) = &mut state.draft { set_key(&mut draft.selection.selected_model_keys, key, enabled); state.revision += 1; } }
        Event::Exclude(key, enabled) => { if state.busy { return Task::none(); } if let Some(draft) = &mut state.draft { set_key(&mut draft.selection.excluded_common_model_keys, key, enabled); state.revision += 1; } }
        Event::Open | Event::Reload | Event::Save | Event::Sync | Event::AutoSync | Event::Poll => {
            if state.busy { return Task::none(); }
            if matches!(event, Event::Open | Event::Reload) { state.open = true; }
            let Some(id) = current else { state.error = Some("网关不可用，请检查连接".into()); return Task::none(); };
            if matches!(event, Event::AutoSync) && !state.result.as_ref().is_some_and(|s| s.config.auto_sync_enabled) { return Task::none(); }
            if matches!(event, Event::Sync) && state.draft.as_ref().zip(state.result.as_ref()).is_some_and(|(draft, saved)| draft != &saved.config) {
                state.error = Some("请先保存模型选择，再立即同步".into());
                return Task::none();
            }
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

            return Task::perform(async move {
                let request_scope = scope(&id);
                let response = match client {
                    Err(e) => Event::Synced(ticket, id, Err(e)),
                    Ok(client) => {
                        let scan = gateway_usage_scan::GatewayUsageScanClient::new(client.transport().clone());
                        let client = gateway_usage_price::GatewayUsagePriceClient::new(client.transport().clone());
                        match event {
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
                                    let snapshot = scan.usage_job(&request_scope, &job).await.map_err(|e| e.to_string())?;
                                    let config = client.usage_models_dev_config(&request_scope).await.map_err(|e| e.to_string())?;
                                    Ok((serde_json::to_string(&snapshot.progress).map_err(|e| e.to_string())?, config))
                                }.await;
                                Event::Polled(ticket, id, result)
                            }
                        }
                    }
                };
                response
            }, message);
        }
    }
    Task::none()
}

pub(crate) fn sync_finished(progress: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(progress).ok()
        .and_then(|value| value.get("phase").and_then(|phase| phase.as_str()).map(|phase| phase == "Terminal"))
        .unwrap_or(true)
}

impl State {
    pub(crate) fn candidates(&self) -> &[ModelsDevEntry] { &self.candidates }
    pub(crate) fn catalog_ready(&self) -> bool { self.result.is_some() }
}

fn action(label: &'static str, event: Event, enabled: bool) -> iced::widget::Button<'static, Event> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(event))
}

pub(crate) fn picker<'a, M: Clone + 'static>(
    state: &'a State,
    base: Element<'a, M>,
    map: impl Fn(Event) -> M + Copy + 'a,
) -> Element<'a, M> {
    let Some(draft) = state.draft.as_ref().filter(|_| state.picker) else { return base };
    let enabled = !state.busy;
    let cancel = Event::Picker(false);
    let search = state.search.trim().to_lowercase();
    let mut body = column![
        checkbox(draft.selection.include_common_models)
            .label("包含常用模型")
            .style(settings_checkbox_style)
            .on_toggle_maybe(enabled.then_some(Event::Common)),
        settings_text_input("搜索模型或供应商", &state.search).on_input(Event::Search),
    ].spacing(10);
    if let Some(error) = &state.error { body = body.push(settings_error_banner(error)); }
    for entry in state.candidates.iter()
        .filter(|entry| format!("{} {} {}", entry.model_name, entry.provider_name, entry.normalized_id).to_lowercase().contains(&search))
        .take(if search.is_empty() { 80 } else { 300 })
    {
        let selected = draft.selection.selected_model_keys.contains(&entry.key);
        let excluded = draft.selection.excluded_common_model_keys.contains(&entry.key);
        let key = entry.key.clone();
        let exclude_key = key.clone();
        body = body.push(row![
            column![
                text(&entry.model_name).size(14),
                text(format!("{} · {}", entry.provider_name, entry.normalized_id)).size(12).style(text::secondary),
            ].spacing(3).width(Length::Fill),
            checkbox(selected).label("选择").style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |value| Event::Select(key.clone(), value))),
            checkbox(excluded).label("排除常用").style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |value| Event::Exclude(exclude_key.clone(), value))),
        ].align_y(iced::Alignment::Center).spacing(12)).push(rule::horizontal(1));
    }
    let header = settings_modal_header(text("选择同步模型").size(20), settings_close_button(cancel.clone()));
    let changed = state.result.as_ref().is_some_and(|result| result.config != *draft);
    let footer = container(row![
        action("取消", cancel.clone(), enabled),
        action("保存选择", Event::Save, enabled && changed).style(primary_action_btn_style),
    ].spacing(8)).align_right(Length::Fill);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(720).height(590);
    settings_modal_overlay(Some(base), map(cancel), Element::from(card).map(map))
}

pub(crate) fn sync_card<'a, M: Clone + 'static>(
    state: &'a State,
    more: Element<'a, M>,
    map: fn(Event) -> M,
) -> Element<'a, M> {
    let ready = !state.busy && state.catalog_ready();
    let result = state.result.as_ref();
    let title = match result {
        Some(result) if result.config.auto_sync_enabled => "models.dev 同步 · 已开启",
        Some(_) => "models.dev 同步 · 已关闭",
        None => "models.dev 同步",
    };
    let summary = result.map_or_else(
        || if state.busy { "正在读取同步设置…".into() } else { "同步设置不可用".into() },
        |result| {
            let last = result.last_sync_at
                .and_then(chrono::DateTime::from_timestamp_millis)
                .map(|time| time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "从未同步".into());
            let common = if result.config.selection.include_common_models { " · 含常用模型" } else { "" };
            format!("最近同步 {last}{common}")
        },
    );
    let toggle = settings_toggle(result.is_some_and(|result| result.config.auto_sync_enabled))
        .on_toggle_maybe(ready.then_some(move |enabled| map(Event::Auto(enabled))));
    let controls = row![
        toggle,
        Element::from(action("选择模型", Event::Picker(true), ready)).map(map),
        Element::from(action("立即同步", Event::Sync, ready)).map(map),
        more,
    ].spacing(8).align_y(iced::Alignment::Center);
    let mut content = column![row![
        icon_svg(Icon::ArrowRepeat, 20.0),
        column![text(title).size(16), text(summary).size(12).style(text::secondary)]
            .spacing(4).width(Length::Fill),
        controls,
    ].spacing(12).align_y(iced::Alignment::Center)].spacing(8);
    if !state.progress.is_empty() { content = content.push(text(&state.progress).size(12).style(text::secondary)); }
    if let Some(error) = state.error.as_ref().or_else(|| result.and_then(|result| result.last_error.as_ref())) {
        content = content.push(settings_error_banner(error));
    }
    if !state.busy && state.error.is_some() {
        let retry = if state.job.is_some() { Event::Poll } else { Event::Reload };
        content = content.push(Element::from(action("重试", retry, true)).map(map));
    }
    container(content).padding([12, 14]).width(Length::Fill).style(|theme: &Theme| {
        container::Style::default().background(theme.extended_palette().background.weak.color)
            .border(iced::border::rounded(12))
    }).into()
}
