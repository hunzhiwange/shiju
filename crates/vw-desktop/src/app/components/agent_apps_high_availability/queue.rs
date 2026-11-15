use crate::app::Message;
use crate::app::message::agent_apps_high_availability::{self as queue, State};
use desktop_apps_ui::agent_apps_high_availability::queue as domain;
use iced::Element;
fn message(event: domain::Event) -> Message {
    queue::message(match event {
        domain::Event::Close => queue::Event::Close,
        domain::Event::Run(op) => queue::Event::Run(match op {
            domain::Operation::Refresh => queue::Operation::Refresh,
            domain::Operation::Add(id) => queue::Operation::Add(id),
            domain::Operation::Remove(id) => queue::Operation::Remove(id),
            domain::Operation::Move(a, b) => queue::Operation::Move(a, b),
            domain::Operation::Enabled(v) => queue::Operation::Enabled(v),
        }),
    })
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = domain::State {
        busy: state.busy,
        error: &state.error,
        queue: &state.queue,
        candidates: &state.candidates,
    };
    iced::widget::stack![
        base,
        domain::overlay(state, iced::widget::Space::new().into()).map(message)
    ]
    .into()
}
