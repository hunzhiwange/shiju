//! Native prompt library and scoped editor.
pub(crate) mod list;
pub(crate) use list::State;
pub(crate) mod activation;
pub(crate) mod editor;

pub(crate) mod pi_files;

pub(crate) mod sync;

pub(crate) mod templates;

pub(crate) mod transfer;

pub(crate) mod shared_import;

pub(crate) fn ui_message(event: desktop_settings_skills::prompts::Event) -> crate::app::Message {
    use crate::app::{
        Message,
        components::system_settings::SystemTab,
        message::{SettingsMessage, settings::prompts},
    };
    use desktop_settings_skills::prompts::ui_route::{UiRoute, ui_route};
    match ui_route(event) {
        UiRoute::Update(event) => prompts::message(event),
        UiRoute::Agents => Message::Settings(SettingsMessage::SystemTabSelected(SystemTab::Agents)),
        UiRoute::Projects => {
            Message::Settings(SettingsMessage::SystemTabSelected(SystemTab::Projects))
        }
    }
}
