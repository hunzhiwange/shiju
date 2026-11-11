use super::list::{self, ListMessage};
use crate::app::state::app_providers::{desktop::Form, list::Scope};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{claude_desktop::*, desktop_mapping::*},
    },
};

fn error(value: AgentAppsReadError) -> String {
    match value {
        AgentAppsReadError::Http { status: 400 | 422, .. } => {
            "兼容校验未通过：请检查模型路由、协议、地址与凭据。草稿已保留。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => "无权访问此主机的 Desktop 配置。",
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "配置或事务已变更，请刷新后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 Desktop 配置。",
        _ => "Desktop 请求失败，请检查网关连接后重试。",
    }
    .into()
}
pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: Form,
    token: String,
) -> Result<String, String> {
    let result = client
        .claude_desktop_mapping_save(
            &form.id,
            &DesktopMappingWrite { target_host_id: scope.host, mapping: form.mapping },
            &token,
        )
        .await
        .map_err(error)?;
    if !result.catalog_saved {
        return Err("映射未保存，请重试".into());
    }
    Ok("映射已保存到目录，未激活；模型映射需要路由运行时，重启 Desktop 不会自动启用路由。".into())
}
pub(crate) async fn import(
    client: GatewayClient,
    scope: Scope,
    token: String,
) -> Result<String, String> {
    let result = client
        .claude_desktop_import_claude(&ClaudeDesktopTarget { target_host_id: scope.host }, &token)
        .await
        .map_err(error)?;
    Ok(format!("已导入 {} 个兼容 Claude 档案，未激活。关闭后刷新列表查看。", result.imported))
}
fn live_notice(result: ClaudeDesktopResult) -> Result<String, String> {
    match result.live {
        ClaudeDesktopLiveResult::Applied => {
            Ok("直连配置已应用，请完全退出并重新启动 Claude Desktop。".into())
        }
        ClaudeDesktopLiveResult::NotAttempted if result.catalog_saved => {
            Ok("直连档案已保存，未应用。应用配置成功后需完全退出并重启 Claude Desktop。".into())
        }
        ClaudeDesktopLiveResult::NotAttempted => Err("未应用配置，请检查当前供应商。".into()),
        ClaudeDesktopLiveResult::RecoveryRequired => {
            Err("事务需要恢复，请刷新后处理；不要重复提交。".into())
        }
        ClaudeDesktopLiveResult::Failed => {
            Err("目录可能已保存，但主机配置应用失败，请刷新检查。".into())
        }
    }
}
#[derive(Clone)]
pub enum Event {
    Open(Option<String>),
    Cancel,
    Id(String),
    Name(String),
    BaseUrl(String),
    Key(String),
    Official(bool),
    Apply(bool),
    Proxy(bool),
    Model(String, String),
    Label(String, String),
    OneM(String, bool),
    Remove(String),
    NewRoute(String),
    AddRoute,
    Defaults,
    SaveMapping,
    SaveDirect,
    Import,
    Activate,
    Loaded {
        scope: Scope,
        generation: u64,
        result: Result<DesktopFormView, String>,
    },
    DefaultsLoaded {
        scope: Scope,
        generation: u64,
        result: Result<Vec<DesktopDefaultRoute>, String>,
    },
    Finished {
        scope: Scope,
        generation: u64,
        result: Result<String, String>,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopEvent([redacted])")
    }
}
pub(crate) fn message(value: Event) -> Message {
    list::message(ListMessage::Desktop(value))
}
fn valid(app: &App, scope: &Scope, generation: u64) -> bool {
    app.app_providers.scope.as_ref() == Some(scope)
        && app.app_providers.generation == generation
        && list::current(app, AppId::ClaudeDesktop).ok().is_some_and(|(_, s)| s == *scope)
        && app.app_providers.desktop.as_ref().is_some_and(|f| f.busy)
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if app.app_providers.desktop.as_ref().is_some_and(|f| f.aggregation.is_some())
        && !matches!(event, Event::Cancel)
        && !crate::app::message::settings::aggregation_desktop::binding_is_current(app)
    {
        app.app_providers.desktop = None;
        return Task::none();
    }
    match event {
        Event::Cancel => {
            if app.app_providers.desktop.as_ref().is_some_and(|f| f.busy) {
                return Task::none();
            }
            app.app_providers.desktop = None;
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
        }
        Event::Open(id) => {
            let Ok((client, scope)) = list::current(app, AppId::ClaudeDesktop) else {
                return app.show_error_toast("无法连接网关");
            };
            if app.app_providers.scope.as_ref() != Some(&scope)
                || app.app_providers.loading
                || app.app_providers.desktop.is_some()
            {
                return Task::none();
            }
            app.app_providers.generation = app.app_providers.generation.wrapping_add(1);
            let generation = app.app_providers.generation;
            let mut form = Form::new(id.clone().unwrap_or_default());
            form.busy = id.is_some();
            app.app_providers.desktop = Some(form);
            if let Some(id) = id {
                let token = app.apps_catalog.token.clone();
                let request = ClaudeDesktopTarget { target_host_id: scope.host.clone() };
                return Task::perform(
                    async move {
                        client.claude_desktop_form_read(&id, &request, &token).await.map_err(error)
                    },
                    move |result| {
                        message(Event::Loaded { scope: scope.clone(), generation, result })
                    },
                );
            }
            return scroll_top();
        }
        Event::Loaded { scope, generation, result } => {
            if !valid(app, &scope, generation) {
                return Task::none();
            }
            match result {
                Ok(view) => app.app_providers.desktop.as_mut().unwrap().load(view),
                Err(e) => {
                    app.app_providers.desktop = None;
                    return app.show_error_toast(e);
                }
            }
            return scroll_top();
        }
        Event::DefaultsLoaded { scope, generation, result } => {
            if !valid(app, &scope, generation) {
                return Task::none();
            }
            let form = app.app_providers.desktop.as_mut().unwrap();
            form.busy = false;
            match result {
                Ok(rows) => {
                    for row in rows {
                        form.mapping.routes.entry(row.route_id.clone()).or_insert(
                            DesktopModelRoute {
                                model: row.route_id,
                                label_override: None,
                                supports_1m: row.supports_1m,
                            },
                        );
                    }
                }
                Err(e) => {
                    form.notice = Some(e.clone());
                    return app.show_error_toast(e);
                }
            }
        }
        Event::Finished { scope, generation, result } => {
            if !valid(app, &scope, generation) {
                return Task::none();
            }
            app.app_providers.desktop.as_mut().unwrap().finish(result.clone());
            return match result {
                Ok(value) => app.show_info_toast(value),
                Err(value) => app.show_error_toast(value),
            };
        }
        Event::Defaults
        | Event::SaveMapping
        | Event::SaveDirect
        | Event::Import
        | Event::Activate => {
            if app.app_providers.desktop.as_ref().is_some_and(|f| f.aggregation.is_some()) {
                return Task::none();
            }
            let Ok((client, scope)) = list::current(app, AppId::ClaudeDesktop) else {
                return app.show_error_toast("无法连接网关");
            };
            if app.app_providers.scope.as_ref() != Some(&scope) {
                return Task::none();
            }
            let generation = app.app_providers.generation;
            let Some(form) = app.app_providers.desktop.as_mut().filter(|f| !f.busy) else {
                return Task::none();
            };
            if matches!(event, Event::SaveMapping | Event::Defaults) && !form.mapping_editable {
                return Task::none();
            }
            if matches!(event, Event::SaveDirect | Event::Activate) && !form.direct_editable {
                return Task::none();
            }
            let token = app.apps_catalog.token.clone();
            form.busy = true;
            let form = form.clone();
            let request = ClaudeDesktopTarget { target_host_id: scope.host.clone() };
            if matches!(event, Event::Defaults) {
                return Task::perform(
                    async move {
                        client.claude_desktop_default_routes(&request, &token).await.map_err(error)
                    },
                    move |result| {
                        message(Event::DefaultsLoaded { scope: scope.clone(), generation, result })
                    },
                );
            }
            let worker_scope = scope.clone();
            return Task::perform(
                async move {
                    match event {
                        Event::Import => import(client, worker_scope, token).await,
                        Event::SaveMapping => save(client, worker_scope, form, token).await,
                        Event::Activate => client
                            .claude_desktop_provider_switch(&form.id, &request, &token)
                            .await
                            .map_err(error)
                            .and_then(live_notice),
                        Event::SaveDirect => {
                            let config = if form.official {
                                ClaudeDesktopConfig::Official {}
                            } else {
                                ClaudeDesktopConfig::ThirdParty {
                                    base_url: form.base_url,
                                    auth_token: form.key,
                                }
                            };
                            let activation = if !form.apply {
                                ClaudeDesktopActivation::CatalogOnly
                            } else if form.create {
                                ClaudeDesktopActivation::First
                            } else {
                                ClaudeDesktopActivation::IfCurrent
                            };
                            client
                                .claude_desktop_provider_save(
                                    &form.id,
                                    &ClaudeDesktopWrite {
                                        target_host_id: worker_scope.host,
                                        profile: ClaudeDesktopProfile { name: form.name, config },
                                        activation,
                                    },
                                    &token,
                                )
                                .await
                                .map_err(error)
                                .and_then(live_notice)
                        }
                        _ => unreachable!(),
                    }
                },
                move |result| message(Event::Finished { scope: scope.clone(), generation, result }),
            );
        }
        event => {
            let Some(form) = app.app_providers.desktop.as_mut().filter(|f| !f.busy) else {
                return Task::none();
            };
            if let Some(session) = form.aggregation.as_mut() {
                if session.running() || !form.mapping_editable {
                    return Task::none();
                }
                session.invalidate();
                session.dirty = true;
            }
            match event {
                Event::Id(v) if form.create => form.id = v,
                Event::NewRoute(v) => form.new_route = v,
                Event::AddRoute if form.mapping_editable && !form.new_route.is_empty() => {
                    form.mapping.routes.entry(form.new_route.clone()).or_insert(
                        DesktopModelRoute {
                            model: form.new_route.clone(),
                            label_override: None,
                            supports_1m: false,
                        },
                    );
                    form.new_route.clear();
                }
                Event::Name(v) => form.name = v,
                Event::BaseUrl(v) => form.base_url = v,
                Event::Key(v) => form.key = v,
                Event::Official(v) => form.official = v,
                Event::Apply(v) => form.apply = v,
                Event::Proxy(v) => {
                    form.mapping.mode =
                        if v { DesktopMappingMode::Proxy } else { DesktopMappingMode::Direct }
                }
                Event::Model(id, v) => {
                    if let Some(row) = form.mapping.routes.get_mut(&id) {
                        row.model = v;
                    }
                }
                Event::Label(id, v) => {
                    if let Some(row) = form.mapping.routes.get_mut(&id) {
                        row.label_override = (!v.is_empty()).then_some(v);
                    }
                }
                Event::OneM(id, v) => {
                    if let Some(row) = form.mapping.routes.get_mut(&id) {
                        row.supports_1m = v;
                    }
                }
                Event::Remove(id) => {
                    form.mapping.routes.remove(&id);
                }
                _ => {}
            }
        }
    }
    Task::none()
}

fn scroll_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        iced::widget::Id::new("desktop-provider-form"),
        iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) },
    )
    .map(|_: ()| Message::None)
}
