//! ACP 会话记录、索引、事件日志与对话模型存储。

#![allow(clippy::result_large_err)]

use acp_types::*;
use acp_metrics::perf_metrics::{increment_perf_counter, measure_perf};
use acp_lease::queue_lease_store;
use acp_lease::terminate_process;
use acp_paths::queue_paths::default_home_dir;

#[cfg(test)]
mod queue_paths {
    pub(crate) static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
}

pub mod persisted_key_policy;
pub mod session_conversation_model;
pub mod session_events;
pub mod session_persistence;

pub use persisted_key_policy::*;
pub use session_conversation_model::*;
pub use session_events::*;
pub use session_persistence::*;
