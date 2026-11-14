//! Gateway-owned device synchronization coordination.

pub mod operation_lock;

pub mod webdav_settings;

pub mod s3_settings;

#[cfg(target_os = "macos")]
pub mod export;

#[cfg(target_os = "macos")]
pub mod import;

#[cfg(target_os = "macos")]
pub mod apply;

#[cfg(target_os = "macos")]
pub mod webdav_upload;

#[cfg(target_os = "macos")]
pub mod projection;

#[cfg(target_os = "macos")]
pub mod s3_upload;

#[cfg(target_os = "macos")]
pub mod s3_download;

#[cfg(target_os = "macos")]
pub mod webdav_download;

pub mod change_events;

pub mod tasks;

pub mod cancel;
