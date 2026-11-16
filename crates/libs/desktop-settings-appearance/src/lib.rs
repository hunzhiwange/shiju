//! Appearance settings views with borrowed inputs and local actions.
pub mod quick_messages;
pub mod editor;
pub mod general;
pub mod companion;
pub mod help;
pub mod headings;

pub mod system_settings_themes;

pub mod quick_messages_update;
pub mod general_update;

pub mod general_message;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_16_tests;

#[cfg(test)]
mod host_module_26_tests;

pub mod general_fields;

#[cfg(test)]
mod companion_test_support;

pub mod theme_state;

pub mod general_state;

#[cfg(test)]
mod host_module_41_tests;

#[cfg(test)]
mod host_module_39_locale_tests;
