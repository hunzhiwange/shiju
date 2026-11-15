use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_update::notifications as domain;
fn map(message: domain::Message) -> Message {
    Message::View(match message {
        domain::Message::AutoCheckAppUpdate => ViewMessage::AutoCheckAppUpdate,
        domain::Message::AutoCheckAppUpdateFinished(a,b,c) => ViewMessage::AutoCheckAppUpdateFinished(a,b,c),
        domain::Message::SkipAppUpdate => ViewMessage::SkipAppUpdate,
        domain::Message::SkipAppUpdateSaved(a,b) => ViewMessage::SkipAppUpdateSaved(a,b),
    })
}
pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::AutoCheckAppUpdate => domain::Message::AutoCheckAppUpdate,
        ViewMessage::AutoCheckAppUpdateFinished(a,b,c) => domain::Message::AutoCheckAppUpdateFinished(a,b,c),
        ViewMessage::SkipAppUpdate => domain::Message::SkipAppUpdate,
        ViewMessage::SkipAppUpdateSaved(a,b) => domain::Message::SkipAppUpdateSaved(a,b),
        _ => return Task::none(),
    };
    let (task, effect) = domain::update(&mut app.desktop_update, app.show_cli_install_modal, env!("CARGO_PKG_VERSION"), message);
    let effect = match effect {
        Some(domain::Effect::SaveSkipped(skipped, patch)) => Task::perform(
            crate::app::config::save_app_config_async(patch),
            move |result| map(domain::Message::SkipAppUpdateSaved(skipped.clone(), result)),
        ),
        Some(domain::Effect::SaveFailed(error)) => app.show_localized_toast(
            crate::app::state::ToastKind::Error, "跳过版本未保存：%s", vec![error.into()],
        ),
        None => Task::none(),
    };
    Task::batch([task.map(map), effect])
}
#[cfg(test)]
#[path = "update_notifications_tests.rs"]
mod tests;
