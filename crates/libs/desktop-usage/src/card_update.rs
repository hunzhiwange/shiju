//! Usage card authorization, request scheduling and linked cache transitions.
use crate::apps_usage_cards::{Event, QueryContext, Scope, Target, read};
use crate::refresh::refresh_batch;
use desktop_settings_account::types::Scope as AuthScope;
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use std::collections::HashSet;
use vw_api_app_auth::AuthCredentialState;
use vw_api_app_identity::AppId;
use vw_gateway_client::GatewayClient;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Apps,
    Auth,
    Other,
}
pub struct Providers<'a> {
    pub scope: &'a Option<desktop_provider_apps::Scope>,
    pub snapshot: &'a Option<desktop_provider_apps::state::list::Snapshot>,
    pub generation: u64,
    pub usage: &'a mut Cards,
}
pub struct Catalog<'a> {
    pub host: &'a str,
    pub token: &'a str,
    pub generation: u64,
}
pub struct Input<'a> {
    pub project_id: &'a Option<String>,
    pub project_path: &'a Option<String>,
    pub agent_apps_auth: &'a mut desktop_provider_apps::auth::state::State<Cards>,
    pub app_providers: Providers<'a>,
    pub apps_catalog: Catalog<'a>,
    pub show_system_settings: bool,
    pub system_settings_tab: Tab,
    pub endpoint: GatewayEndpoint,
    pub error: Option<String>,
}
impl Input<'_> {
    fn show_error_toast(&mut self, text: impl Into<String>) -> Task<Event> {
        self.error = Some(text.into());
        Task::none()
    }
}
#[derive(Clone, Debug, Default)]
pub struct Cards {
    pub data: crate::apps_usage_cards::Cards,
    pub settings: crate::apps_usage_settings::Editor,
}
impl std::ops::Deref for Cards {
    type Target = crate::apps_usage_cards::Cards;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl std::ops::DerefMut for Cards {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
impl Cards {
    pub fn clear(&mut self) {
        self.data.clear();
        self.settings.clear();
    }
    fn select_scope(&mut self, scope: &Scope) {
        if self.data.scope.as_ref() != Some(scope) {
            self.clear();
            self.data.scope = Some(scope.clone());
        }
    }
}
pub struct ScopeInput<'a> {
    pub project_id: &'a Option<String>,
    pub project_path: &'a Option<String>,
    pub agent_apps_auth: &'a desktop_provider_apps::auth::state::State<Cards>,
    pub app_providers: ProviderScope<'a>,
    pub apps_catalog: Catalog<'a>,
}
pub struct ProviderScope<'a> {
    pub scope: &'a Option<desktop_provider_apps::Scope>,
    pub generation: u64,
}
pub fn scope(app: &Input<'_>, endpoint: &GatewayEndpoint, account: bool) -> Option<Scope> {
    resolve_scope(
        &ScopeInput {
            project_id: app.project_id,
            project_path: app.project_path,
            agent_apps_auth: app.agent_apps_auth,
            app_providers: ProviderScope {
                scope: app.app_providers.scope,
                generation: app.app_providers.generation,
            },
            apps_catalog: Catalog {
                host: app.apps_catalog.host,
                token: app.apps_catalog.token,
                generation: app.apps_catalog.generation,
            },
        },
        endpoint,
        account,
    )
}
pub fn resolve_scope(
    app: &ScopeInput<'_>,
    endpoint: &GatewayEndpoint,
    account: bool,
) -> Option<Scope> {
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
            || &origin.context.project != app.project_path
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
    app: &Input<'_>,
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
pub fn read_linked_providers(app: &mut Input<'_>, app_id: AppId) -> Task<Event> {
    if !app.show_system_settings || app.system_settings_tab != Tab::Apps {
        return Task::none();
    }
    let endpoint = app.endpoint.clone();
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

pub fn update(app: &mut Input<'_>, event: Event) -> Task<Event> {
    update_at(app, event, app.endpoint.clone())
}

fn update_at(app: &mut Input<'_>, event: Event, endpoint: GatewayEndpoint) -> Task<Event> {
    let target = match &event {
        Event::Read(target)
        | Event::Refresh(target, _)
        | Event::ToggleDetails(target, _)
        | Event::Finished(target, ..) => target,
        Event::RefreshAll | Event::BatchFinished(..) => return update_batch(app, event, endpoint),
    };
    let is_account = matches!(target, Target::Account(..));
    let expected_tab = if is_account { Tab::Auth } else { Tab::Apps };
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
    let other: &Cards =
        if is_account { &*app.app_providers.usage } else { &app.agent_apps_auth.usage };
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
            (&mut *app.app_providers.usage, app.apps_catalog.token.trim())
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
                        if is_account { &mut *app.app_providers.usage } else { &mut auth.usage };
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
        move |result| Event::Finished(target.clone(), scope.clone(), ticket, result),
    )
}

fn update_batch(app: &mut Input<'_>, event: Event, endpoint: GatewayEndpoint) -> Task<Event> {
    let Some(scope) = scope(app, &endpoint, true) else { return Task::none() };
    let linked = linked_scope(app, &endpoint, &scope, true);
    let auth = &mut app.agent_apps_auth;
    let Some(connection) =
        auth.connection.as_ref().filter(|connection| endpoint.is_loopback() && connection.valid())
    else {
        return Task::none();
    };
    let cards = &mut auth.usage;
    match event {
        Event::RefreshAll => {
            if !app.show_system_settings
                || app.system_settings_tab != Tab::Auth
                || auth.binding.is_some()
                || auth.pending.is_some()
                || auth.account_action.is_some()
                || auth.rows.is_none()
                || cards.refreshing(&scope)
                || linked.as_ref().is_some_and(|scope| app.app_providers.usage.refreshing(scope))
            {
                return Task::none();
            }
            let Ok(client) = GatewayClient::new(endpoint) else {
                return app.show_error_toast("无法建立额度查询连接");
            };
            cards.select_scope(&scope);
            let ticket = cards.next_ticket();
            cards.batch_pending = Some(ticket);
            cards.batch_result = None;
            let project = scope.project.clone();
            let host = scope.host.clone();
            let token = connection.token.trim().to_owned();
            Task::perform(
                async move { refresh_batch(&client, &project, &host, &token).await },
                move |result| Event::BatchFinished(scope.clone(), ticket, result),
            )
        }
        Event::BatchFinished(expected, ticket, result)
            if scope == expected
                && cards.scope.as_ref() == Some(&scope)
                && cards.batch_pending == Some(ticket) =>
        {
            cards.batch_pending = None;
            if let Ok(items) = &result {
                cards.apply_updates(&scope, items);
                if let Some(linked) = linked {
                    app.app_providers.usage.apply_updates(&linked, items);
                }
            }
            let error = result.as_ref().err().copied();
            cards.batch_result = Some(result);
            if let Some(error) = error {
                return app.show_error_toast(error.label());
            }
            if let Some(provider_scope) = &app.app_providers.scope {
                return read_linked_providers(app, provider_scope.app);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

impl desktop_provider_apps::auth::state::Usage for Cards {
    fn clear(&mut self) {
        self.clear();
    }
}
