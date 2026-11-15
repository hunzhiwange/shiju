//! Application skill installation and management contracts.

#[path = "repositories/repos_api.rs"]
pub mod repos_api;

#[path = "repositories/discovery_api.rs"]
pub mod discovery_api;

#[path = "repositories/search_api.rs"]
pub mod search_api;

#[path = "repositories/inventory_api.rs"]
pub mod inventory_api;

#[path = "distribution/bindings_api.rs"]
pub mod bindings_api;

#[path = "distribution/storage_api.rs"]
pub mod storage_api;

#[path = "installation/install_api.rs"]
pub mod install_api;

#[path = "installation/updates_api.rs"]
pub mod updates_api;

#[path = "installation/jobs_api.rs"]
pub mod jobs_api;

#[path = "installation/uninstall_api.rs"]
pub mod uninstall_api;

#[path = "maintenance/backups_api.rs"]
pub mod backups_api;

#[path = "maintenance/import_api.rs"]
pub mod import_api;

#[path = "maintenance/claude_api.rs"]
pub mod claude_api;
