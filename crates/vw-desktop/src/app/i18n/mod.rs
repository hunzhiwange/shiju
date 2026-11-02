//! Explicit locale catalogs. Raw user and service content is never translated.
#[cfg(test)]
mod tests;

use vw_config_types::ui::DesktopLocale;

/// Text owned by the settings domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsText {
    Language,
}

pub fn settings_text(locale: DesktopLocale, key: SettingsText) -> &'static str {
    match key {
        SettingsText::Language => I18n::new(locale).__("语言"),
    }
}

/// Localize the application prefix while retaining the service error verbatim.
pub fn save_error(locale: DesktopLocale, raw_error: &str) -> String {
    let prefix = I18n::new(locale).__("保存失败：");
    format!("{prefix}{raw_error}")
}

pub mod settings_models;
#[cfg(test)]
mod settings_models_tests;

pub mod settings_runtime;
#[cfg(test)]
mod settings_runtime_tests;

pub mod settings_security;

pub mod catalog;
pub use catalog::I18n;
#[cfg(test)]
mod gettext_catalog_tests;
