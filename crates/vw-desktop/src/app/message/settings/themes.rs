//! Theme-domain update and desktop effect adapter.
use crate::app::{App, Message};
use iced::Task;
use super::SettingsMessage;
use desktop_settings_appearance::system_settings_themes::update::{self as domain, Effect};
pub use desktop_settings_appearance::system_settings_themes::messages::{ThemeChromeField, ThemeColorPickerTarget, ThemePaletteField, ThemesMessage};

pub(super) fn toast_kind(kind: domain::ToastKind) -> crate::app::state::ToastKind {
    match kind {
        domain::ToastKind::Success => crate::app::state::ToastKind::Success,
        domain::ToastKind::Warning => crate::app::state::ToastKind::Warning,
        domain::ToastKind::Error => crate::app::state::ToastKind::Error,
    }
}
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Themes(message) = message else { return Task::none(); };
    let mut inputs = domain::Update {
        themes_settings: &mut app.themes_settings,
        active_custom_theme_id: &mut app.active_custom_theme_id,
        active_theme_skin: &mut app.active_theme_skin,
        app_theme: &mut app.app_theme,
        effects: Vec::new(),
    };
    let task = domain::update(&mut inputs, message).map(|message| Message::Settings(SettingsMessage::Themes(message)));
    let effects = std::mem::take(&mut inputs.effects);
    let final_skin = app.active_theme_skin.clone();
    let final_id = app.active_custom_theme_id.clone();
    let mut tasks = vec![task];
    for effect in effects {
        match effect {
            Effect::ApplyTheme(theme, id) => {
                app.active_custom_theme_id = id;
                tasks.push(crate::app::message::view::update(app, crate::app::message::view::ViewMessage::AppThemeSelected(theme)));
            }
            Effect::Toast(kind, source, arguments) => tasks.push(app.show_localized_toast(toast_kind(kind), &source, arguments)),
        }
    }
    app.active_theme_skin = final_skin;
    app.active_custom_theme_id = final_id;
    Task::batch(tasks)
}
#[cfg(test)]
#[path = "themes_tests.rs"]
mod themes_tests;
