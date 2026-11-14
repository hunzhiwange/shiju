//! Device synchronization primitives; authorization and persistence belong to Gateway.

pub mod s3_config;
pub mod artifact;
pub mod webdav_config;
pub mod manifest;
pub mod archive_pack;
pub mod archive_extract;
pub mod webdav_url;
pub mod s3_signing;
pub mod webdav_transport;
pub mod conflict;
pub mod s3_transport;
pub mod webdav_remote;
pub mod s3_remote;

#[cfg(target_os = "macos")]
pub mod skill_update;
