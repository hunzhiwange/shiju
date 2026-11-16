//! Runtime settings domain.
pub mod state;
pub mod messages;
pub mod help;
#[derive(Debug, Clone)]
pub enum Message { None, Settings(messages::SettingsMessage), CopyCode(String) }
pub mod system_settings_hooks;
pub mod system_settings_runtime;
pub mod system_settings_multimodal;
pub mod system_settings_observability;
pub mod system_settings_query_classification;
pub mod system_settings_reliability;
pub mod hooks;
pub mod multimodal;
pub mod observability;
pub mod query_classification;

pub mod app_init;

pub mod runtime;
pub mod reliability;

#[cfg(test)]
mod state_tests;

pub mod save_feedback;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_29_tests;

#[cfg(test)]
mod host_module_20_tests;

#[cfg(test)]
mod host_module_17_tests;

#[cfg(test)]
mod host_module_39_runtime_tests;

#[cfg(test)]
mod host_module_39_locale_tests;
