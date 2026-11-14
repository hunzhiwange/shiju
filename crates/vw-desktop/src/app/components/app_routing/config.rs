//! Explicit, host-scoped drafts. Reading and cancelling never issue writes.
use super::lifecycle::{self, Context};
use crate::app::{App, Message, components::system_settings_common::*, state::AppsCatalogState};
use iced::{
    Element, Task,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_routing::{AppRoutingConfig, RoutingConfig},
    id::ProjectId,
};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Listener(RoutingConfig),
    App(AppRoutingConfig),
}
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    open: bool,
    busy: bool,
    serial: u64,
    context: Option<Context>,
    project: Option<String>,
    target: Option<AppId>,
    value: Option<Value>,
    fields: [String; 3],
    error: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open(Option<AppId>),
    Close,
    Reload,
    Edit(usize, String),
    Save,
    Reply(Context, Option<String>, u64, bool, Result<Value, String>),
}
fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::Routing(lifecycle::Event::Config(event)),
    ))
}
impl State {
    pub(super) fn next_serial(&self) -> u64 {
        self.serial.wrapping_add(1)
    }
    pub(super) fn with_serial(serial: u64) -> Self {
        Self { serial, ..Default::default() }
    }
    fn draft(&self) -> Result<Value, String> {
        let number = |i: usize| {
            self.fields[i]
                .trim()
                .parse::<u16>()
                .map_err(|_| "请输入 0–65535 之间的整数".to_string())
        };
        match self.value.clone().ok_or("请先读取配置")? {
            Value::Listener(mut config) => {
                config.listen_address =
                    self.fields[0].trim().parse().map_err(|_| "请输入有效 IP 地址")?;
                config.listen_port = number(1)?;
                Ok(Value::Listener(config))
            }
            Value::App(_) => {
                let config = AppRoutingConfig {
                    first_byte_timeout_secs: number(0)?,
                    idle_timeout_secs: number(1)?,
                    non_streaming_timeout_secs: number(2)?,
                };
                if config.first_byte_timeout_secs == 0 || config.non_streaming_timeout_secs == 0 {
                    return Err("首字节与非流式超时必须大于零".into());
                }
                Ok(Value::App(config))
            }
        }
    }
    fn load(&mut self, value: Value) {
        self.fields = match &value {
            Value::Listener(c) => {
                [c.listen_address.to_string(), c.listen_port.to_string(), String::new()]
            }
            Value::App(c) => [
                c.first_byte_timeout_secs.to_string(),
                c.idle_timeout_secs.to_string(),
                c.non_streaming_timeout_secs.to_string(),
            ],
        };
        self.value = Some(value);
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(context) = lifecycle::current(app) else {
        return Task::none();
    };
    let project = app.project_id.clone();
    let state = &mut app.apps_catalog.routing.config;
    match event {
        Event::Close => {
            // Saving cannot be cancelled: its outcome must be observed before closing.
            if !state.busy || state.value.is_none() {
                *state = State { serial: state.serial.wrapping_add(1), ..Default::default() };
            }
            return Task::none();
        }
        Event::Open(target) => {
            if state.busy {
                return Task::none();
            }
            if target.is_some() && project.as_deref().is_none_or(str::is_empty) {
                return app.show_error_toast("请先选择项目");
            }
            *state = State {
                open: true,
                serial: state.serial.wrapping_add(1),
                context: Some(context.clone()),
                project: project.clone(),
                target,
                ..Default::default()
            };
        }
        Event::Reply(request, request_project, serial, saving, result) => {
            if request != context
                || request_project != project
                || state.context.as_ref() != Some(&request)
                || state.serial != serial
                || !state.open
                || !state.busy
            {
                return Task::none();
            }
            state.busy = false;
            match result {
                Ok(value) => {
                    state.load(value);
                    state.error = None;
                    if saving {
                        state.open = false;
                    }
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    return app.show_error_toast(error);
                }
            }
            return Task::none();
        }
        Event::Edit(index, value) => {
            if state.open
                && !state.busy
                && state.context.as_ref() == Some(&context)
                && state.project == project
                && index < state.fields.len()
            {
                state.fields[index] = value;
                state.error = None;
            }
            return Task::none();
        }
        Event::Reload | Event::Save => {}
    }
    if !state.open
        || state.busy
        || state.context.as_ref() != Some(&context)
        || state.project != project
    {
        return Task::none();
    }
    let saving = matches!(event, Event::Save);
    let draft = if saving {
        match state.draft() {
            Ok(value) => Some(value),
            Err(error) => {
                state.error = Some(error);
                return Task::none();
            }
        }
    } else {
        None
    };
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("无法连接网关");
    };
    state.busy = true;
    state.error = None;
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    let target = state.target;
    let token = app.apps_catalog.token.clone();
    Task::perform(
        async move {
            let request = async {
                match (target, draft) {
                    (None, None) => {
                        client.app_routing_config(&context.host, &token).await.map(Value::Listener)
                    }
                    (None, Some(Value::Listener(c))) => client
                        .app_routing_set_config(&context.host, &token, &c)
                        .await
                        .map(Value::Listener),
                    (Some(id), value) => {
                        let project: ProjectId = project.clone().unwrap_or_default().into();
                        match value {
                            None => client
                                .app_routing_app_config(&context.host, &project, id, &token)
                                .await
                                .map(Value::App),
                            Some(Value::App(c)) => client
                                .app_routing_set_app_config(&context.host, &project, id, &token, &c)
                                .await
                                .map(Value::App),
                            _ => return Err("配置类型不匹配".into()),
                        }
                    }
                    _ => return Err("配置类型不匹配".into()),
                }
                .map_err(lifecycle::error_message)
            };
            #[cfg(not(target_arch = "wasm32"))]
            let result = tokio::time::timeout(std::time::Duration::from_secs(15), request)
                .await
                .unwrap_or_else(|_| Err("请求超时；保存结果未知，请重新读取确认后重试".into()));
            #[cfg(target_arch = "wasm32")]
            let result = {
                let _ = request;
                Err("此平台不支持路由设置".into())
            };
            Event::Reply(context, project, serial, saving, result)
        },
        message,
    )
}
pub(crate) fn timeout_button(id: AppId) -> Element<'static, Message> {
    button(text("应用路由超时"))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open(Some(id))))
        .into()
}
pub(crate) fn overlay<'a>(
    catalog: &'a AppsCatalogState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &catalog.routing.config;
    if !state.open {
        return base;
    }
    let title = if state.target.is_some() { "应用路由超时" } else { "路由监听设置" };
    let header =
        settings_modal_header(text(title).size(20), settings_close_button(message(Event::Close)));
    let labels: &[&str] = if state.target.is_some() {
        &["首字节超时（秒）", "流式空闲超时（秒，0 表示禁用）", "非流式超时（秒）"]
    } else {
        &["监听 IP 地址", "监听端口（0 表示自动分配）"]
    };
    let mut body =
        column![text("配置由执行主机保存；监听变更可能需要重启服务后生效。")].spacing(12);
    if state.value.is_some() {
        for (index, label) in labels.iter().enumerate() {
            let mut input = settings_text_input("", &state.fields[index]);
            if !state.busy {
                input = input.on_input(move |v| message(Event::Edit(index, v)));
            }
            body = body.push(column![text(*label), input].spacing(6));
        }
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let actions = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(message(Event::Close))),
        button(text("重新读取"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(message(Event::Reload))),
        button(text(if state.busy { "请求中" } else { "保存" }))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!state.busy && state.value.is_some()).then_some(message(Event::Save))),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(620).height(480);
    settings_modal_overlay(Some(base), message(Event::Close), container(card).padding(12))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
