//! Explicit locale catalogs. Raw user and service content is never translated.
pub mod native;
pub mod format;
pub mod other;
pub mod apps;
pub mod settings_extensions;
pub mod settings_runtime;
pub mod tasks;
pub mod catalog;
pub mod git;
pub mod chat_time;
pub mod settings_security;
pub mod project;
pub mod editor;
pub mod workflow;
pub mod settings_models;
pub mod modals;
pub mod input;
pub mod chat;
pub use catalog::I18n;
#[cfg(test)]
mod settings_models_tests;
#[cfg(test)]
mod gettext_catalog_tests;
#[cfg(test)]
mod gettext_area_17_tests;
#[cfg(test)]
mod gettext_area_16_tests;
#[cfg(test)]
mod settings_extensions_tests;
#[cfg(test)]
mod gettext_area_7_tests;
#[cfg(test)]
mod gettext_area_6_tests;
#[cfg(test)]
mod modals_tests;
#[cfg(test)]
mod settings_runtime_tests;
#[cfg(test)]
mod gettext_area_10_tests;
#[cfg(test)]
mod gettext_area_11_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod gettext_area_9_tests;
#[cfg(test)]
mod gettext_area_8_tests;
#[cfg(test)]
mod gettext_area_14_tests;
#[cfg(test)]
mod gettext_area_15_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod gettext_sprintf_tests;
#[cfg(test)]
mod gettext_area_13_tests;
#[cfg(test)]
mod gettext_area_12_tests;
#[cfg(test)]
mod gettext_area_19_tests;
#[cfg(test)]
mod gettext_area_18_tests;
#[cfg(test)]
mod workflow_tests;
#[cfg(test)]
mod chat_tests;
#[cfg(test)]
mod gettext_selection_tests;
#[cfg(test)]
mod gettext_area_4_tests;
#[cfg(test)]
mod gettext_area_5_tests;

#[cfg(test)]
mod desktop_split_22_tests;
