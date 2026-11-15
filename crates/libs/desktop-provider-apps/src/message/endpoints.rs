use crate::Scope;
use vw_api_app_provider_edit::{endpoints::*,request_options::*};
#[derive(Clone)]
pub enum Event {
    Open(String),
    Cancel,
    Refresh,
    Url(String),
    FullUrl(bool),
    Headers(String),
    Overrides(String),
    UserAgentEnabled(bool),
    UserAgent(String),
    Add,
    Remove(String, AppProviderEndpointKind),
    MarkUsed(String, AppProviderEndpointKind),
    SaveOptions,
    Loaded {
        scope: Scope,
        generation: u64,
        result: Result<Vec<AppProviderEndpoint>, String>,
    },
    OptionsLoaded {
        scope: Scope,
        generation: u64,
        saved: bool,
        result: Result<RequestOptionsView, String>,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EndpointEvent([redacted])")
    }
}

use super::provider_update::{State,Effect as Message};
use crate::{state::endpoints::Form};
use iced::Task;
use gateway_http::AgentAppsReadError;
use vw_gateway_client::GatewayClient;
use vw_api_types::agent_apps::AppId;


use vw_api_app_provider_base::validation::ProviderRequestOptions;
pub async fn add(
    client: GatewayClient,
    scope: Scope,
    form: Form,
) -> Result<Vec<AppProviderEndpoint>, String> {
    gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_endpoint_change(
            scope.app,
            &form.id,
            &AppProviderEndpointChange::Add {
                url: form.url,
                kind: if form.full_url {
                    AppProviderEndpointKind::FullUrl
                } else {
                    AppProviderEndpointKind::BaseUrl
                },
            },
        )
        .await
}
fn options_error(e: AgentAppsReadError) -> String {
    match e {
        AgentAppsReadError::Http { status, .. } => {
            format!("请求选项未完成（HTTP {status}），草稿已保留。请检查格式、权限或刷新后重试。")
        }
        _ => "请求选项响应无效或网关不可达，草稿已保留。".into(),
    }
}
pub async fn save_options(
    client: GatewayClient,
    scope: Scope,
    form: Form,
    token: String,
) -> Result<RequestOptionsView, String> {
    // Deserialize only the wire shape. All header/protocol policy stays on the execution host.
    for draft in [&form.headers, &form.overrides] {
        if !draft.trim().is_empty()
            && !serde_json::from_str::<serde_json::Value>(draft)
                .is_ok_and(|value| value.is_object())
        {
            return Err("请求选项必须是单个 JSON 对象，草稿已保留。".into());
        }
    }
    let mut fields = Vec::new();
    if !form.headers.trim().is_empty() {
        fields.push(format!("\"requestHeaders\":{}", form.headers));
    }
    if !form.overrides.trim().is_empty() {
        fields.push(format!("\"localProxyRequestOverrides\":{}", form.overrides));
    }
    let mut options: ProviderRequestOptions =
        serde_json::from_str(&format!("{{{}}}", fields.join(",")))
            .map_err(|_| "请求选项必须是有效 JSON 对象，字段类型或重复头名称无效；草稿已保留。")?;
    if let Some(value) = form.user_agent {
        // This is wire assembly only. Header validity and protected names remain host policy.
        // Overrides are replaced as a group by the API, so retain the supplied body and headers.
        let overrides = options.local_proxy_request_overrides.get_or_insert_with(Default::default);
        let headers = overrides.headers.get_or_insert_with(Default::default);
        headers.retain(|name, _| !name.trim().eq_ignore_ascii_case("user-agent"));
        headers.insert("user-agent".into(), value);
    }
    gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_request_options_save(
            scope.app,
            &form.id,
            &RequestOptionsWrite { target_host_id: scope.host, options },
            &token,
        )
        .await
        .map_err(options_error)
}


fn effect_message(event: Event) -> Message {
    Message::Endpoints(event)
}
fn valid(state: &State, current: &Option<(GatewayClient, Scope)>, scope: &Scope, generation: u64) -> bool {
    state.generation == generation
        && state.scope.as_ref() == Some(scope)
        && current.clone().is_some_and(|(_, s)| s == *scope)
        && state.endpoints.as_ref().is_some_and(|f| f.busy)
}
pub fn update(state: &mut State, current: Option<(GatewayClient, Scope)>, token: &str, event: Event) -> Task<Message> {
    match event {
        Event::Cancel => {
            if state.endpoints.as_ref().is_some_and(|f| f.busy) {
                return Task::none();
            }
            state.endpoints = None;
            state.generation = state.generation.wrapping_add(1);
            return Task::none();
        }
        Event::Open(id) => {
            if state.loading
                || state.endpoints.is_some()
                || state.form.is_some()
                || state.editor.is_some()
                || state.desktop.is_some()
            {
                return Task::none();
            }
            state.generation = state.generation.wrapping_add(1);
            state.endpoints = Some(Form::new(id));
            return Task::batch([
                update(state, current.clone(), token, Event::Refresh),
                iced::widget::operation::snap_to(
                    iced::widget::Id::new("cc04-endpoints-scroll"),
                    iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
                )
                .map(|_: ()| Message::None),
            ]);
        }
        Event::Loaded { scope, generation, result } => {
            if !valid(state, &current, &scope, generation) {
                return Task::none();
            }
            let error = result.as_ref().err().cloned();
            state.endpoints.as_mut().unwrap().finish(result);
            if let Some(e) = error {
                return super::provider_update::error(e);
            }
            if matches!(scope.app, AppId::Claude | AppId::Codex) {
                let Ok((client, _)) = current.clone().ok_or("无法连接网关".to_string()) else {
                    return Task::none();
                };
                let form = state.endpoints.as_mut().unwrap();
                form.busy = true;
                let id = form.id.clone();
                let token = token.to_owned();
                let host = scope.host.clone();
                return Task::perform(
                    async move {
                        gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_request_options_read(
                                scope.app,
                                &id,
                                &RequestOptionsTarget { target_host_id: host },
                                &token,
                            )
                            .await
                            .map_err(options_error)
                    },
                    move |result| {
                        effect_message(Event::OptionsLoaded {
                            scope: scope.clone(),
                            generation,
                            saved: false,
                            result,
                        })
                    },
                );
            }
        }
        Event::OptionsLoaded { scope, generation, saved, result } => {
            if !valid(state, &current, &scope, generation) {
                return Task::none();
            }
            let form = state.endpoints.as_mut().unwrap();
            form.busy = false;
            match result {
                Ok(view) => {
                    form.options = Some(view);
                    if saved {
                        form.headers.clear();
                        form.overrides.clear();
                        form.user_agent = None;
                        form.notice = Some("请求选项已保存；未应用到原生应用或代理。".into());
                        return super::provider_update::info("请求选项已保存，未应用");
                    }
                }
                Err(e) => {
                    form.notice = Some(e.clone());
                    return super::provider_update::error(e);
                }
            }
        }
        Event::Refresh
        | Event::Add
        | Event::Remove(..)
        | Event::MarkUsed(..)
        | Event::SaveOptions => {
            let Some(scope) = state.scope.clone() else { return Task::none() };
            let Ok((client, current)) = current.clone().ok_or("无法连接网关".to_string()) else {
                return super::provider_update::error("无法连接网关");
            };
            if current != scope {
                return Task::none();
            }
            let generation = state.generation;
            let Some(form) = state.endpoints.as_mut().filter(|f| !f.busy) else {
                return Task::none();
            };
            if matches!(event, Event::SaveOptions) && form.options.is_none() {
                return Task::none();
            }
            form.busy = true;
            form.notice = None;
            let form = form.clone();
            let worker = scope.clone();
            if matches!(event, Event::SaveOptions) {
                let token = token.to_owned();
                return Task::perform(save_options(client, worker, form, token), move |result| {
                    effect_message(Event::OptionsLoaded {
                        scope: scope.clone(),
                        generation,
                        saved: true,
                        result,
                    })
                });
            }
            return Task::perform(
                async move {
                    match event {
                        Event::Add => add(client, worker, form).await,
                        Event::Remove(url, kind) => {
                            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_endpoint_change(
                                    worker.app,
                                    &form.id,
                                    &AppProviderEndpointChange::Remove { url, kind },
                                )
                                .await
                        }
                        Event::MarkUsed(url, kind) => {
                            gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_endpoint_change(
                                    worker.app,
                                    &form.id,
                                    &AppProviderEndpointChange::MarkUsed { url, kind },
                                )
                                .await
                        }
                        _ => gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_endpoints(worker.app, &form.id).await,
                    }
                },
                move |result| effect_message(Event::Loaded { scope: scope.clone(), generation, result }),
            );
        }
        field => {
            let Some(form) = state.endpoints.as_mut().filter(|f| !f.busy) else {
                return Task::none();
            };
            match field {
                Event::Url(v) => form.url = v,
                Event::FullUrl(v) => form.full_url = v,
                Event::Headers(v) => form.headers = v,
                Event::Overrides(v) => form.overrides = v,
                Event::UserAgentEnabled(enabled) => {
                    form.user_agent = enabled.then(String::new);
                }
                Event::UserAgent(v) => form.user_agent = Some(v),
                _ => {}
            }
        }
    }
    Task::none()
}

pub fn message(value:Event)->crate::Message {crate::Message::Endpoints(value)}
