mod repos_api;

mod search_api;
pub use search_api::MarketSearchError;

#[cfg(test)]
mod search_api_tests;

mod claude_api;
#[cfg(test)]
mod claude_api_tests;

mod discovery_api;
pub use discovery_api::SkillDiscoveryError;
#[cfg(test)]
mod discovery_api_tests;

mod inventory_api;
pub use inventory_api::SkillInventoryError;

mod install_api;
pub use install_api::SkillInstallError;
#[cfg(test)]
mod install_api_tests;

mod import_api;
pub use import_api::SkillImportError;

mod jobs_api;
pub use jobs_api::SkillJobError;

mod updates_api;
pub use updates_api::SkillUpdateError;
#[cfg(test)]
mod updates_api_tests;

#[cfg(test)]
mod jobs_api_tests;

mod bindings_api;
pub use bindings_api::SkillBindingError;

mod uninstall_api;
pub use uninstall_api::SkillUninstallError;

#[cfg(test)]
mod uninstall_api_tests;

mod storage_api;
pub use storage_api::SkillStorageError;

mod backups_api;
pub use backups_api::SkillBackupError;

#[cfg(test)]
mod backups_api_tests;
