//! Desktop wiring for usage cards and the provider settings editor.
use gateway_endpoint::GatewayEndpoint;
use iced::{Element, Task};

use vw_api_app_auth::AuthService;
use vw_api_app_identity::AppId;

pub use desktop_usage::apps_usage_cards::{Event, Scope, binding_app};

use super::system_settings::SystemTab;
use crate::app::{
    App, Message,
    message::app_providers::list::{ListMessage, message},
};

pub use desktop_usage::card_update::Cards;
fn map_view(event: desktop_usage::apps_usage_cards::ViewMessage) -> Message {
    match event {
        desktop_usage::apps_usage_cards::ViewMessage::Event(event) => message_event(event),
        desktop_usage::apps_usage_cards::ViewMessage::OpenSettings(app, id) => {
            super::apps_usage_settings::send(super::apps_usage_settings::Event::Open(app, id))
        }
        desktop_usage::apps_usage_cards::ViewMessage::OpenUrl(url) => {
            Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))
        }
    }
}
pub(crate) fn provider_view<'a>(
    scope: Option<&Scope>,
    cards: &'a Cards,
    app: AppId,
    id: &str,
) -> Element<'a, Message> {
    desktop_usage::apps_usage_cards::provider_view(scope, &cards.data, app, id).map(map_view)
}
pub(crate) fn account_view<'a>(
    scope: Option<&Scope>,
    cards: &'a Cards,
    service: AuthService,
    id: &str,
) -> Element<'a, Message> {
    desktop_usage::apps_usage_cards::account_view(scope, &cards.data, service, id).map(map_view)
}
pub(super) fn input(app: &mut App) -> desktop_usage::card_update::Input<'_> {
    use desktop_usage::card_update::{Catalog, Input, Providers, Tab};
    let third_party_visible = super::third_party_settings::is_visible(app);
    Input {
        project_id: &app.workspace_state.project_id,
        project_path: &app.navigation_tabs.project_path,
        agent_apps_auth: &mut app.agent_apps_auth,
        app_providers: Providers {
            scope: &app.app_providers.scope,
            snapshot: &app.app_providers.snapshot,
            generation: app.app_providers.generation,
            usage: &mut app.app_providers.usage,
        },
        apps_catalog: Catalog {
            host: &app.apps_catalog.host,
            token: &app.apps_catalog.token,
            generation: app.apps_catalog.generation,
        },
        show_system_settings: app.navigation_panels.show_system_settings || third_party_visible,
        system_settings_tab: match if third_party_visible { SystemTab::Apps } else { app.navigation_panels.system_settings_tab } {
            SystemTab::Apps => Tab::Apps,
            SystemTab::Auth => Tab::Auth,
            _ => Tab::Other,
        },
        endpoint: crate::app::gateway_client_endpoint(),
        error: None,
    }
}
fn run(
    app: &mut App,
    f: impl FnOnce(&mut desktop_usage::card_update::Input<'_>) -> Task<Event>,
) -> Task<Message> {
    let mut input = input(app);
    let task = f(&mut input).map(message_event);
    let error = input.error.take();
    Task::batch([task, error.map_or_else(Task::none, |error| app.show_error_toast(error))])
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    run(app, |input| desktop_usage::card_update::update(input, event))
}
pub(crate) fn read_linked_providers(app: &mut App, id: AppId) -> Task<Message> {
    run(app, |input| desktop_usage::card_update::read_linked_providers(input, id))
}
pub(crate) fn message_event(event: Event) -> Message {
    message(ListMessage::Usage(event))
}

pub(super) fn scope(app: &App, endpoint: &GatewayEndpoint, account: bool) -> Option<Scope> {
    use desktop_usage::card_update::{Catalog, ProviderScope, ScopeInput};
    desktop_usage::card_update::resolve_scope(
        &ScopeInput {
            project_id: &app.workspace_state.project_id,
            project_path: &app.navigation_tabs.project_path,
            agent_apps_auth: &app.agent_apps_auth,
            app_providers: ProviderScope {
                scope: &app.app_providers.scope,
                generation: app.app_providers.generation,
            },
            apps_catalog: Catalog {
                host: &app.apps_catalog.host,
                token: &app.apps_catalog.token,
                generation: app.apps_catalog.generation,
            },
        },
        endpoint,
        account,
    )
}
pub(crate) fn current_scope(app: &App, account: bool) -> Option<Scope> {
    scope(app, &crate::app::gateway_client_endpoint(), account)
}
#[cfg(test)]
#[path = "apps_usage_cards_tests.rs"]
pub(in crate::app) mod tests;

#[cfg(test)]
pub use desktop_usage::apps_usage_cards::Failure;
#[cfg(test)]
pub use desktop_usage::apps_usage_cards::Target;
