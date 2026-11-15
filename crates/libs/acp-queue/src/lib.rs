//! ACP 队列通信、健康检查与所有者控制。

pub mod queue_messages;
pub mod queue_ipc_transport;
pub mod queue_ipc_health;
pub mod queue_ipc;
pub mod queue_ipc_server;
pub mod queue_owner_turn_controller;

pub use queue_messages::*;
pub use queue_ipc_transport::*;
pub use queue_ipc_health::*;
pub use queue_ipc::*;
pub use queue_ipc_server::*;
pub use queue_owner_turn_controller::*;

#[cfg(test)]
mod queue_paths {
    pub use acp_paths::queue_paths::*;

    pub(crate) static TEST_HOME_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
}
