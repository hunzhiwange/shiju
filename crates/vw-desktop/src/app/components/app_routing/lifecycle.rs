pub use desktop_apps_ui::app_routing::lifecycle::*;
use crate::app::{App, Message, state::AppsCatalogState};
pub(super) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::SettingsMessage::AppsCatalog(crate::app::message::settings::AppsCatalogMessage::Routing(event)))
}
fn context(catalog: &AppsCatalogState) -> Option<Context> {
    desktop_apps_ui::app_routing::lifecycle::context(catalog.context.as_ref(), &catalog.host, catalog.generation, catalog.model_diagnostics.access_generation, catalog.loading, &catalog.token)
}
pub(super) fn current(app: &App) -> Option<Context> {
    let current = context(&app.apps_catalog)?;
    let client = crate::app::config::gateway_client().ok()?;
    let expected = crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.navigation_tabs.project_path.clone() };
    (app.apps_catalog.context.as_ref() == Some(&expected)).then_some(current)
}
pub(crate) fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    use desktop_ui_controls::settings_navigation::ThirdPartyPage;

    let inline_page = super::super::third_party_settings::is_visible(app)
        && app.apps_catalog.third_party_page
            == ThirdPartyPage::Settings;
    let context = current(app);
    let load_policy = inline_page
        && matches!(event, Event::InspectPage)
        && page_can_configure(&app.apps_catalog.routing, context.clone())
        && !app.apps_catalog.high_availability.open;
    let client = crate::app::config::gateway_client().ok();
    let mut notice = None;
    let task = desktop_apps_ui::app_routing::lifecycle::update(
        &mut app.apps_catalog.routing,
        context,
        Some(
            vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref())
                .to_owned(),
        ),
        &app.apps_catalog.token,
        client,
        &mut notice,
        event,
        inline_page,
    )
    .map(message);
    let feedback = match notice {
        Some(desktop_apps_ui::Notice::Error(error)) => app.show_message_toast(crate::app::state::ToastKind::Error, error),
        Some(desktop_apps_ui::Notice::Success(text)) => app.show_message_toast(crate::app::state::ToastKind::Success, text),
        None => iced::Task::none(),
    };
    let policy = if load_policy {
        use crate::app::message::agent_apps_high_availability_policy::{self, Event};
        let first = ROUTING_APPS
            .into_iter()
            .find(|id| app.apps_catalog.rows.iter().any(|row| row.entry.id == *id));
        first.map_or_else(iced::Task::none, |id| {
            agent_apps_high_availability_policy::update(app, Event::Open(id))
        })
    } else {
        iced::Task::none()
    };
    iced::Task::batch([task, feedback, policy])
}
pub(crate) fn view(i18n: desktop_i18n::I18n, catalog: &AppsCatalogState) -> iced::Element<'_, Message> {
    desktop_apps_ui::app_routing::lifecycle::view(i18n, &catalog.routing, context(catalog)).map(message)
}

pub(crate) fn restore_overlay<'a>(i18n: desktop_i18n::I18n,
    catalog: &'a AppsCatalogState,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    let Some(card) = restore_confirmation(i18n, &catalog.routing, context(catalog)) else {
        return base;
    };
    desktop_ui_controls::system_settings_common::settings_modal_overlay(
        Some(base),
        message(Event::CancelRestore),
        card.map(message),
    )
}

pub(crate) fn inline_service(i18n: desktop_i18n::I18n, catalog: &AppsCatalogState) -> iced::Element<'_, Message> {
    page_service(i18n, &catalog.routing, context(catalog)).map(message)
}

pub(crate) fn attached_apps(
    catalog: &AppsCatalogState,
) -> Option<(&[vw_api_app_identity::AppId], bool)> {
    attached(&catalog.routing, context(catalog))
}

pub(crate) fn inline_available(catalog: &AppsCatalogState) -> bool {
    page_can_configure(&catalog.routing, context(catalog))
}

pub(crate) fn inline_rectifier(
    i18n: desktop_i18n::I18n,
    catalog: &AppsCatalogState,
) -> iced::Element<'_, Message> {
    page_rectifier(i18n, &catalog.routing, context(catalog)).map(message)
}
