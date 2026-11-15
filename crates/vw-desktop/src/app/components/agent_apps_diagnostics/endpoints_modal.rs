use crate::app::Message;
use crate::app::message::agent_apps_diagnostics::{endpoint_state::State, endpoints};
use desktop_apps_ui::agent_apps_diagnostics::endpoints_modal as domain;
use iced::Element;
fn message(event: domain::Event) -> Message {
    endpoints::message(match event {
        domain::Event::Close => endpoints::Event::Close,
        domain::Event::Start => endpoints::Event::Start,
        domain::Event::Cancel => endpoints::Event::Cancel,
        domain::Event::Reload => endpoints::Event::Reload,
        domain::Event::ManageCustom => endpoints::Event::ManageCustom,
        domain::Event::Apply => endpoints::Event::Apply,
        domain::Event::Preset(v) => endpoints::Event::Preset(v),
        domain::Event::Select(id) => endpoints::Event::Select(id),
    })
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = domain::State {
        loading: state.loading,
        active: state.active(),
        cancelling: state.cancelling,
        closing: state.closing,
        candidates: state.sorted(),
        reports: state
            .candidates
            .iter()
            .filter_map(|c| state.report(&c.id).map(|r| (&c.id, r)))
            .collect(),
        preset_name: &state.preset_name,
        snapshot: &state.snapshot,
        selected: &state.selected,
        error: &state.error,
    };
    iced::widget::stack![
        base,
        domain::overlay(state, iced::widget::Space::new().into()).map(message)
    ]
    .into()
}
#[cfg(test)]
pub(crate) use domain::report_text;
