use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{endpoints::Form, list::Scope},
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{endpoints::*, request_options::*, validation::ProviderRequestOptions},
    },
};
pub(crate) async fn add(
    client: GatewayClient,
    scope: Scope,
    form: Form,
) -> Result<Vec<AppProviderEndpoint>, String> {
    client
        .app_provider_endpoint_change(
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
pub(crate) async fn save_options(
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
    let options: ProviderRequestOptions =
        serde_json::from_str(&format!("{{{}}}", fields.join(",")))
            .map_err(|_| "请求选项必须是有效 JSON 对象，字段类型或重复头名称无效；草稿已保留。")?;
    client
        .app_provider_request_options_save(
            scope.app,
            &form.id,
            &RequestOptionsWrite { target_host_id: scope.host, options },
            &token,
        )
        .await
        .map_err(options_error)
}
#[derive(Clone)]
pub enum Event {
    Open(String),
    Cancel,
    Refresh,
    Url(String),
    FullUrl(bool),
    Headers(String),
    Overrides(String),
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
pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Endpoints(event))
}
fn valid(app: &App, scope: &Scope, generation: u64) -> bool {
    app.app_providers.generation == generation
        && app.app_providers.scope.as_ref() == Some(scope)
        && list::current(app, scope.app).ok().is_some_and(|(_, s)| s == *scope)
        && app.app_providers.endpoints.as_ref().is_some_and(|f| f.busy)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Cancel => {
            if app.app_providers.endpoints.as_ref().is_some_and(|f| f.busy) {
                return Task::none();
            }
            app.app_providers.endpoints = None;
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            if let Some(speed) = &mut app.app_providers.endpoint_speed {
                speed.generation = app.app_providers.generation;
                return crate::app::message::agent_apps_diagnostics::endpoints::update(
                    app,
                    crate::app::message::agent_apps_diagnostics::endpoints::Event::Reload,
                );
            }
        }
        Event::Open(id) => {
            if app.app_providers.loading
                || app.app_providers.endpoints.is_some()
                || app.app_providers.form.is_some()
                || app.app_providers.editor.is_some()
                || app.app_providers.desktop.is_some()
            {
                return Task::none();
            }
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            app.app_providers.endpoints = Some(Form::new(id));
            return Task::batch([
                update(app, Event::Refresh),
                iced::widget::operation::snap_to(
                    iced::widget::Id::new("cc04-endpoints-scroll"),
                    iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
                )
                .map(|_: ()| Message::None),
            ]);
        }
        Event::Loaded { scope, generation, result } => {
            if !valid(app, &scope, generation) {
                return Task::none();
            }
            let error = result.as_ref().err().cloned();
            app.app_providers.endpoints.as_mut().unwrap().finish(result);
            if let Some(e) = error {
                return app.show_error_toast(e);
            }
            if matches!(scope.app, AppId::Claude | AppId::Codex) {
                let Ok((client, _)) = list::current(app, scope.app) else {
                    return Task::none();
                };
                let form = app.app_providers.endpoints.as_mut().unwrap();
                form.busy = true;
                let id = form.id.clone();
                let token = app.apps_catalog.token.clone();
                let host = scope.host.clone();
                return Task::perform(
                    async move {
                        client
                            .app_provider_request_options_read(
                                scope.app,
                                &id,
                                &RequestOptionsTarget { target_host_id: host },
                                &token,
                            )
                            .await
                            .map_err(options_error)
                    },
                    move |result| {
                        message(Event::OptionsLoaded {
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
            if !valid(app, &scope, generation) {
                return Task::none();
            }
            let form = app.app_providers.endpoints.as_mut().unwrap();
            form.busy = false;
            match result {
                Ok(view) => {
                    form.options = Some(view);
                    if saved {
                        form.headers.clear();
                        form.overrides.clear();
                        form.notice = Some("请求选项已保存；未应用到原生应用或代理。".into());
                        return app.show_info_toast("请求选项已保存，未应用");
                    }
                }
                Err(e) => {
                    form.notice = Some(e.clone());
                    return app.show_error_toast(e);
                }
            }
        }
        Event::Refresh
        | Event::Add
        | Event::Remove(..)
        | Event::MarkUsed(..)
        | Event::SaveOptions => {
            let Some(scope) = app.app_providers.scope.clone() else { return Task::none() };
            let Ok((client, current)) = list::current(app, scope.app) else {
                return app.show_error_toast("无法连接网关");
            };
            if current != scope {
                return Task::none();
            }
            let generation = app.app_providers.generation;
            let Some(form) = app.app_providers.endpoints.as_mut().filter(|f| !f.busy) else {
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
                let token = app.apps_catalog.token.clone();
                return Task::perform(save_options(client, worker, form, token), move |result| {
                    message(Event::OptionsLoaded {
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
                            client
                                .app_provider_endpoint_change(
                                    worker.app,
                                    &form.id,
                                    &AppProviderEndpointChange::Remove { url, kind },
                                )
                                .await
                        }
                        Event::MarkUsed(url, kind) => {
                            client
                                .app_provider_endpoint_change(
                                    worker.app,
                                    &form.id,
                                    &AppProviderEndpointChange::MarkUsed { url, kind },
                                )
                                .await
                        }
                        _ => client.app_provider_endpoints(worker.app, &form.id).await,
                    }
                },
                move |result| message(Event::Loaded { scope: scope.clone(), generation, result }),
            );
        }
        field => {
            let Some(form) = app.app_providers.endpoints.as_mut().filter(|f| !f.busy) else {
                return Task::none();
            };
            match field {
                Event::Url(v) => form.url = v,
                Event::FullUrl(v) => form.full_url = v,
                Event::Headers(v) => form.headers = v,
                Event::Overrides(v) => form.overrides = v,
                _ => {}
            }
        }
    }
    Task::none()
}
