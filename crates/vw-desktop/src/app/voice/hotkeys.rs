//! Desktop settings and message mapping for voice shortcuts.
pub use desktop_voice::hotkeys::*;
pub(crate) fn record_subscription() -> iced::Subscription<crate::app::Message> {
    desktop_voice::hotkeys::record_subscription().map(|event| {
        use crate::app::message::settings::voice::Event;
        let event = match event {
            RecordEvent::Cancel => Event::RecordHotkey(None),
            RecordEvent::Recorded(shortcut) => Event::RecordedHotkey(shortcut),
        };
        crate::app::Message::Settings(crate::app::message::SettingsMessage::Voice(event))
    })
}
