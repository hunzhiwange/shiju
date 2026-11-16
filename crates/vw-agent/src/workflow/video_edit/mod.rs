mod cloud;
pub(crate) mod cloud_failure;
pub use cloud::ShijuCloudVideoToolProvider;
mod command;
mod types;
pub use command::{parse_video_edit_command, prepare_video_edit_request};
pub use types::{VideoEditCommand, VideoEditRequest};
#[cfg(test)]
mod command_tests;

#[cfg(test)]
mod cloud_tests;

#[cfg(test)]
mod cloud_prepare_tests;
pub mod manifest;
pub mod prepare;
#[cfg(test)]
mod prepare_tests;
pub mod segments;

pub mod transcript;
#[cfg(test)]
mod transcript_tests;

pub use super::budget;

pub mod assets;
#[cfg(test)]
mod cloud_assets_tests;
pub mod merge;

pub mod planning;
#[cfg(test)]
mod planning_tests;

pub mod apply;
mod apply_journal;

pub mod preview;

pub use apply_journal::VideoProjectJournal;


pub use cloud::ShijuVideoProjectCloud;
#[cfg(test)]
mod cloud_delivery_tests;


pub(crate) mod entry;
mod entry_contract;
pub const DEFAULT_VIDEO_EDIT_YAML: &str = entry_contract::DEFAULT_YAML;

pub(crate) fn validate_entry_contract(source: &str) -> Result<(), String> {
    entry_contract::validate(source).map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) mod entry_test_transport;

#[cfg(test)]
mod e2e_tests;
