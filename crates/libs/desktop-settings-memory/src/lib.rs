//! Memory, storage, backup and device synchronization settings.
pub mod system_settings_device_sync;
pub mod state;
pub mod messages;
pub mod fields;
pub mod system_settings_memory;
pub mod system_settings_storage;
#[cfg(test)]
mod fields_tests;
pub mod backup_inventory;
pub mod system_settings_data;
#[cfg(test)]
mod storage_tests;
mod memory_tabs;
#[cfg(test)]
mod view_test_support;
#[cfg(test)]
mod system_settings_memory_tests;
#[cfg(test)]
mod system_settings_storage_tests;

pub mod app_init;

pub mod data_management;
pub mod device_sync_s3;
pub mod device_sync_webdav;
pub mod memory_update;
pub mod storage_update;
#[cfg(test)]
mod memory_update_tests;

pub mod save_feedback;

#[cfg(test)]
mod desktop_split_02_tests;
