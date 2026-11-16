pub mod repos_api;

pub mod search_api;

#[cfg(test)]
mod search_api_tests;

pub mod claude_api;
#[cfg(all(test, target_os = "macos"))]
mod claude_api_tests;

pub mod discovery_api;
#[cfg(test)]
mod discovery_api_tests;

pub mod inventory_api;

pub mod install_api;
#[cfg(all(test, target_os = "macos"))]
mod install_api_tests;

pub mod import_api;

pub mod updates_api;

pub mod jobs_api;

#[cfg(all(test, target_os = "macos"))]
mod jobs_api_tests;

#[cfg(all(test, target_os = "macos"))]
mod updates_api_tests;

pub mod bindings_api;

pub mod uninstall_api;

#[cfg(all(test, target_os = "macos"))]
mod uninstall_api_tests;

pub mod storage_api;

#[cfg(all(test, target_os = "macos"))]
mod macos_api_tests;

pub mod backups_api;

#[cfg(all(test, target_os = "macos"))]
mod backups_api_tests;

#[cfg(all(test, target_os = "macos"))]
mod macos_files_tests;
