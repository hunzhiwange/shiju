//! Authorized production observations; diagnostics never mutate this state.
use crate::app::{App, Message, state::app_providers::list::Scope};
use super::app_providers::list::{self, ListMessage};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps_high_availability::*, id::ProviderId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    scope: Scope,
    query: FailoverQueueQuery,
    generation: u64,
    access: u64,
    provider: ProviderId,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub context: Option<Context>,
    pub serial: u64,
    pub busy: bool,
    pub health: Option<FailoverHealthResponse>,
    pub route: Option<FailoverRouteResponse>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(String),
    Refresh,
    Reset,
    Poll(Context, u64),
    Loaded(Context, u64, Result<(FailoverHealthResponse, FailoverRouteResponse), String>),
}
pub fn message(event: Event) -> Message { list::message(ListMessage::FailoverHealth(event)) }
fn current(app: &App, c: &Context) -> bool {
    app.app_providers.scope.as_ref() == Some(&c.scope)
        && app.app_providers.generation == c.generation
        && app.apps_catalog.context.as_ref() == Some(&c.scope.context)
        && app.apps_catalog.connection_open
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.model_diagnostics.access_generation == c.access
        && app.project_id.as_deref() == Some(c.query.project_id.as_ref())
        && list::current(app, c.scope.app).is_ok_and(|(_, s)| s == c.scope)
}
pub fn visible(app: &App) -> bool {
    app.apps_catalog.failover_health.context.as_ref().is_some_and(|c| current(app, c))
}
impl State {
    pub fn accept(&mut self, context: &Context, serial: u64, health: FailoverHealthResponse, route: FailoverRouteResponse) -> bool {
        let scope = &health.scope;
        if self.context.as_ref() != Some(context) || serial != self.serial
            || health.provider_id != context.provider || route.scope != *scope
            || scope.project.project_id() != &context.query.project_id
            || scope.project.tenant_scope() != &context.scope.context.tenant
            || scope.target_host_id != context.query.target_host_id || scope.app_id != context.query.app_id {
            return false;
        }
        self.health = Some(health);
        self.route = Some(route);
        true
    }
}
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Open(provider) = &event {
        let Some(scope) = app.app_providers.scope.clone() else { return Task::none(); };
        if !app.app_providers.snapshot.as_ref().is_some_and(|s| s.catalog.rows.iter().any(|r| &r.provider.id == provider)) { return Task::none(); }
        let Some(project) = app.project_id.as_deref().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先打开项目"); };
        let context = Context {
            query: FailoverQueueQuery { project_id: project.into(), target_host_id: scope.host.clone(), app_id: scope.app },
            scope, generation: app.app_providers.generation,
            access: app.apps_catalog.model_diagnostics.access_generation,
            provider: provider.clone().into(),
        };
        if !current(app, &context) { return app.show_error_toast("请重新连接应用"); }
        let serial = app.apps_catalog.failover_health.serial.wrapping_add(1);
        app.apps_catalog.failover_health = State { context: Some(context), serial, ..Default::default() };
        return update(app, Event::Refresh);
    }
    let Some(context) = app.apps_catalog.failover_health.context.clone() else { return Task::none(); };
    if !current(app, &context) { return Task::none(); }
    match event {
        Event::Loaded(request, serial, result) => {
            let state = &mut app.apps_catalog.failover_health;
            if request != context || serial != state.serial || !state.busy { return Task::none(); }
            state.busy = false;
            let accepted = match result {
                Ok((health, route)) => state.accept(&request, serial, health, route),
                Err(_) => false,
            };
            if !accepted {
                state.health = None;
                state.route = None;
                state.error = Some("健康与实际路由未知，请刷新并检查连接和授权".into());
                return app.show_error_toast("健康与实际路由未知，请刷新并检查连接和授权");
            }
            state.error = None;
            return Task::perform(async move {
                super::sleep(std::time::Duration::from_secs(3)).await;
                Event::Poll(request, serial)
            }, message);
        }
        Event::Poll(request, serial) => {
            if request != context || serial != app.apps_catalog.failover_health.serial
                || !app.show_system_settings
                || app.system_settings_tab != crate::app::components::system_settings::SystemTab::Apps {
                return Task::none();
            }
            return update(app, Event::Refresh);
        }
        Event::Refresh | Event::Reset => {
            if app.apps_catalog.failover_health.busy { return Task::none(); }
            let client = match crate::app::config::gateway_client() { Ok(c) => c, Err(e) => return app.show_error_toast(e) };
            let token = app.apps_catalog.token.clone();
            let reset = matches!(event, Event::Reset);
            let state = &mut app.apps_catalog.failover_health;
            state.serial = state.serial.wrapping_add(1);
            state.busy = true;
            state.health = None;
            state.route = None;
            let serial = state.serial;
            return Task::perform(async move {
                let result = async {
                    let health = if reset {
                        client.agent_apps_failover_reset_health(&context.query, &context.provider, Some(&token)).await
                    } else {
                        client.agent_apps_failover_health(&context.query, &context.provider, Some(&token)).await
                    }.map_err(|_| "健康请求失败".to_string())?;
                    let route = client.agent_apps_failover_route(&context.query, Some(&token)).await.map_err(|_| "路由请求失败".to_string())?;
                    Ok((health, route))
                }.await;
                Event::Loaded(context, serial, result)
            }, message);
        }
        Event::Open(_) => {}
    }
    Task::none()
}

pub(crate) fn selected(app: &App, provider: &str) -> bool {
    app.apps_catalog.failover_health.context.as_ref().is_some_and(|c| c.provider.as_ref() == provider)
}

#[cfg(test)]
#[path = "../components/agent_apps_high_availability/health_tests.rs"]
mod tests;
