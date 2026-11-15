use crate::app::{App, Message};
use crate::app::message::settings::SettingsMessage;
#[cfg(test)]
use crate::app::message::settings::MemoryMessage;
use crate::app::state::MemorySettingsTab;
use crate::app::components::system_settings_search::SettingsTabDescriptor;
#[cfg(test)]
use crate::app::i18n::settings_runtime::text as runtime_text;
use desktop_settings_memory::{messages::PageMessage, system_settings_memory as domain};
use iced::Element;
pub(super) const MEMORY_TABS: &[SettingsTabDescriptor<MemorySettingsTab>] = &[
    descriptor(domain::MEMORY_TABS[0]),
    descriptor(domain::MEMORY_TABS[1]),
    descriptor(domain::MEMORY_TABS[2]),
    descriptor(domain::MEMORY_TABS[3]),
    descriptor(domain::MEMORY_TABS[4]),
];
pub(super) const QDRANT_TAB: &[SettingsTabDescriptor<MemorySettingsTab>] = &[descriptor(domain::QDRANT_TAB[0])];
const fn descriptor(d: domain::SettingsTabDescriptor<MemorySettingsTab>) -> SettingsTabDescriptor<MemorySettingsTab> {
    SettingsTabDescriptor::new(d.tab, d.label, d.keywords)
}
pub fn view(app: &App) -> Element<'_, Message> {
    let routes = if app.memory_settings.active_tab == MemorySettingsTab::EmbeddingRoutes {
        crate::app::components::system_settings_embedding_routes::embedded_view(app).map(PageMessage::External)
    } else {
        iced::widget::Space::new().into()
    };
    domain::view(&app.memory_settings, &app.storage_settings, app.desktop_locale, &app.app_theme, routes).map(|event| match event {
        PageMessage::Memory(event) => Message::Settings(SettingsMessage::Memory(event)),
        PageMessage::Storage(event) => Message::Settings(SettingsMessage::Storage(event)),
        PageMessage::External(message) => message,
        PageMessage::None => Message::None,
    })
}
pub(super) use domain::memory_tab_label;
#[cfg(test)]
#[path = "system_settings_memory_ui_refresh_20_tests.rs"]
mod ui_refresh_20_tests;
