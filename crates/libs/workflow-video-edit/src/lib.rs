//! Video editing contracts, durable assets and reconciled project writes.
pub mod apply;
pub mod apply_journal;
pub mod assets;
pub mod budget;
pub mod manifest;
pub mod merge;
pub mod preview;
pub mod segments;
pub mod types;
pub mod planning;
pub mod transcript;
pub use apply_journal::VideoProjectJournal;
#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod apply_test_cloud;
#[cfg(test)]
mod merge_tests;
#[cfg(test)]
mod segments_tests;
#[cfg(test)]
mod preview_tests;
pub mod context;
pub mod prepare;
pub mod cloud;
pub mod cloud_failure;
pub use cloud::{ShijuCloudVideoToolProvider, ShijuVideoProjectCloud};
pub mod command;
pub mod entry;

#[cfg(test)]
mod host_migration_44_tests;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
