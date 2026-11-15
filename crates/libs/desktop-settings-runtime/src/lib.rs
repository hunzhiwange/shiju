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
