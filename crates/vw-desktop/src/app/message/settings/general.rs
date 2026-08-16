use crate::app::{App, Message};
use iced::Task;

use super::{GeneralSettingsMessage, SettingsMessage};

#[cfg(test)]
static LAST_PERSISTED_TIMEZONE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(test)]
fn persist_timezone(timezone: String) {
    *LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock") = Some(timezone);
}

#[cfg(not(test))]
fn persist_timezone(timezone: String) {
    crate::app::update_system_settings_config(move |system| {
        system.timezone = timezone;
    });
}

pub(super) fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::General(message) = message else {
        return Task::none();
    };

    match message {
        GeneralSettingsMessage::TabSelected(tab) => {
            app.general_settings_tab = tab;
        }
        GeneralSettingsMessage::TimezoneSelected(timezone) => {
            if !vw_config_types::ui::timezone_options().contains(&timezone.as_str()) {
                return Task::none();
            }

            app.chat_timezone = timezone.clone();
            persist_timezone(timezone);
        }
    }
    Task::none()
}

#[cfg(test)]
fn clear_last_persisted_timezone() {
    *LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock") = None;
}

#[cfg(test)]
fn last_persisted_timezone() -> Option<String> {
    LAST_PERSISTED_TIMEZONE.lock().expect("timezone recorder lock").clone()
}

#[cfg(test)]
#[path = "general_tests.rs"]
mod general_tests;
