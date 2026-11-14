//! Native skill discovery and installation business rules.

#[cfg(target_os = "macos")]
pub mod claude_status;

#[cfg(target_os = "macos")]
pub mod claude_onboarding;

#[cfg(all(test, target_os = "macos"))]
mod claude_onboarding_tests;

#[cfg(target_os = "macos")]
pub mod claude_toggle;

#[cfg(all(test, target_os = "macos"))]
mod claude_toggle_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_status_tests;

pub mod repos;

pub mod market_search;

pub mod archive;

#[cfg(target_os = "macos")]
pub mod local_install;

#[cfg(all(test, target_os = "macos"))]
mod local_install_tests;

#[cfg(test)]
mod repos_tests;

#[cfg(test)]
mod market_search_tests;

pub mod app_paths;

#[cfg(test)]
mod app_paths_tests;

pub mod discovery;

#[cfg(test)]
mod discovery_tests;

#[cfg(target_os = "macos")]
pub mod repo_install;

pub mod discovery_cache;


#[cfg(target_os = "macos")]
pub mod zip_install;

#[cfg(all(test, target_os = "macos"))]
mod repo_install_tests;

#[cfg(all(test, target_os = "macos"))]
mod zip_install_tests;

#[cfg(target_os = "macos")]
pub mod inventory;

#[cfg(all(test, target_os = "macos"))]
mod inventory_tests;

#[cfg(target_os = "macos")]
pub mod copy_deploy;

#[cfg(all(test, target_os = "macos"))]
mod copy_deploy_tests;

#[cfg(target_os = "macos")]
pub mod link_deploy;

#[cfg(target_os = "macos")]
pub mod unmanaged_scan;

#[cfg(target_os = "macos")]
pub mod update_check;

#[cfg(all(test, target_os = "macos"))]
mod update_check_tests;

#[cfg(all(test, target_os = "macos"))]
mod link_deploy_tests;

#[cfg(all(test, target_os = "macos"))]
mod unmanaged_scan_tests;

pub mod jobs;

#[cfg(test)]
mod jobs_tests;

#[cfg(target_os = "macos")]
pub mod unmanaged_import;

#[cfg(all(test, target_os = "macos"))]
mod unmanaged_import_tests;

#[cfg(target_os = "macos")]
pub mod disable_deploy;

#[cfg(all(test, target_os = "macos"))]
mod disable_deploy_tests;

#[cfg(target_os = "macos")]
pub mod update_apply;

#[cfg(all(test, target_os = "macos"))]
mod update_apply_tests;

pub mod job_cancel;
#[cfg(test)]
mod job_cancel_tests;

#[cfg(target_os = "macos")]
pub mod bulk_sync;

#[cfg(target_os = "macos")]
pub mod uninstall;

#[cfg(all(test, target_os = "macos"))]
mod uninstall_tests;

#[cfg(target_os = "macos")]
pub mod storage_move;

#[cfg(target_os = "macos")]
pub mod backup_catalog;

#[cfg(all(test, target_os = "macos"))]
mod storage_move_tests;

#[cfg(target_os = "macos")]
pub mod backup_restore;

#[cfg(all(test, target_os = "macos"))]
mod backup_restore_tests;
