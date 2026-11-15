pub use crate::message::AgentsMessage;
pub type AgentsLoaded = Result<(Vec<desktop_settings_providers::provider_state::ProviderSummary>, Vec<desktop_settings_providers::state::ProviderModelsSummary>, Vec<String>), String>;
#[derive(Debug, Clone)] pub enum SettingsMessage { Agents(AgentsMessage) }
#[derive(Debug, Clone)] pub enum Message { Settings(SettingsMessage), ConfigSaved { tag: &'static str, result: Result<(), String> } }
#[derive(Debug, Clone, Copy)] pub enum ToastKind { Success, Warning, Error }
pub enum Effect { Toast(ToastKind, &'static str, Vec<desktop_i18n::format::Argument>), StartupEdit, EditAcp(String), RefreshAcp, RefreshMcp, Launch { project: String, key: String } }
