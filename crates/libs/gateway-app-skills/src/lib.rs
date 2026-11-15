mod repos_api;

mod search_api;
pub use search_api::MarketSearchError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod search_api_tests;

mod claude_api;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod claude_api_tests;

mod discovery_api;
pub use discovery_api::SkillDiscoveryError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod discovery_api_tests;

mod inventory_api;
pub use inventory_api::SkillInventoryError;

mod install_api;
pub use install_api::SkillInstallError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod install_api_tests;

mod import_api;
pub use import_api::SkillImportError;

mod jobs_api;
pub use jobs_api::SkillJobError;

mod updates_api;
pub use updates_api::SkillUpdateError;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod updates_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod jobs_api_tests;

mod bindings_api;
pub use bindings_api::SkillBindingError;

mod uninstall_api;
pub use uninstall_api::SkillUninstallError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod uninstall_api_tests;

mod storage_api;
pub use storage_api::SkillStorageError;

mod backups_api;
pub use backups_api::SkillBackupError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod backups_api_tests;

#[derive(Debug, Clone)]
pub struct GatewayAppSkillsClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayAppSkillsClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
