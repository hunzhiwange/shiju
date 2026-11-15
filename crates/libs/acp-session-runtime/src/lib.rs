//! 会话运行编排与队列所有者环境入口。

use acp_types::{error, errors, prompt_content, session_event_log, types};
use acp_access_policy::session_mode_preference;
use acp_config::mcp_servers;
use acp_metrics::{perf_metrics, perf_metrics_capture};
use acp_lease::queue_lease_store;
use acp_queue::{queue_ipc, queue_ipc_health, queue_ipc_server, queue_messages, queue_owner_turn_controller};
use acp_storage::{session_conversation_model, session_events, session_persistence};
use acp_client::AcpClient;
use acp_types::{PromptRequest, SessionStrategy, is_retryable_prompt_error};
use acp_access_policy::session_mode_preference::sync_advertised_model_state;
use acp_prompt_runner::set_session_model;
use acp_queue::QUEUE_CONNECT_RETRY_MS;
use acp_types::{AcpxOperationalError, OutputErrorCode, SessionAcpxState};
#[cfg(test)]
use acp_types::{AuthPolicy, NonInteractivePermissionPolicy, PermissionMode, SESSION_RECORD_SCHEMA, SessionRecord, parse_prompt_source, text_prompt};
#[cfg(test)]
use acp_owner_process::QUEUE_OWNER_PAYLOAD_ENV;

pub mod runtime;
mod queue_owner_env;
pub use runtime::*;
pub use queue_owner_env::{QueueOwnerPayloadError, QueueOwnerRunFromEnvError, parse_queue_owner_payload, queue_owner_runtime_options_from_env, run_queue_owner_from_env};

#[cfg(test)]
mod queue_owner_env_tests;

#[cfg(test)]
mod queue_paths {
    pub(crate) static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
}
