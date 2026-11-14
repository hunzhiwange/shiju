pub(crate) mod backup_crud_api;
#[cfg(target_os = "macos")]
mod backup_capture;
#[cfg(all(test, target_os = "macos"))]
mod backup_crud_api_tests;

#[cfg(target_os = "macos")]
pub(crate) mod periodic;

pub(crate) mod migration_read_api;

#[cfg(target_os = "macos")]
pub(crate) mod migration_write_api;

#[cfg(all(test, target_os = "macos"))]
mod migration_write_api_tests;

#[cfg(target_os = "macos")]
pub(crate) mod data_aux_api;

#[cfg(all(test, target_os = "macos"))]
mod data_aux_api_tests;

#[cfg(target_os = "macos")]
pub(crate) mod backup_restore_api;
#[cfg(all(test, target_os = "macos"))]
mod backup_restore_api_tests;
