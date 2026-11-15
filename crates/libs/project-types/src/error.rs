//! 项目操作错误契约。

use storage_types as storage;
use vw_api_transport::id::IdValidationError;

/// 项目模块错误类型
#[derive(Debug)]
pub enum Error {
    Storage(storage::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Join(tokio::task::JoinError),
    Database(anyhow::Error),
    Scope(IdValidationError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Storage(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::Join(e) => write!(f, "{}", e),
            Error::Database(e) => write!(f, "{}", e),
            Error::Scope(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Error::Storage(value)
    }
}

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

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Database(value)
    }
}

impl From<IdValidationError> for Error {
    fn from(value: IdValidationError) -> Self {
        Error::Scope(value)
    }
}
