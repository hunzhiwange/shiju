use crate::app::Message;
use crate::app::message::agent_apps_high_availability_policy::{self as policy, State};
use desktop_apps_ui::agent_apps_high_availability::policy_modal as domain;
use iced::Element;
fn message(event: domain::Event) -> Message {
    policy::message(match event {
        domain::Event::Close => policy::Event::Close,
        domain::Event::Edit(i, v) => policy::Event::Edit(i, v),
        domain::Event::Save => policy::Event::Save,
        domain::Event::Reload => policy::Event::Reload,
    })
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let state = domain::State {
        open: state.open,
        busy: state.busy,
        loaded: state.loaded,
        fields: &state.fields,
        error: &state.error,
    };
    iced::widget::stack![
        base,
        domain::overlay(state, iced::widget::Space::new().into()).map(message)
    ]
    .into()
}
