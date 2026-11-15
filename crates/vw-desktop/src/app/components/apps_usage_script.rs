//! Maps local script editor messages into provider settings.
pub use desktop_usage::apps_usage_script::{Event, Editor};
use super::apps_usage_settings::{self, Event as SettingsEvent};
use crate::app::Message;
use iced::Element;
use vw_api_apps_usage::apps_usage::UsageSnapshotQuery;
pub(super) fn map(event: desktop_usage::apps_usage_script::Message) -> Message {
    apps_usage_settings::send(match event {
        desktop_usage::apps_usage_script::Message::Event(event) => SettingsEvent::Script(event),
        desktop_usage::apps_usage_script::Message::Close => SettingsEvent::Close,
        desktop_usage::apps_usage_script::Message::OpenScript => SettingsEvent::OpenScript,
    })
}
pub(super) fn modal_view<'a>(editor: &'a Editor, selection: &UsageSnapshotQuery, base: Element<'a, Message>) -> Element<'a, Message> {
    // The modal is composed with the desktop base through a generic mapped message.
    desktop_usage::apps_usage_script::modal_view_with_base(editor, selection, base, map)
}
