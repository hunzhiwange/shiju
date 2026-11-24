pub mod agent_apps;

pub use desktop_provider_apps::{AgentAppsContext, Scope as ProviderScope};

pub struct Connection {
    pub client: vw_gateway_client::GatewayClient,
    pub context: AgentAppsContext,
    pub catalog_context: Option<AgentAppsContext>,
    pub project: String,
    pub host: String,
    pub generation: u64,
    pub token: String,
}

/// Product feedback keeps source templates separate from raw service data.
#[derive(Clone, Debug)]
pub enum Notice {
    Error(desktop_i18n::format::Message),
    Success(desktop_i18n::format::Message),
}
/// A product source or an opaque value supplied by a service.
pub struct Feedback(desktop_i18n::format::Message);
impl Feedback {
    pub fn into_message(self) -> desktop_i18n::format::Message { self.0 }
}
impl From<&'static str> for Feedback {
    fn from(source: &'static str) -> Self {
        Self(desktop_i18n::format::Message::new(source, vec![]))
    }
}
impl From<String> for Feedback {
    fn from(raw: String) -> Self { Self(desktop_i18n::format::Message::literal(raw)) }
}
impl From<desktop_i18n::format::Message> for Feedback {
    fn from(message: desktop_i18n::format::Message) -> Self { Self(message) }
}
fn error<E>(notice: &mut Option<Notice>, text: impl Into<Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Error(text.into().0)); iced::Task::none()
}
fn success<E>(notice: &mut Option<Notice>, text: impl Into<Feedback>) -> iced::Task<E> {
    *notice = Some(Notice::Success(text.into().0)); iced::Task::none()
}

pub mod aggregation;
pub mod diagnostics;

pub mod agent_apps_diagnostics;
pub mod agent_apps_high_availability;

pub mod app_routing;
pub mod agent_apps_auth;

pub mod agent_apps_high_availability_policy;
pub mod failover_queue;
pub mod agent_apps_health;
pub mod shiju_apps;
pub mod bookmark_modals;
pub mod apps_view;
pub mod catalog_state;

pub mod apps_tiles;
pub mod apps_ui;

pub mod bookmark;

pub mod web;

#[cfg(test)]
mod desktop_split_13_tests;

#[cfg(test)]
mod desktop_split_17_tests;

#[cfg(test)]
mod host_module_28_tests;

#[cfg(test)]
mod host_module_35_tests;

#[cfg(test)]
mod host_module_38_tests;

#[cfg(test)]
mod host_module_39_locale_tests;
