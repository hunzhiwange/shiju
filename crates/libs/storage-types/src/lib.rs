//! 轻量存储错误契约。

use std::fmt;

#[derive(Debug, Clone)]
/// 表示指定存储资源不存在。
pub struct NotFoundError {
    /// 面向调用方的错误说明。
    pub message: String,
}

impl fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NotFoundError {}

#[derive(Debug)]
/// 存储层统一错误类型。
pub enum Error {
    /// key 对应的 JSON 资源不存在。
    NotFound(NotFoundError),
    /// 文件系统读写错误。
    Io(std::io::Error),
    /// JSON 序列化或反序列化错误。
    Json(serde_json::Error),
    /// 阻塞任务 join 失败。
    Join(tokio::task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Join(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(value: tokio::task::JoinError) -> Self {
        Error::Join(value)
    }
}
