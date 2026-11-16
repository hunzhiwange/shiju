//! Authorization center requests through the scoped management SDK.

#[path = "agent_apps_auth_actions.rs"]
pub(in crate::app) mod actions;
pub(in crate::app) mod binding;
#[path = "agent_apps_auth_browser.rs"]
pub(in crate::app) mod browser;
pub(in crate::app) mod flow;

use crate::app::{App, Message, components::system_settings::SystemTab, message::SettingsMessage};
use browser::LinkAction;
use iced::Task;

pub(crate) use desktop_provider_apps::auth::SERVICES;

pub use desktop_provider_apps::auth::Event;

pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::AgentAppsAuth(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let endpoint = crate::app::gateway_client_endpoint();
    update_at(app, event, endpoint)
}

pub(in crate::app) fn update_at(
    app: &mut App,
    event: Event,
    endpoint: gateway_endpoint::GatewayEndpoint,
) -> Task<Message> {
    use desktop_provider_apps::auth::{
        binding::{Effects, Origin},
        controller,
    };
    let origin = Origin {
        visible_apps: app.navigation_panels.show_system_settings && app.navigation_panels.system_settings_tab == SystemTab::Apps,
        visible_auth: app.navigation_panels.show_system_settings && app.navigation_panels.system_settings_tab == SystemTab::Auth,
        provider_generation: app.app_providers.generation,
        provider_scope: app.app_providers.scope.as_ref(),
        form: app.app_providers.form.as_ref(),
        project_path: &app.navigation_tabs.project_path,
        token: &app.apps_catalog.token,
        host: &app.apps_catalog.host,
        catalog_generation: app.apps_catalog.generation,
    };
    let mut notice = None;
    let mut effects = Effects::default();
    let task = controller::update(
        &mut app.agent_apps_auth,
        &mut notice,
        &mut effects,
        origin,
        app.apps_catalog.context.as_ref(),
        &NativePorts,
        event,
        endpoint,
    )
    .map(message);
    if effects.opened {
        app.navigation_panels.system_settings_tab = SystemTab::Auth;
        app.navigation_panels.system_settings_help_tab = None;
    }
    if effects.saved {
        app.app_providers.usage.clear();
    }
    let feedback = match notice {
        Some(desktop_provider_apps::Notice::Error(text)) => app.show_error_toast(text),
        Some(desktop_provider_apps::Notice::Success(text)) => app.show_success_toast(text),
        None => Task::none(),
    };
    Task::batch([task, feedback])
}
struct NativePorts;
impl desktop_provider_apps::auth::controller::Ports for NativePorts {
    fn store_preference(&self, open: bool) -> Result<(), &'static str> {
        browser::store_preference(open)
    }
    fn deliver(&self, action: LinkAction, url: &reqwest::Url) -> Result<(), &'static str> {
        browser::deliver(action, url)
    }
    fn connect_auth(
        &self,
        client: vw_gateway_client::GatewayClient,
    ) -> impl std::future::Future<Output = Result<local_app_auth::Credentials, String>> + Send + 'static
    {
        crate::app::components::system_settings_apps::local::connect_auth(client)
    }
}
pub(crate) use desktop_provider_apps::auth::{load, load_service};
