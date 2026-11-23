pub mod backup_crud_api;
#[cfg(target_os = "macos")]
mod backup_capture;
#[cfg(all(test, target_os = "macos"))]
mod backup_crud_api_tests;

#[cfg(target_os = "macos")]
pub mod periodic;

#[cfg(target_os = "macos")]
pub mod data_aux_api;

#[cfg(all(test, target_os = "macos"))]
mod data_aux_api_tests;

#[cfg(target_os = "macos")]
pub mod backup_restore_api;
#[cfg(all(test, target_os = "macos"))]
mod backup_restore_api_tests;

#[cfg(all(test, target_os = "macos"))]
mod backup_acceptance_tests;
