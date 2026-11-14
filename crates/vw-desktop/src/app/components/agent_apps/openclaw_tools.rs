//! OpenClaw tools section editor; writes remain on the authorized executor.
use crate::app::components::system_settings_common::*;
use crate::app::{App, Message, state::AgentAppsContext};
use iced::{
    Element, Length, Task,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::{
    OpenClawSettingsError, OpenClawSettingsScope, vw_api_types::openclaw_settings::*,
};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    loaded: bool,
    generation: u64,
    draft: OpenClawToolsConfig,
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
            Ok(outcome) => {
                self.error = None;
                self.warnings = outcome.warnings;
            }
            Err(error) => self.error = Some(error),
        }
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
    Profile(Profile),
    Edit(bool, usize, String),
    Add(bool),
    Remove(bool, usize),
    Save,
    Loaded(Scope, Result<Option<OpenClawToolsConfig>, String>),
    Saved(Scope, Result<OpenClawWriteOutcome, String>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpenClawToolsEvent")
    }
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::OpenClawTools(event),
    ))
}
fn safe_error(error: OpenClawSettingsError) -> String {
    match error {
        OpenClawSettingsError::Http(401 | 403) => "不可访问：需要工具配置读取或写入授权".into(),
        OpenClawSettingsError::Http(409) => "配置冲突，草稿已保留".into(),
        OpenClawSettingsError::Http(400 | 422) => "服务端拒绝工具配置，草稿已保留".into(),
        _ => "工具配置请求失败，请检查连接后重试".into(),
    }
}
#[derive(Clone, PartialEq)]
pub enum Profile {
    Unset,
    Minimal,
    Coding,
    Messaging,
    Full,
    Unknown(String),
}
impl Profile {
    fn from_value(value: &Option<String>) -> Self {
        match value.as_deref() {
            None => Self::Unset,
            Some("minimal") => Self::Minimal,
            Some("coding") => Self::Coding,
            Some("messaging") => Self::Messaging,
            Some("full") => Self::Full,
            Some(value) => Self::Unknown(value.into()),
        }
    }
}
impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unset => f.write_str("未设置"),
            Self::Minimal => f.write_str("minimal"),
            Self::Coding => f.write_str("coding"),
            Self::Messaging => f.write_str("messaging"),
            Self::Full => f.write_str("full"),
            Self::Unknown(value) => write!(f, "未知值：{value}"),
        }
    }
}
fn candidate(draft: &OpenClawToolsConfig) -> OpenClawToolsConfig {
    let mut value = draft.clone();
    value.allow.retain(|item| !item.trim().is_empty());
    value.deny.retain(|item| !item.trim().is_empty());
    value
}
pub(crate) fn entry() -> Element<'static, Message> {
    button(text("工具设置"))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open))
        .into()
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let close = message(Event::Close);
    let mut body = column![text("配置 OpenClaw 工具范围；留空条目保存时忽略。")].spacing(12);
    if state.busy {
        body = body.push(text("正在处理…"));
    }
    if let Some(error) = &state.error {
        body = body.push(text(error));
    }
    for warning in &state.warnings {
        body = body.push(text(&warning.message));
    }
    if state.loaded {
        let selected = Profile::from_value(&state.draft.profile);
        if let Profile::Unknown(value) = &selected {
            body = body.push(text(format!("未知 profile：{value}。选择其他值前将原样保留。")));
        }
        body = body.push(text("工具 profile")).push(
            settings_select(
                [
                    Profile::Unset,
                    Profile::Minimal,
                    Profile::Coding,
                    Profile::Messaging,
                    Profile::Full,
                ],
                Some(selected),
                |profile| message(Event::Profile(profile)),
            )
            .disabled(state.busy),
        );
        for (deny, label, items) in [
            (false, "允许列表 allow", &state.draft.allow),
            (true, "拒绝列表 deny", &state.draft.deny),
        ] {
            body = body.push(text(label));
            for (index, value) in items.iter().enumerate() {
                let mut input =
                    settings_text_input("工具名称或匹配模式", value).width(Length::Fill);
                if !state.busy {
                    input = input.on_input(move |value| message(Event::Edit(deny, index, value)));
                }
                body = body.push(
                    row![
                        input,
                        button(text("删除"))
                            .padding([8, 12])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!state.busy).then(|| message(Event::Remove(deny, index)))
                            )
                    ]
                    .spacing(8),
                );
            }
            body = body.push(
                button(text("添加条目"))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!state.busy).then(|| message(Event::Add(deny)))),
            );
        }
    }
    let footer = row![
        button(text("取消"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
        button(text("重试读取"))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy && !state.loaded).then(|| message(Event::Open))),
        button(text("保存"))
            .padding([8, 12])
            .style(primary_action_btn_style)
            .on_press_maybe((state.loaded && !state.busy).then(|| message(Event::Save)))
    ]
    .spacing(8);
    let header = settings_modal_header(
        text("OpenClaw 工具配置").size(20),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_layout(header, body, Some(footer.into())).width(640).height(520);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.openclaw_tools.invalidate();
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
            && scope.generation == app.apps_catalog.openclaw_tools.generation
    };
    match event {
        Event::Loaded(scope, result) => {
            if !valid(&scope, app) {
                return Task::none();
            }
            let state = &mut app.apps_catalog.openclaw_tools;
            state.busy = false;
            match result {
                Ok(value) => {
                    state.draft = value.unwrap_or_default();
                    state.loaded = true;
                }
                Err(error) => state.error = Some(error),
            }
            return Task::none();
        }
        Event::Saved(scope, result) => {
            if !valid(&scope, app) {
                return Task::none();
            }
            app.apps_catalog.openclaw_tools.busy = false;
            let state = &mut app.apps_catalog.openclaw_tools;
            state.finish_save(result);
            return if state.error.is_some() {
                app.show_error_toast("工具配置保存失败，草稿已保留")
            } else {
                app.show_success_toast("工具配置已保存，请留意配置警告")
            };
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        return app.show_error_toast("请先连接应用服务");
    }
    let state = &mut app.apps_catalog.openclaw_tools;
    if state.busy {
        return Task::none();
    }
    if !matches!(event, Event::Open) && (!state.open || !state.loaded) {
        return Task::none();
    }
    match event {
        Event::Profile(profile) => {
            state.draft.profile = match profile {
                Profile::Unset => None,
                Profile::Unknown(_) => return Task::none(),
                profile => Some(profile.to_string()),
            };
            return Task::none();
        }
        Event::Edit(deny, index, value) => {
            let items = if deny { &mut state.draft.deny } else { &mut state.draft.allow };
            if let Some(item) = items.get_mut(index) {
                *item = value;
            }
            return Task::none();
        }
        Event::Add(deny) => {
            (if deny { &mut state.draft.deny } else { &mut state.draft.allow }).push(String::new());
            return Task::none();
        }
        Event::Remove(deny, index) => {
            let items = if deny { &mut state.draft.deny } else { &mut state.draft.allow };
            if index < items.len() {
                items.remove(index);
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
    if matches!(event, Event::Save) {
        let value = candidate(&state.draft);
        return Task::perform(
            async move {
                let result = client
                    .set_openclaw_settings_tools(&query, &value, &token)
                    .await
                    .map_err(safe_error);
                (scope, result)
            },
            |(scope, result)| message(Event::Saved(scope, result)),
        );
    }
    Task::perform(
        async move {
            let result = client.openclaw_settings_tools(&query, &token).await.map_err(safe_error);
            (scope, result)
        },
        |(scope, result)| message(Event::Loaded(scope, result)),
    )
}

#[cfg(test)]
#[path = "openclaw_tools_tests.rs"]
mod tests;
