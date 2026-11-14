//! Defaults form consumes the gateway section; configuration IO stays on the executor.
use crate::app::components::system_settings_common::*;
use crate::app::{App, Message, state::AgentAppsContext};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::{OpenClawSettingsScope, vw_api_types::openclaw_settings::*};

const FIELDS: [(&str, &str); 4] = [
    ("workspace", "工作区"),
    ("timeoutSeconds", "超时秒数"),
    ("contextTokens", "上下文令牌数"),
    ("maxConcurrent", "最大并发数"),
];
#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    loaded: bool,
    generation: u64,
    original: OpenClawAgentsDefaults,
    inputs: [String; 4],
    errors: [Option<String>; 4],
    warnings: Vec<OpenClawHealthWarning>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Self::default() };
    }
}
#[derive(Clone)]
pub struct Scope {
    context: AgentAppsContext,
    host: String,
    project: String,
    catalog: u64,
    generation: u64,
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Edit(usize, String),
    Save,
    Loaded(Scope, Result<(Option<OpenClawAgentsDefaults>, Vec<OpenClawHealthWarning>), String>),
    Saved(Scope, Result<OpenClawWriteOutcome, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpenClawDefaultsEvent")
    }
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawDefaults(event),
    ))
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("代理默认设置"))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open))
        .into()
}
// Match the domain's form projection, including numeric legacy timeout fallback.
fn project_fields(value: &OpenClawAgentsDefaults) -> [String; 4] {
    std::array::from_fn(|i| {
        let v = value.0.get(FIELDS[i].0);
        if i == 0 {
            return v.and_then(serde_json::Value::as_str).unwrap_or("").into();
        }
        v.filter(|v| v.is_number())
            .or_else(|| {
                (i == 1).then(|| value.0.get("timeout").filter(|v| v.is_number())).flatten()
            })
            .map(ToString::to_string)
            .unwrap_or_default()
    })
}
fn candidate(
    original: &OpenClawAgentsDefaults,
    inputs: &[String; 4],
) -> Result<OpenClawAgentsDefaults, [Option<String>; 4]> {
    let mut next = original.clone();
    let mut errors: [Option<String>; 4] = Default::default();
    for (i, (key, _)) in FIELDS.iter().enumerate() {
        let input = inputs[i].trim();
        if input.is_empty() {
            next.0.remove(*key);
        } else if i == 0 {
            next.0.insert((*key).into(), input.into());
        } else {
            match input.parse::<f64>().ok().and_then(serde_json::Number::from_f64) {
                Some(number) => {
                    next.0.insert((*key).into(), number.into());
                }
                None => errors[i] = Some("请输入有限数字".into()),
            }
        }
    }
    // Clear only the known numeric alias; never migrate unrelated legacy data.
    if next.0.get("timeout").is_some_and(serde_json::Value::is_number) {
        next.0.remove("timeout");
    }
    if errors.iter().any(Option::is_some) { Err(errors) } else { Ok(next) }
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let close = message(Event::Close);
    let mut body = column![text("留空可清除对应字段；模型与其他默认配置将保留。")].spacing(12);
    if state.busy {
        body = body.push(text("正在处理…"));
    }
    for warning in &state.warnings {
        body = body.push(text(format!(
            "{}{}",
            warning.message,
            warning.path.as_ref().map(|p| format!(" ({p})")).unwrap_or_default()
        )));
    }
    if state.loaded {
        for (index, (_, label)) in FIELDS.iter().enumerate() {
            let input = settings_text_input("未设置", &state.inputs[index]).width(Length::Fill);
            body = body.push(text(*label)).push(if state.busy {
                input
            } else {
                input.on_input(move |v| message(Event::Edit(index, v)))
            });
            if let Some(error) = &state.errors[index] {
                body = body.push(text(error).style(iced::widget::text::danger));
            }
        }
    }
    let action = if !state.loaded { Event::Open } else { Event::Save };
    let footer = row![
        button(text("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(text(if state.loaded { "保存" } else { "重试" }))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe((!state.busy).then(|| message(action)))
    ]
    .spacing(8);
    let header = settings_modal_header(
        text("OpenClaw 代理默认设置").size(20),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_layout(header, body, Some(footer.into())).width(620).height(560);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.openclaw_defaults.invalidate();
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用");
    };
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let Some(project) = app.project_id.clone().filter(|p| !p.is_empty()) else {
        return app.show_error_toast("请先选择项目");
    };
    let valid = |scope: &Scope, app: &App| {
        scope.context == context
            && scope.project == project.to_string()
            && scope.host == app.apps_catalog.host
            && scope.catalog == app.apps_catalog.generation
            && scope.generation == app.apps_catalog.openclaw_defaults.generation
    };
    match event {
        Event::Loaded(scope, result) => {
            if !valid(&scope, app) {
                return Task::none();
            }
            let state = &mut app.apps_catalog.openclaw_defaults;
            state.busy = false;
            match result {
                Ok((defaults, warnings)) => {
                    state.original = defaults.unwrap_or_default();
                    state.inputs = project_fields(&state.original);
                    state.warnings = warnings;
                    state.loaded = true;
                }
                Err(error) => return app.show_error_toast(error),
            }
            return Task::none();
        }
        Event::Saved(scope, result) => {
            if !valid(&scope, app) {
                return Task::none();
            }
            app.apps_catalog.openclaw_defaults.busy = false;
            return match result {
                Ok(outcome) => {
                    app.apps_catalog.openclaw_defaults.warnings = outcome.warnings;
                    app.show_success_toast("默认设置已保存；请留意配置警告")
                }
                Err(error) => app.show_error_toast(error),
            };
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        return app.show_error_toast("请先连接应用服务");
    }
    let state = &mut app.apps_catalog.openclaw_defaults;
    if state.busy {
        return Task::none();
    }
    if !matches!(event, Event::Open) && (!state.open || !state.loaded) {
        return Task::none();
    }
    match event {
        Event::Edit(index, value) => {
            if index < 4 {
                state.inputs[index] = value;
                state.errors[index] = None;
            }
            return Task::none();
        }
        Event::Open => {
            state.invalidate();
            state.open = true;
        }
        Event::Save => {}
        _ => return Task::none(),
    }
    let candidate = if matches!(event, Event::Save) {
        match candidate(&state.original, &state.inputs) {
            Ok(value) => Some(value),
            Err(errors) => {
                state.errors = errors;
                return Task::none();
            }
        }
    } else {
        None
    };
    state.busy = true;
    state.generation = state.generation.wrapping_add(1);
    let scope = Scope {
        context,
        host: app.apps_catalog.host.clone(),
        project: project.to_string(),
        catalog: app.apps_catalog.generation,
        generation: state.generation,
    };
    let query =
        OpenClawSettingsScope { project_id: project.into(), target_host_id: scope.host.clone() };
    let token = app.apps_catalog.token.clone();
    if let Some(value) = candidate {
        return Task::perform(
            async move {
                let result = client
                    .set_openclaw_settings_defaults(&query, &value, &token)
                    .await
                    .map_err(|e| e.to_string());
                (scope, result)
            },
            |(scope, result)| message(Event::Saved(scope, result)),
        );
    }
    Task::perform(
        async move {
            let result = async {
                let defaults = client
                    .openclaw_settings_defaults(&query, &token)
                    .await
                    .map_err(|e| e.to_string())?;
                let warnings = client
                    .openclaw_settings_health(&query, &token)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok((defaults, warnings))
            }
            .await;
            (scope, result)
        },
        |(scope, result)| message(Event::Loaded(scope, result)),
    )
}

#[cfg(test)]
#[path = "openclaw_defaults_tests.rs"]
mod tests;
