//! Desktop wiring for usage cards and the provider settings editor.
use iced::{Task, Element};
use gateway_endpoint::GatewayEndpoint;
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
use vw_api_app_auth::{AuthService, AuthCredentialState};
use std::collections::HashSet;
pub use desktop_usage::apps_usage_cards::{Event, Target, Scope, Failure, binding_app};
use desktop_usage::apps_usage_cards::{QueryContext, read};
use crate::app::{App, Message, message::app_providers::list::{ListMessage, message}, state::agent_apps_auth::Scope as AuthScope};
use super::system_settings::SystemTab;
#[path = "apps_usage_cards/batch.rs"] mod batch;
#[derive(Clone, Debug, Default)]
pub(crate) struct Cards {
    data: desktop_usage::apps_usage_cards::Cards,
    pub(super) settings: super::apps_usage_settings::Editor,
}
impl std::ops::Deref for Cards {
    type Target = desktop_usage::apps_usage_cards::Cards;
    fn deref(&self) -> &Self::Target { &self.data }
}
impl std::ops::DerefMut for Cards {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.data }
}
impl Cards {
    pub(crate) fn clear(&mut self) { self.data.clear(); self.settings.clear(); }
    fn select_scope(&mut self, scope: &Scope) {
        if self.data.scope.as_ref() != Some(scope) { self.clear(); self.data.scope = Some(scope.clone()); }
    }
}
fn map_view(event: desktop_usage::apps_usage_cards::ViewMessage) -> Message {
    match event {
        desktop_usage::apps_usage_cards::ViewMessage::Event(event) => message_event(event),
        desktop_usage::apps_usage_cards::ViewMessage::OpenSettings(app, id) => super::apps_usage_settings::send(super::apps_usage_settings::Event::Open(app, id)),
        desktop_usage::apps_usage_cards::ViewMessage::OpenUrl(url) => Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url)),
    }
}
pub(crate) fn provider_view<'a>(scope: Option<&Scope>, cards: &'a Cards, app: AppId, id: &str) -> Element<'a, Message> {
    desktop_usage::apps_usage_cards::provider_view(scope, &cards.data, app, id).map(map_view)
}
pub(crate) fn account_view<'a>(scope: Option<&Scope>, cards: &'a Cards, service: AuthService, id: &str) -> Element<'a, Message> {
    desktop_usage::apps_usage_cards::account_view(scope, &cards.data, service, id).map(map_view)
}
pub(super) fn scope(app: &App, endpoint: &GatewayEndpoint, account: bool) -> Option<Scope> {
    let endpoint = AuthScope::from(endpoint);
    let project = app.project_id.as_deref().filter(|id| !id.trim().is_empty())?.into();
    let host = if account {
        if app.agent_apps_auth.scope.as_ref() != Some(&endpoint) {
            return None;
        }
        app.agent_apps_auth.connection.as_ref()?.host.trim().to_owned()
    } else {
        let origin = app.app_providers.scope.as_ref()?;
        if origin.context.gateway != endpoint.gateway
            || origin.context.tenant != endpoint.tenant
            || origin.context.project != app.project_path
            || origin.host != app.apps_catalog.host
        {
            return None;
        }
        origin.host.clone()
    };
    Some(Scope {
        endpoint,
        project,
        host,
        provider_generation: if account { 0 } else { app.app_providers.generation },
        catalog_generation: if account { 0 } else { app.apps_catalog.generation },
        auth_generation: app.agent_apps_auth.rows_generation(),
    })
}

fn linked_scope(
    app: &App,
    endpoint: &GatewayEndpoint,
    current: &Scope,
    account: bool,
) -> Option<Scope> {
    let connection = app.agent_apps_auth.connection.as_ref()?;
    if !endpoint.is_loopback()
        || !connection.valid()
        || connection.token.trim() != app.apps_catalog.token.trim()
    {
        return None;
    }
    scope(app, endpoint, !account).filter(|other| current.same_context(other))
}

/// Reopening the provider list resolves current bindings before displaying linked results.
pub(crate) fn read_linked_providers(app: &mut App, app_id: AppId) -> Task<Message> {
    if !app.show_system_settings || app.system_settings_tab != SystemTab::Apps {
        return Task::none();
    }
    let endpoint = crate::app::gateway_client_endpoint();
    let Some(scope) = scope(app, &endpoint, false) else { return Task::none() };
    let auth_cards = &app.agent_apps_auth.usage;
    if linked_scope(app, &endpoint, &scope, false).as_ref() != auth_cards.scope.as_ref()
        || auth_cards.scope.is_none()
    {
        return Task::none();
    }
    let providers: HashSet<_> = auth_cards
        .entries
        .values()
        .filter_map(|entry| entry.result.as_ref())
        .chain(auth_cards.batch_result.as_ref())
        .filter_map(|result| result.as_ref().ok())
        .flatten()
        .filter(|item| item.selection.app_id == app_id && item.snapshot.is_ok())
        .map(|item| item.provider().to_owned())
        .collect();
    let cards = &app.app_providers.usage;
    let mut reads = Vec::new();
    for id in providers {
        let target = Target::Provider(app_id, id);
        if cards.scope.as_ref() != Some(&scope) || !cards.entries.contains_key(&target) {
            reads.push(target);
        }
    }
    Task::batch(
        reads.into_iter().map(|target| update_at(app, Event::Read(target), endpoint.clone())),
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    update_at(app, event, crate::app::gateway_client_endpoint())
}

fn update_at(app: &mut App, event: Event, endpoint: GatewayEndpoint) -> Task<Message> {
    let target = match &event {
        Event::Read(target)
        | Event::Refresh(target, _)
        | Event::ToggleDetails(target, _)
        | Event::Finished(target, ..) => target,
        Event::RefreshAll | Event::BatchFinished(..) => return batch::update(app, event, endpoint),
    };
    let is_account = matches!(target, Target::Account(..));
    let expected_tab = if is_account { SystemTab::Auth } else { SystemTab::Apps };
    let completed = matches!(event, Event::Finished(..));
    if !completed && (!app.show_system_settings || app.system_settings_tab != expected_tab) {
        return Task::none();
    }
    let Some(scope) = scope(app, &endpoint, is_account) else {
        return if matches!(event, Event::Read(_) | Event::Refresh(..)) {
            app.show_error_toast("请先选择项目并连接执行主机")
        } else {
            Task::none()
        };
    };
    let linked = linked_scope(app, &endpoint, &scope, is_account);
    let other = if is_account { &app.app_providers.usage } else { &app.agent_apps_auth.usage };
    let linked_busy = linked.as_ref().is_some_and(|scope| other.refreshing(scope));
    let auth = &mut app.agent_apps_auth;
    let connection = auth.connection.as_ref().filter(|connection| {
        endpoint.is_loopback()
            && auth.scope.as_ref() == Some(&scope.endpoint)
            && connection.valid()
            && connection.host.trim() == scope.host
    });
    let (cards, token) = match target {
        Target::Provider(app_id, id) => {
            if app.app_providers.scope.as_ref().is_none_or(|scope| scope.app != *app_id)
                || !app.app_providers.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.catalog.rows.iter().any(|row| row.provider.id == *id)
                })
            {
                return Task::none();
            }
            (&mut app.app_providers.usage, app.apps_catalog.token.trim())
        }
        Target::Account(service, id) => {
            if !auth.rows.as_ref().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row.service == *service
                        && row.accounts.as_ref().is_ok_and(|accounts| {
                            accounts.iter().any(|account| {
                                account.account_id == *id
                                    && account.credential_state == AuthCredentialState::Ready
                            })
                        })
                })
            }) {
                return Task::none();
            }
            let Some(connection) = connection else { return Task::none() };
            (&mut auth.usage, connection.token.trim())
        }
    };
    let (target, refresh) = match event {
        Event::Read(target) => (target, None),
        Event::Refresh(target, provider) => (target, Some(provider)),
        Event::Finished(target, expected, ticket, result) => {
            if scope == expected && cards.scope.as_ref() == Some(&scope) {
                let refresh = cards.entries.get(&target).and_then(|entry| entry.refreshing.clone());
                if cards.finish(&target, ticket, result)
                    && let (Some(provider), Some(linked)) = (refresh, linked)
                    && let Some(Ok(items)) = &cards.entries[&target].result
                    && let Some(item) = items.iter().find(|item| item.provider() == provider)
                {
                    let item = item.clone();
                    let other =
                        if is_account { &mut app.app_providers.usage } else { &mut auth.usage };
                    other.apply_updates(&linked, std::slice::from_ref(&item));
                }
            }
            return Task::none();
        }
        Event::ToggleDetails(target, provider) => {
            cards.toggle_details(&scope, &target, provider);
            return Task::none();
        }
        Event::RefreshAll | Event::BatchFinished(..) => unreachable!("batch events handled above"),
    };
    if token.is_empty() {
        return app.show_error_toast("请先连接执行主机");
    }
    if cards.refreshing(&scope) || linked_busy {
        return Task::none();
    }
    let Ok(client) = GatewayClient::new(endpoint) else {
        return app.show_error_toast("无法建立额度查询连接");
    };
    cards.select_scope(&scope);
    let ticket = cards.begin(target.clone());
    if let Some(entry) = cards.entries.get_mut(&target) {
        entry.refreshing.clone_from(&refresh);
    }
    let context = QueryContext {
        project: scope.project.clone(),
        host: scope.host.clone(),
        token: token.into(),
        connection: connection.filter(|connection| connection.token.trim() == token).cloned(),
    };
    let requested = target.clone();
    Task::perform(
        async move { read(&client, &context, &requested, refresh.as_deref()).await },
        move |result| {
            message(ListMessage::Usage(Event::Finished(
                target.clone(),
                scope.clone(),
                ticket,
                result,
            )))
        },
    )
}

pub(crate) fn message_event(event: Event) -> Message {
    message(ListMessage::Usage(event))
}

pub(crate) fn current_scope(app: &App, account: bool) -> Option<Scope> {
    scope(app, &crate::app::gateway_client_endpoint(), account)
}
#[cfg(test)]
#[path="apps_usage_cards_tests.rs"]
pub(in crate::app) mod tests;
