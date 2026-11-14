//! Section-only catalog editor. Configuration text remains owned by the gateway.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text, text_editor}};
use vw_gateway_client::{OpenClawSettingsScope, vw_api_types::openclaw_settings::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    loaded: bool,
    generation: u64,
    editor: text_editor::Content,
    missing: bool,
    warnings: Vec<OpenClawHealthWarning>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone)]
pub struct Scope { context: AgentAppsContext, host: String, project: String, catalog: u64, generation: u64 }
#[derive(Clone)]
pub enum Event {
    Open, Close, Edit(text_editor::Action), Save,
    Loaded(Scope, Result<Option<OpenClawModelCatalog>, String>),
    Saved(Scope, Result<OpenClawWriteOutcome, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("OpenClawCatalogEvent") }
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawCatalog(event)))
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("模型白名单")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press(message(Event::Open)).into()
}
pub(crate) fn parse_draft(value: &str) -> Result<OpenClawModelCatalog, String> {
    serde_json::from_str(value).map_err(|_| "请输入模型目录 JSON 对象，每个模型的 alias 必须为字符串，额外字段可保留任意 JSON 值".into())
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let close = message(Event::Close);
    let mut body = column![text("以 JSON 对象编辑模型白名单、alias 和额外字段。保存 {} 会明确写入空目录。")].spacing(12);
    if state.busy { body = body.push(text("正在处理…")); }
    for warning in &state.warnings {
        body = body.push(text(format!("{}{}", warning.message, warning.path.as_ref().map(|p| format!(" ({p})")).unwrap_or_default())));
    }
    if state.loaded {
        if state.missing { body = body.push(text("当前配置没有模型目录；保存后将创建该分节。")); }
        let mut editor = settings_text_editor(&state.editor).height(260);
        if !state.busy { editor = editor.on_action(|action| message(Event::Edit(action))); }
        body = body.push(editor);
    } else if !state.busy {
        body = body.push(button(text("重试读取")).padding([8, 12]).style(rounded_action_btn_style).on_press(message(Event::Open)));
    }
    let footer = row![button(text("取消")).padding([8, 12]).style(rounded_action_btn_style).on_press(close.clone()),
        button(text("保存")).padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe((state.loaded && !state.busy).then(|| message(Event::Save)))].spacing(8);
    let header = settings_modal_header(text("OpenClaw 模型白名单").size(20), settings_close_button(close.clone()));
    let card = settings_modal_layout(header, body, Some(footer.into())).width(620).height(520);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { app.apps_catalog.openclaw_catalog.invalidate(); return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
    let valid = |scope: &Scope, app: &App| scope.context == context && scope.project == project.to_string()
        && scope.host == app.apps_catalog.host && scope.catalog == app.apps_catalog.generation
        && scope.generation == app.apps_catalog.openclaw_catalog.generation
        && app.apps_catalog.openclaw_catalog.open;
    match event {
        Event::Loaded(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            let state = &mut app.apps_catalog.openclaw_catalog;
            state.busy = false;
            match result {
                Ok(catalog) => {
                    state.missing = catalog.is_none();
                    let value = serde_json::to_string_pretty(&catalog.unwrap_or_default())
                        .expect("catalog consists of JSON values");
                    state.editor = text_editor::Content::with_text(&value);
                    state.loaded = true;
                }
                Err(error) => return app.show_error_toast(error),
            }
            return Task::none();
        }
        Event::Saved(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            app.apps_catalog.openclaw_catalog.busy = false;
            return match result {
                Ok(outcome) => {
                    app.apps_catalog.openclaw_catalog.warnings = outcome.warnings;
                    app.apps_catalog.openclaw_catalog.missing = false;
                    app.show_success_toast("模型白名单已保存；请留意配置警告")
                }
                Err(error) => app.show_error_toast(error),
            };
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用服务"); }
    let state = &mut app.apps_catalog.openclaw_catalog;
    if state.busy { return Task::none(); }
    if !matches!(event, Event::Open) && (!state.open || !state.loaded) { return Task::none(); }
    match event {
        Event::Edit(action) => { state.editor.perform(action); return Task::none(); }
        Event::Open => { state.invalidate(); state.open = true; }
        Event::Save => {}
        _ => return Task::none(),
    }
    let value = if matches!(event, Event::Save) {
        match parse_draft(&state.editor.text()) {
            Ok(value) => Some(value),
            Err(error) => return app.show_error_toast(error),
        }
    } else { None };
    state.busy = true;
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope { context, host: app.apps_catalog.host.clone(), project: project.to_string(), catalog: app.apps_catalog.generation, generation: state.generation };
    let query = OpenClawSettingsScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    if let Some(value) = value {
        return Task::perform(async move {
            let result = client.set_openclaw_settings_catalog(&query, &value, &token).await.map_err(|e| e.to_string());
            (scope, result)
        }, |(scope, result)| message(Event::Saved(scope, result)));
    }
    Task::perform(async move {
        let result = client.openclaw_settings_catalog(&query, &token).await.map_err(|e| e.to_string());
        (scope, result)
    }, |(scope, result)| message(Event::Loaded(scope, result)))
}
