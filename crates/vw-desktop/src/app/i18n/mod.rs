//! Explicit locale catalogs. Raw user and service content is never translated.
#[cfg(test)]
mod tests;

pub mod settings_models;
#[cfg(test)]
mod settings_models_tests;

pub mod settings_runtime;
#[cfg(test)]
mod settings_runtime_tests;

pub mod settings_security;

pub mod settings_extensions;
#[cfg(test)]
mod settings_extensions_tests;

pub mod catalog;
pub use catalog::I18n;
#[cfg(test)]
mod gettext_catalog_tests;

pub mod format;
#[cfg(test)]
mod gettext_sprintf_tests;

#[cfg(test)]
mod gettext_selection_tests;

#[cfg(test)]
mod gettext_area_4_tests;

#[cfg(test)]
mod gettext_area_5_tests;

#[cfg(test)]
mod gettext_area_6_tests;
