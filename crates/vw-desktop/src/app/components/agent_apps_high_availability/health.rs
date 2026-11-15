use crate::app::{App, Message, message::agent_apps_health as health};
use desktop_apps_ui::agent_apps_high_availability::health as domain;
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
    let state = &app.apps_catalog.failover_health;
    domain::view(
        domain::State {
            health: &state.health,
            route: &state.route,
            error: &state.error,
            busy: state.busy,
        },
        health::visible(app),
    )
    .map(|event| {
        health::message(match event {
            domain::Event::Refresh => health::Event::Refresh,
            domain::Event::Reset => health::Event::Reset,
        })
    })
}
