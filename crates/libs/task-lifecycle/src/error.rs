//! 存储服务错误；由宿主映射到对应的 HTTP 状态。

/// 保留原存储操作的请求无效、资源缺失与内部失败分类。
#[derive(Debug)]
pub enum TaskLifecycleError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl TaskLifecycleError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

impl std::fmt::Display for TaskLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::BadRequest(message) | Self::NotFound(message) | Self::Internal(message) => message,
        };
        f.write_str(message)
    }
}

impl std::error::Error for TaskLifecycleError {}
