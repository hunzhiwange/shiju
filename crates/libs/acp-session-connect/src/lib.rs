//! ACP 连接建立、会话恢复与偏好重放。

pub mod connect_load;
pub use connect_load::{
    ConnectAndLoadClient, ConnectAndLoadClientSession, ConnectAndLoadSessionError,
    ConnectAndLoadSessionOptions, ConnectAndLoadSessionResult, connect_and_load_session,
};

#[cfg(test)]
mod connect_load_tests;
