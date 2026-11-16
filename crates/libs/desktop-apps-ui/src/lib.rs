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

pub enum Notice { Error(String), Success(String) }
fn error<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Error(text.into())); iced::Task::none()
}
fn success<E>(notice: &mut Option<Notice>, text: impl Into<String>) -> iced::Task<E> {
    *notice = Some(Notice::Success(text.into())); iced::Task::none()
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
