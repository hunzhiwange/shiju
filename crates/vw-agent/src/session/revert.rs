//! Shared revert orchestration with the existing metadata storage binding.
pub use session_engine::revert::{Error, RevertInput};
#[cfg(test)]
use session_engine::revert::{collect_revert_plan, collect_cleanup_messages, worktree_path, instance_directory_opt, extra};
use super::session_io::NativeSessionIo;
pub async fn revert(input: RevertInput) -> Result<session_info::Info, Error> { session_engine::revert::revert::<NativeSessionIo>(input).await }
pub async fn unrevert(id: &str) -> Result<session_info::Info, Error> { session_engine::revert::unrevert::<NativeSessionIo>(id).await }
pub async fn cleanup(info: &session_info::Info) -> Result<(), Error> { session_engine::revert::cleanup::<NativeSessionIo>(info).await }
pub use session_engine::revert::{cleanup_from_message, cleanup_after_message};
#[cfg(test)]
#[path = "../../../libs/session-engine/tests/host_suites/revert_tests.rs"]
mod revert_tests;
