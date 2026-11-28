use crate::app::Message;
use crate::app::message::agent_apps_high_availability_policy::{self as policy, State};
use desktop_apps_ui::agent_apps_high_availability::policy_modal as domain;
use iced::Element;
pub(crate) use domain::overview;
fn message(event: domain::Event) -> Message {
    policy::message(match event {
        domain::Event::Close => policy::Event::Close,
        domain::Event::Edit(i, v) => policy::Event::Edit(i, v),
        domain::Event::Save => policy::Event::Save,
        domain::Event::Reload => policy::Event::Reload,
    })
}
pub(crate) fn inline(i18n: desktop_i18n::I18n, state: &State) -> Element<'_, Message> {
    domain::inline(i18n, view_state(state)).map(message)
}

fn view_state(state: &State) -> domain::State<'_> {
    domain::State {
        open: state.open,
        busy: state.busy,
        loaded: state.loaded,
        fields: &state.fields,
        error: &state.error,
    }
}

pub(crate) fn overlay<'a>(i18n: desktop_i18n::I18n, state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let state = view_state(state);
    iced::widget::stack![
        base,
        domain::overlay(i18n, state, iced::widget::Space::new().into()).map(message)
    ]
    .into()
}
