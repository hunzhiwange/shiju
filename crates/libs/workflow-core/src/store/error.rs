//! 工作流持久化错误；协议状态由调用方映射。
#[derive(Debug, thiserror::Error)]
pub enum WorkflowStoreError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
}

impl WorkflowStoreError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}
