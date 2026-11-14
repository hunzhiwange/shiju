//! Section-only model editor. Configuration text remains owned by the gateway.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text}};
use vw_gateway_client::{OpenClawSettingsScope, vw_api_types::openclaw_settings::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    loaded: bool,
    generation: u64,
    model: Option<OpenClawDefaultModel>,
    options: Vec<Choice>,
    warnings: Vec<OpenClawHealthWarning>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Choice { value: String, label: String }
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.label) }
}
#[derive(Clone)]
pub struct Scope { context: AgentAppsContext, host: String, project: String, catalog: u64, generation: u64 }
#[derive(Clone)]
pub enum Event {
    Open, Close, Primary(String), Fallback(usize, String), Add, Remove(usize), Save,
    Loaded(Scope, Result<(Option<OpenClawDefaultModel>, Option<OpenClawModelCatalog>, Vec<OpenClawHealthWarning>), String>),
    Saved(Scope, Result<OpenClawWriteOutcome, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("OpenClawModelEvent") }
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawModel(event)))
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("默认模型与回退模型")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(message(Event::Open)).into()
}
fn empty_model() -> OpenClawDefaultModel {
    OpenClawDefaultModel { primary: String::new(), fallbacks: vec![], extra: Default::default() }
}
pub(crate) fn choices(options: &[Choice], current: &str, excluded: &[String]) -> Vec<Choice> {
    let mut result = vec![Choice { value: String::new(), label: "未设置".into() }];
    result.extend(options.iter().filter(|o| !excluded.contains(&o.value) || o.value == current).cloned());
    if !current.is_empty() && !result.iter().any(|o| o.value == current) {
        result.push(Choice { value: current.into(), label: format!("{current}（未配置）") });
    }
    result
}
pub(crate) fn normalize(model: &mut OpenClawDefaultModel) {
    let mut used = vec![model.primary.clone()];
    model.fallbacks.retain(|v| {
        if v.is_empty() || used.contains(v) { false } else { used.push(v.clone()); true }
    });
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let close = message(Event::Close);
    let mut body = column![text("模型选项来自执行主机的模型白名单；未配置的当前值仍保留。")].spacing(12);
    if state.busy { body = body.push(text("正在处理…")); }
    for warning in &state.warnings {
        body = body.push(text(format!("{}{}", warning.message, warning.path.as_ref().map(|p| format!(" ({p})")).unwrap_or_default())));
    }
    if state.loaded && !state.busy {
        let model = state.model.clone().unwrap_or_else(empty_model);
        let opts = choices(&state.options, &model.primary, &[]);
        let selected = opts.iter().find(|o| o.value == model.primary).cloned();
        body = body.push(text("默认模型")).push(settings_select(opts, selected, |c| message(Event::Primary(c.value))).width(Length::Fill));
        for (index, current) in model.fallbacks.iter().enumerate() {
            let mut excluded = model.fallbacks.clone();
            excluded.push(model.primary.clone());
            let opts = choices(&state.options, current, &excluded);
            let selected = opts.iter().find(|o| &o.value == current).cloned();
            body = body.push(row![settings_select(opts, selected, move |c| message(Event::Fallback(index, c.value))).width(Length::Fill),
                button(text("移除")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Remove(index)))].spacing(8));
        }
        body = body.push(button(text("添加回退模型")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Add)));
    }
    let footer = row![button(text("取消")).padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
        button(text("保存")).padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe((state.loaded && !state.busy).then(|| message(Event::Save)))].spacing(8);
    let header = settings_modal_header(text("OpenClaw 模型选择").size(20), settings_close_button(close.clone()));
    let card = settings_modal_layout(header, body, Some(footer.into())).width(620).height(520);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { app.apps_catalog.openclaw_model.invalidate(); return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
    let valid = |scope: &Scope, app: &App| scope.context == context && scope.project == project.to_string()
        && scope.host == app.apps_catalog.host && scope.catalog == app.apps_catalog.generation
        && scope.generation == app.apps_catalog.openclaw_model.generation;
    match event {
        Event::Loaded(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            let state = &mut app.apps_catalog.openclaw_model;
            state.busy = false;
            match result {
                Ok((model, catalog, warnings)) => {
                    state.model = model;
                    state.options = catalog.unwrap_or_default().into_iter().map(|(value, entry)| Choice {
                        label: entry.alias.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| value.clone()), value,
                    }).collect();
                    state.warnings = warnings;
                    state.loaded = true;
                }
                Err(error) => return app.show_error_toast(error),
            }
            return Task::none();
        }
        Event::Saved(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            app.apps_catalog.openclaw_model.busy = false;
            return match result {
                Ok(outcome) => {
                    app.apps_catalog.openclaw_model.warnings = outcome.warnings;
                    app.show_success_toast("模型已保存；请留意配置警告")
                }
                Err(error) => app.show_error_toast(error),
            };
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用服务"); }
    let state = &mut app.apps_catalog.openclaw_model;
    if state.busy { return Task::none(); }
    if !matches!(event, Event::Open) && (!state.open || !state.loaded) { return Task::none(); }
    match event {
        Event::Primary(value) => { state.model.get_or_insert_with(empty_model).primary = value; return Task::none(); }
        Event::Fallback(index, value) => { if let Some(slot) = state.model.get_or_insert_with(empty_model).fallbacks.get_mut(index) { *slot = value; } return Task::none(); }
        Event::Add => { state.model.get_or_insert_with(empty_model).fallbacks.push(String::new()); return Task::none(); }
        Event::Remove(index) => { let model = state.model.get_or_insert_with(empty_model); if index < model.fallbacks.len() { model.fallbacks.remove(index); } return Task::none(); }
        Event::Open => { state.invalidate(); state.open = true; }
        Event::Save => {}
        _ => return Task::none(),
    }
    state.busy = true;
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.to_string(), catalog: app.apps_catalog.generation, generation: state.generation };
    let query = OpenClawSettingsScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    if matches!(event, Event::Save) {
        let mut model = state.model.clone().unwrap_or_else(empty_model);
        normalize(&mut model);
        state.model = Some(model.clone());
        return Task::perform(async move {
            let result = client.set_openclaw_settings_model(&query, &model, &token).await.map_err(|e| e.to_string());
            (scope, result)
        }, |(scope, result)| message(Event::Saved(scope, result)));
    }
    Task::perform(async move {
        let result = async {
            let model = client.openclaw_settings_model(&query, &token).await.map_err(|e| e.to_string())?;
            let catalog = client.openclaw_settings_catalog(&query, &token).await.map_err(|e| e.to_string())?;
            let warnings = client.openclaw_settings_health(&query, &token).await.map_err(|e| e.to_string())?;
            Ok((model, catalog, warnings))
        }.await;
        (scope, result)
    }, |(scope, result)| message(Event::Loaded(scope, result)))
}
