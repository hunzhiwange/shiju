//! Desktop configuration access with an explicitly supplied endpoint and local path.
pub mod access;
mod config_agent;
pub use config_agent::*;
use gateway_endpoint::GatewayEndpoint;
use vw_config_types::config::Config;
#[derive(Clone)]
pub struct ConfigAccess {
    endpoint: GatewayEndpoint,
    local_path: Option<std::path::PathBuf>,
}
impl ConfigAccess {
    pub fn new(endpoint: GatewayEndpoint, local_path: Option<std::path::PathBuf>) -> Self { Self { endpoint, local_path } }
    fn gateway_client(&self) -> Result<vw_gateway_client::GatewayClient, String> { vw_gateway_client::GatewayClient::new(self.endpoint.clone()) }
    fn local_config(&self) -> Option<Config> {
        let path = self.local_path.as_ref()?;
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }
}
pub struct ConfigSaved { pub tag: &'static str, pub result: Result<(), String> }
#[cfg(not(target_arch="wasm32"))]
fn spawn_gateway_task(tag: &'static str, future: impl std::future::Future<Output=Result<(), String>> + Send + 'static) -> iced::Task<ConfigSaved> {
    iced::Task::perform(future, move |result| ConfigSaved { tag, result })
}
#[cfg(target_arch="wasm32")]
fn spawn_gateway_task(tag: &'static str, future: impl std::future::Future<Output=Result<(), String>> + 'static) -> iced::Task<ConfigSaved> {
    iced::Task::perform(future, move |result| ConfigSaved { tag, result })
}

pub mod config_cron_jobs;
#[cfg(not(target_arch = "wasm32"))]
pub mod config_cli_bootstrap;
