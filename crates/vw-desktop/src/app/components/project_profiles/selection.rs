//! Map the workspace profile chooser into the desktop settings route.
use crate::app::{Message, state::project_profiles::State};
use crate::app::message::project_profiles::{Event, ApplyEvent, message};
use desktop_workspace::components::project_profiles as domain;
use vw_api_project_profiles::ProjectProfileScope;

pub(crate) fn view(state: &State, scope: ProjectProfileScope) -> iced::Element<'_, Message> {
    domain::view(state, scope).map(|event| message(match event {
        domain::Event::Apply(scope, event) => Event::Apply(scope, match event {
            domain::ApplyEvent::Open(id, version) => ApplyEvent::Open(id, version),
            domain::ApplyEvent::Close => ApplyEvent::Close,
            domain::ApplyEvent::Confirm => ApplyEvent::Confirm,
            domain::ApplyEvent::Recover => ApplyEvent::Recover,
            domain::ApplyEvent::Cancel => ApplyEvent::Cancel,
        }),
        domain::Event::Open(scope, action) => Event::Open(scope, action),
        domain::Event::Name(scope, name) => Event::Name(scope, name),
        domain::Event::Close(scope) => Event::Close(scope),
        domain::Event::Submit(scope) => Event::Submit(scope),
        domain::Event::Refresh(scope) => Event::Refresh(scope),
        domain::Event::Query(scope, query) => Event::Query(scope, query),
    }))
}
