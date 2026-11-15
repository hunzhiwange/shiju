pub use desktop_provider_apps::message::universal::Event;
use crate::app::{App, Message};
pub(crate) fn message(event: Event) -> Message {
    super::list::message(super::list::ListMessage::Universal(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    let current = app.app_providers.scope.as_ref().ok_or("缺少供应商作用域".to_string())
        .and_then(|scope| super::list::current(app, scope.app));
    super::provider_update::run(app, |state, _, _| {
        desktop_provider_apps::message::universal::update(state, current, event)
    })
}
