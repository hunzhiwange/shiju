//! Explicit, host-scoped drafts. Reading and cancelling never issue writes.
use super::lifecycle::{self, Context};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Task,
    widget::{button, column, container, row, text},
};
use {
    vw_api_app_identity::AppId,
    vw_api_app_routing::AppRoutingConfig,
    vw_api_app_routing::{RoutingClientHost, RoutingClientHostUpdate, normalize_routing_client_host},
    vw_api_app_routing::RoutingConfig,
    vw_api_transport::id::ProjectId,
};

#[derive(Clone, Debug)]
pub enum Value {
    Listener(RoutingConfig),
    ClientHost(RoutingClientHost),
    App(AppRoutingConfig),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Target {
    #[default]
    Listener,
    ClientHost,
    App(AppId),
}

#[derive(Clone, Debug, Default)]
pub struct State {
    open: bool,
    busy: bool,
    serial: u64,
    context: Option<Context>,
    project: Option<String>,
    target: Target,
    value: Option<Value>,
    fields: [String; 3],
    error: Option<String>,
    logging: bool,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(Option<AppId>),
    OpenClientHost,
    Close,
    Reload,
    Edit(usize, String),
    Logging(bool),
    Save,
    Reply(Context, Option<String>, u64, bool, Result<Value, String>),
}
type Message = lifecycle::Event;
fn message(event: Event) -> Message { lifecycle::Event::Config(event) }
impl State {
    pub(super) fn is_open(&self) -> bool {
        self.open
    }
    pub(super) fn is_busy(&self) -> bool {
        self.busy
    }

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
                if (1..1024).contains(&config.listen_port) {
                    return Err("监听端口须为 0 或 1024–65535".into());
                }
                config.logging_enabled = self.logging;
                Ok(Value::Listener(config))
            }
            Value::ClientHost(mut config) => {
                config.host = normalize_routing_client_host(Some(&self.fields[0]))
                    .map_err(|_| "请输入不带端口的 IP 地址或主机名")?;
                Ok(Value::ClientHost(config))
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
            Value::ClientHost(c) => {
                [c.host.clone().unwrap_or_default(), String::new(), String::new()]
            }
            Value::App(c) => [
                c.first_byte_timeout_secs.to_string(),
                c.idle_timeout_secs.to_string(),
                c.non_streaming_timeout_secs.to_string(),
            ],
        };
        if let Value::Listener(config) = &value {
            self.logging = config.logging_enabled;
        }
        self.value = Some(value);
    }
}
pub fn update(state: &mut State, context: Option<Context>, project: Option<String>, token: &str, client: Option<vw_gateway_client::GatewayClient>, notice: &mut Option<crate::Notice>, event: Event) -> Task<Message> {
    update_settings(state, context, project, token, client, notice, event, false)
}

pub(super) fn update_inline(
    state: &mut State,
    context: Option<Context>,
    project: Option<String>,
    token: &str,
    client: Option<vw_gateway_client::GatewayClient>,
    notice: &mut Option<crate::Notice>,
    event: Event,
) -> Task<Message> {
    update_settings(state, context, project, token, client, notice, event, true)
}

fn update_settings(
    state: &mut State,
    context: Option<Context>,
    project: Option<String>,
    token: &str,
    client: Option<vw_gateway_client::GatewayClient>,
    notice: &mut Option<crate::Notice>,
    event: Event,
    inline: bool,
) -> Task<Message> {
    let message: fn(Event) -> Message =
        if inline { lifecycle::Event::Listener } else { lifecycle::Event::Config };
    let project = Some(vw_api_transport::local_settings_project(project.as_deref()).to_owned());
    let Some(context) = context else {
        return Task::none();
    };
    match event {
        Event::Close => {
            // Saving cannot be cancelled: its outcome must be observed before closing.
            if !state.busy || state.value.is_none() {
                *state = State { serial: state.serial.wrapping_add(1), ..Default::default() };
            }
            return Task::none();
        }
        Event::Open(_) | Event::OpenClientHost => {
            if state.busy {
                return Task::none();
            }
            *state = State {
                open: true,
                serial: state.serial.wrapping_add(1),
                context: Some(context.clone()),
                project: project.clone(),
                target: match event {
                    Event::Open(Some(id)) => Target::App(id),
                    Event::Open(None) => Target::Listener,
                    _ => Target::ClientHost,
                },
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
                    if saving && !inline {
                        state.open = false;
                    }
                    if saving && state.target == Target::Listener {
                        return Task::done(lifecycle::Event::Run(lifecycle::Operation::Refresh));
                    }
                }
                Err(error) => {
                    state.error = Some(error.clone());
                    if !inline {
                        return crate::error(notice, error);
                    }
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
        Event::Logging(enabled) => {
            if state.open
                && !state.busy
                && state.context.as_ref() == Some(&context)
                && state.project == project
                && matches!(state.value, Some(Value::Listener(_))) {
                    state.logging = enabled;
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
    let Some(client) = client else {
        return crate::error(notice, "无法连接网关");
    };
    let client = gateway_app_routing::GatewayAppRoutingClient::new(client.transport().clone());
    state.busy = true;
    state.error = None;
    state.serial = state.serial.wrapping_add(1);
    let serial = state.serial;
    let target = state.target;
    let token = token.to_owned();
    Task::perform(
        async move {
            #[cfg(not(target_arch = "wasm32"))]
            let request = async {
                match (target, draft) {
                    (Target::Listener, None) => {
                        client.app_routing_config(&context.host, &token).await.map(Value::Listener)
                    }
                    (Target::Listener, Some(Value::Listener(c))) => client.app_routing_set_config(&context.host, &token, &c)
                        .await
                        .map(Value::Listener),
                    (Target::ClientHost, None) => client.app_routing_client_host(&context.host, &token)
                        .await.map(Value::ClientHost),
                    (Target::ClientHost, Some(Value::ClientHost(c))) => client.app_routing_set_client_host(
                        &context.host, &token, &RoutingClientHostUpdate { host: c.host },
                    ).await.map(Value::ClientHost),
                    (Target::App(id), value) => {
                        let project: ProjectId = project.clone().unwrap_or_default().into();
                        match value {
                            None => client.app_routing_app_config(&context.host, &project, id, &token)
                                .await
                                .map(Value::App),
                            Some(Value::App(c)) => client.app_routing_set_app_config(&context.host, &project, id, &token, &c)
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
            let result = Err("此平台不支持路由设置".into());
            Event::Reply(context, project, serial, saving, result)
        },
        message,
    )
}
pub(super) fn listener_fields(i18n: desktop_i18n::I18n,
    state: &State,
    service: Option<vw_api_app_routing::RoutingState>,
) -> Element<'_, Message> {
    use iced::Length;

    let Some(Value::Listener(saved)) = &state.value else {
        return inline_status(i18n, state);
    };
    let ready = !state.busy && service.is_some();
    let bind_changed = state.fields[0] != saved.listen_address.to_string()
        || state.fields[1] != saved.listen_port.to_string();
    let changed = bind_changed || state.logging != saved.logging_enabled;
    let controls = move || {
        let field = |index: usize, label: &'static str, hint: &'static str| {
            column![
                settings_field_label(label),
                settings_text_input("", &state.fields[index]).on_input_maybe(ready.then_some(
                    move |value| { lifecycle::Event::Listener(Event::Edit(index, value)) }
                )),
                text(hint).size(12).style(settings_muted_text_style),
            ]
            .spacing(5)
            .width(Length::Fill)
        };
        let save = button(
            text(if bind_changed && service == Some(vw_api_app_routing::RoutingState::Running) {
                i18n.__("保存并重启")
            } else {
                i18n.__("保存")
            })
            .size(SETTINGS_CONTROL_TEXT_SIZE),
        )
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe((ready && changed).then_some(lifecycle::Event::Listener(Event::Save)));
        column![
            row![field(0, i18n.__("监听地址"), i18n.__("推荐 127.0.0.1")), field(1, i18n.__("监听端口"), i18n.__("0 或 1024–65535"))]
                .spacing(10),
            container(save).align_right(Length::Fill),
        ]
        .spacing(8)
    };
    let mut fields =
        column![container(settings_responsive_row(i18n.__("监听设置"), "", controls)).padding([12, 0]),];
    if state.error.is_some() {
        fields = fields.push(inline_status(i18n, state));
    }
    fields.into()
}

pub(super) fn logging_field(i18n: desktop_i18n::I18n, state: &State, enabled: bool) -> Element<'_, Message> {
    settings_row(
        i18n.__("记录路由请求"),
        i18n.__("通过监听设置的「保存」提交"),
        44.0,
        settings_toggle(state.logging).on_toggle_maybe(
            (enabled && state.value.is_some() && !state.busy)
                .then_some(|enabled| lifecycle::Event::Listener(Event::Logging(enabled))),
        ),
    )
}

pub(super) fn client_host_field(i18n: desktop_i18n::I18n, state: &State) -> Element<'_, Message> {
    let host = match &state.value {
        Some(Value::Listener(config)) => {
            vw_api_app_routing::routing_client_host_from_bind(config.listen_address)
        }
        _ => String::new(),
    };
    settings_responsive_row(i18n.__("连接地址"), i18n.__("由保存的监听地址推导"), move || {
        settings_text_input(i18n.__("等待监听设置"), &host)
    })
}

fn inline_status(i18n: desktop_i18n::I18n, state: &State) -> Element<'_, Message> {
    let mut content = column![].spacing(8);
    if let Some(error) = &state.error {
        content = content.push(settings_error_banner(i18n.__(error).to_owned()));
        content = content.push(
            button(text(i18n.__("重新读取")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(lifecycle::Event::Listener(Event::Reload)),
        );
    } else {
        content = content.push(text(i18n.__("正在读取…")).size(13).style(settings_muted_text_style));
    }
    container(content).padding([12, 0]).into()
}

pub fn timeout_button(i18n: desktop_i18n::I18n, id: AppId) -> Element<'static, Message> {
    button(text(i18n.__("应用路由超时")))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open(Some(id))))
        .into()
}
pub fn overlay<'a>(i18n: desktop_i18n::I18n,
    routing: &'a lifecycle::State,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let base = super::outbound::overlay(i18n, routing, base);
    let state = &routing.config;
    if !state.open {
        return base;
    }
    let (title, description, labels): (_, _, &[_]) = match state.target {
        Target::Listener => (
            i18n.__("路由监听设置"),
            i18n.__("修改监听设置可能重新绑定路由服务。"),
            &[i18n.__("监听 IP 地址"), i18n.__("监听端口（0 表示自动分配）")],
        ),
        Target::ClientHost => (
            i18n.__("路由连接地址"),
            i18n.__("客户端连接路由时使用的地址。留空时由监听地址推导。"),
            &[i18n.__("IP 地址或主机名")],
        ),
        Target::App(_) => (
            i18n.__("应用路由超时"),
            i18n.__("设置连接和请求的超时。"),
            &[i18n.__("首字节超时（秒）"), i18n.__("流式空闲超时（秒，0 表示禁用）"), i18n.__("非流式超时（秒）")],
        ),
    };
    let header =
        settings_modal_header(text(title).size(20), settings_close_button(message(Event::Close)));
    let mut body = column![text(description)].spacing(12);
    if state.value.is_some() {
        let placeholder = match &state.value {
            Some(Value::ClientHost(config)) => config.effective_host.as_str(),
            _ => "",
        };
        for (index, label) in labels.iter().enumerate() {
            let mut input = settings_text_input(placeholder, &state.fields[index]);
            if !state.busy {
                input = input.on_input(move |v| message(Event::Edit(index, v)));
            }
            body = body.push(column![text(*label), input].spacing(6));
        }
    }
    if matches!(state.value, Some(Value::Listener(_))) {
        body = body.push(settings_row(
            i18n.__("记录路由请求"),
            i18n.__("将已授权请求写入路由日志。"),
            48.0,
            settings_toggle(state.logging)
                .on_toggle_maybe((!state.busy).then_some(|value| message(Event::Logging(value)))),
        ));
    }
    if let Some(error) = &state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    let actions = row![
        button(text(i18n.__("取消")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(message(Event::Close))),
        button(text(i18n.__("重新读取")))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(message(Event::Reload))),
        button(text(if state.busy { i18n.__("请求中") } else { i18n.__("保存") }))
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
