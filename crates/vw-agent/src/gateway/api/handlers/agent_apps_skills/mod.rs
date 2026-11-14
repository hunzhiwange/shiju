pub(crate) mod repos_api;

pub(crate) mod search_api;

#[cfg(test)]
mod search_api_tests;

pub(crate) mod claude_api;
#[cfg(all(test, target_os = "macos"))]
mod claude_api_tests;

pub(crate) mod discovery_api;
#[cfg(test)]
mod discovery_api_tests;

pub(crate) mod inventory_api;

pub(crate) mod install_api;
#[cfg(all(test, target_os = "macos"))]
mod install_api_tests;

pub(crate) mod import_api;

pub(crate) mod updates_api;

pub(crate) mod jobs_api;

#[cfg(all(test, target_os = "macos"))]
mod jobs_api_tests;

#[cfg(all(test, target_os = "macos"))]
mod updates_api_tests;

pub(crate) mod bindings_api;

pub(crate) mod uninstall_api;

#[cfg(all(test, target_os = "macos"))]
mod uninstall_api_tests;

pub(crate) mod storage_api;

#[cfg(all(test, target_os = "macos"))]
mod macos_api_tests;

pub(crate) mod backups_api;

#[cfg(all(test, target_os = "macos"))]
mod backups_api_tests;

#[cfg(all(test, target_os = "macos"))]
mod macos_files_tests;
