//! SDK-backed ledger entry; current-session details remain in the parent view.
pub mod charts;
pub mod display;
pub mod groups;
pub mod logs;
pub mod overview;
pub mod pricing;
pub mod scan;
pub mod state;
use crate::app::state::agent_apps_auth::Scope;
use crate::app::{App, Message, Screen, message::ViewMessage};
use iced::{Element, Task};

pub use desktop_usage::Identity;

fn identity(app: &App) -> Option<Identity> {
    let project = vw_api_transport::local_settings_project(app.workspace_state.project_id.as_deref()).to_owned();
    let host = local_app_auth::HOST_ID.to_owned();
    Some(Identity {
        endpoint: Scope::from(&crate::app::config::gateway_client_endpoint()),
        project,
        host,
    })
}

pub use desktop_usage::center::event::Event;

fn is_usage_screen(app: &App) -> bool {
    matches!(app.navigation_tabs.screen, Screen::Usage | Screen::ThirdPartyAgents)
        && !app.navigation_panels.show_system_settings
}

pub(crate) fn is_visible(app: &App) -> bool {
    app.usage_center.open && is_usage_screen(app)
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = identity(app);
    let visible = is_usage_screen(app);
    let mut input = desktop_usage::center::update::Input {
        state: &mut app.usage_center,
        current,
        visible,
        client: crate::app::config::gateway_client,
        feedback: None,
    };
    let task = desktop_usage::center::update::update(&mut input, event)
        .map(|event| Message::View(ViewMessage::UsageCenter(event)));
    let feedback = input.feedback.take();
    let navigation = desktop_usage::center::scan::take_navigation(&mut app.usage_center.scan);
    let navigation = match navigation {
        Some(desktop_usage::center::scan::Navigation::Display) => update(app, Event::Display(display::Event::Open)),
        Some(desktop_usage::center::scan::Navigation::Pricing) => update(app, Event::Pricing(pricing::Event::Open)),
        Some(desktop_usage::center::scan::Navigation::Routing) => {
            let navigation = crate::app::components::third_party_settings::navigate(
                app, desktop_ui_controls::settings_navigation::ThirdPartyPage::ManagementTools,
            );
            if app.navigation_tabs.screen == Screen::ThirdPartyAgents {
                navigation
            } else {
                Task::batch([navigation, Task::done(Message::View(ViewMessage::OpenThirdPartyAgents))])
            }
        }
        None => Task::none(),
    };
    Task::batch([
        task,
        navigation,
        feedback.map_or_else(Task::none, |(success, message)| {
            if success { app.show_success_toast(message) } else { app.show_error_toast(message) }
        }),
    ])
}

pub fn view(app: &App) -> Element<'_, Message> {
    let known_cost = app
        .usage_center
        .result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|summary| display::amount(app, &summary.known_cost_usd));
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let content = desktop_usage::center::view::view(&app.usage_center, known_cost, i18n)
        .map(|event| match event {
            Event::Logs(event) => logs::map(event),
            event => Message::View(ViewMessage::UsageCenter(event)),
        });
    display::overlay(app, pricing::overlay(app, scan::overlay(app, content)))
}

/// The sidebar snapshot covers the local day independently of dashboard filters.
pub(crate) fn today_summary(app: &App) -> Option<(String, String)> {
    let current = identity(app);
    desktop_usage::center::display::sidebar_today(
        &app.usage_center.display, current, app.usage_center.today.as_ref(),
    )
}
