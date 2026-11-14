//! Sensitive env editor; payloads never enter Debug output or error messages.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length, Task, widget::{button, column, container, row, text, text_editor}};
use vw_gateway_client::{OpenClawSettingsScope, OpenClawSettingsError, vw_api_types::openclaw_settings::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    loaded: bool,
    generation: u64,
    editor: text_editor::Content,
    error: Option<String>,
    warnings: Vec<OpenClawHealthWarning>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
    fn finish_save(&mut self, result: Result<OpenClawWriteOutcome, String>) {
        self.busy = false;
        match result {
            Ok(outcome) => { self.error = None; self.warnings = outcome.warnings; }
            Err(error) => self.error = Some(error),
        }
    }
}
#[derive(Clone)]
pub struct Scope { context: AgentAppsContext, host: String, project: String, catalog: u64, generation: u64 }
#[derive(Clone)]
pub enum Event {
    Open, Close, Edit(text_editor::Action), Save,
    Loaded(Scope, Result<Option<OpenClawEnvConfig>, String>),
    Saved(Scope, Result<OpenClawWriteOutcome, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("OpenClawEnvEvent") }
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawEnv(event)))
}
fn safe_error(error: OpenClawSettingsError) -> String {
    match error {
        OpenClawSettingsError::Http(401 | 403) => "不可访问：需要环境配置敏感读取或写入授权".into(),
        OpenClawSettingsError::Http(409) => "配置冲突，草稿已保留".into(),
        OpenClawSettingsError::Http(400 | 422) => "服务端拒绝环境配置，草稿已保留".into(),
        _ => "环境配置请求失败，请检查连接后重试".into(),
    }
}
fn parse(draft: &str) -> Result<OpenClawEnvConfig, &'static str> {
    if draft.trim().is_empty() { return Err("环境配置不能为空，请使用 {} 表示空对象"); }
    serde_json::from_str(draft).map_err(|_| "环境配置必须是 JSON 对象，支持嵌套对象")
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("环境配置 JSON")).padding([8,12]).style(rounded_action_btn_style).on_press(message(Event::Open)).into()
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let close = message(Event::Close);
    let mut body = column![text("编辑完整 env 对象，支持 vars、shellEnv 等嵌套对象。内容可能包含密钥。")].spacing(12);
    if state.busy { body = body.push(text("正在处理…")); }
    if let Some(error) = &state.error { body = body.push(text(error)); }
    for warning in &state.warnings { body = body.push(text(&warning.message)); }
    if state.loaded {
        let mut editor = settings_text_editor(&state.editor).height(260);
        if !state.busy { editor = editor.on_action(|action| message(Event::Edit(action))); }
        body = body.push(editor);
    }
    let footer = row![
        button(text("取消")).padding([8,12]).style(rounded_action_btn_style).on_press(close.clone()),
        button(text("重试读取")).padding([8,12]).style(rounded_action_btn_style).on_press_maybe((!state.busy && !state.loaded).then(|| message(Event::Open))),
        button(text("保存")).padding([8,12]).style(primary_action_btn_style).on_press_maybe((state.loaded && !state.busy).then(|| message(Event::Save)))
    ].spacing(8);
    let header = settings_modal_header(text("OpenClaw 环境配置").size(20), settings_close_button(close.clone()));
    let card = settings_modal_layout(header, body, Some(footer.into())).width(640).height(520);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) { app.apps_catalog.openclaw_env.invalidate(); return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return app.show_error_toast("网关不可用"); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
    let valid = |scope: &Scope, app: &App| scope.context == context && scope.project == project.to_string()
        && scope.host == app.apps_catalog.host && scope.catalog == app.apps_catalog.generation
        && scope.generation == app.apps_catalog.openclaw_env.generation;
    match event {
        Event::Loaded(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            let state = &mut app.apps_catalog.openclaw_env;
            state.busy = false;
            match result {
                Ok(value) => {
                    state.editor = text_editor::Content::with_text(&serde_json::to_string_pretty(&value.unwrap_or_default()).unwrap_or_else(|_| "{}".into()));
                    state.loaded = true;
                }
                Err(error) => state.error = Some(error),
            }
            return Task::none();
        }
        Event::Saved(scope, result) => {
            if !valid(&scope, app) { return Task::none(); }
            app.apps_catalog.openclaw_env.busy = false;
            let state = &mut app.apps_catalog.openclaw_env;
            state.finish_save(result);
            return if state.error.is_some() { app.show_error_toast("环境配置保存失败，草稿已保留") }
                else { app.show_success_toast("环境配置已保存，请留意配置警告") };
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用服务"); }
    let state = &mut app.apps_catalog.openclaw_env;
    if state.busy { return Task::none(); }
    if !matches!(event, Event::Open) && (!state.open || !state.loaded) { return Task::none(); }
    match event {
        Event::Edit(action) => { state.editor.perform(action); state.error = None; return Task::none(); }
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
        let value = match parse(&state.editor.text()) {
            Ok(value) => value,
            Err(error) => { state.busy = false; state.error = Some(error.into()); return Task::none(); }
        };
        return Task::perform(async move {
            let result = client.set_openclaw_settings_env(&query, &value, &token).await.map_err(safe_error);
            (scope, result)
        }, |(scope, result)| message(Event::Saved(scope, result)));
    }
    Task::perform(async move {
        let result = client.openclaw_settings_env(&query, &token).await.map_err(safe_error);
        (scope, result)
    }, |(scope, result)| message(Event::Loaded(scope, result)))
}

#[cfg(test)]
#[path = "openclaw_env_tests.rs"]
mod tests;
