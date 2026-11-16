//! Local fixtures use the real domain state, reducer and embedded route view.
use crate::messages::PageMessage;
pub use crate::system_settings_memory::memory_tab_label;
pub use crate::{
    messages::{MemoryMessage, StorageMessage},
    state::MemorySettingsTab,
};
pub use desktop_i18n::settings_runtime::text as runtime_text;
pub use desktop_settings_providers::embedding_view_message::EmbeddingRoutesMessage;
pub use iced::Element;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Memory(MemoryMessage),
    Storage(StorageMessage),
    EmbeddingRoutes(EmbeddingRoutesMessage),
}
#[derive(Debug, Clone)]
pub enum Message {
    Settings(SettingsMessage),
    CopyCode(String),
    None,
}

#[derive(Default)]
pub struct Fixture {
    pub memory_settings: crate::state::MemorySettingsState,
    pub storage_settings: crate::state::StorageSettingsState,
    pub embedding_routes_settings:
        desktop_settings_providers::app_init::state::EmbeddingRoutesSettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}

pub fn update(app: &mut Fixture, message: SettingsMessage) -> crate::memory_update::Effect {
    match message {
        SettingsMessage::Memory(message) => {
            let effect = crate::memory_update::update(&mut app.memory_settings, message);
            if effect == crate::memory_update::Effect::ClearEmbeddingRouteFeedback {
                app.embedding_routes_settings.save_error = None;
                app.embedding_routes_settings.save_success = false;
            }
            effect
        }
        SettingsMessage::Storage(message) => {
            let _ = crate::storage_update::prepare_save(&mut app.storage_settings, message);
            crate::memory_update::Effect::None
        }
        SettingsMessage::EmbeddingRoutes(message) => {
            let _ = desktop_settings_providers::embedding_routes_update::update(
                &mut app.embedding_routes_settings,
                message,
            );
            crate::memory_update::Effect::None
        }
    }
}

impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(value: String) -> Self {
        Self::CopyCode(value)
    }
    fn context(_: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        Self::None
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}

pub fn view(app: &Fixture) -> Element<'_, Message> {
    use desktop_settings_providers::{view_message as routes, view_style::ViewStyle};
    fn help<'a>(
        title: &'a str,
        body: std::borrow::Cow<'a, str>,
        color: Option<iced::Color>,
        i18n: desktop_i18n::I18n,
    ) -> Element<'a, routes::Message> {
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(
            title, body, color, i18n,
        )
        .map(|message| match message {
            Message::CopyCode(value) => routes::Message::CopyCode(value),
            _ => routes::Message::None,
        })
    }
    let embedded = if app.memory_settings.active_tab == MemorySettingsTab::EmbeddingRoutes {
        desktop_settings_providers::embedding_routes_view::embedded_view(
            &app.embedding_routes_settings,
            ViewStyle { locale: app.desktop_locale, theme: &app.app_theme, help_text: help },
        )
        .map(|message| match message {
            routes::Message::Settings(routes::SettingsMessage::EmbeddingRoutes(message)) => {
                Message::Settings(SettingsMessage::EmbeddingRoutes(message))
            }
            routes::Message::CopyCode(value) => Message::CopyCode(value),
            _ => Message::None,
        })
    } else {
        iced::widget::Space::new().into()
    };
    crate::system_settings_memory::view(
        &app.memory_settings,
        &app.storage_settings,
        app.desktop_locale,
        &app.app_theme,
        embedded,
    )
    .map(|message| match message {
        PageMessage::Memory(message) => Message::Settings(SettingsMessage::Memory(message)),
        PageMessage::Storage(message) => Message::Settings(SettingsMessage::Storage(message)),
        PageMessage::External(message) => message,
        PageMessage::None => Message::None,
    })
}
