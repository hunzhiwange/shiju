//! 会话操作错误契约。

use std::fmt;
use ordered_id as id;
use storage_types as storage;

/// 会话操作错误类型
///
/// 定义了会话模块可能返回的各种错误。
#[derive(Debug)]
pub enum Error {
    /// ID 生成错误
    Id(id::Error),
    /// 存储操作错误
    Storage(storage::Error),
    /// JSON 序列化/反序列化错误
    Json(serde_json::Error),
    /// 无活动项目上下文
    NoProjectContext,
    /// 异步运行时与同步存储之间的隔离执行失败
    Runtime(anyhow::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Id(e) => write!(f, "{}", e),
            Error::Storage(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
            Error::NoProjectContext => write!(f, "no active project context"),
            Error::Runtime(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<id::Error> for Error {
    fn from(value: id::Error) -> Self {
        Error::Id(value)
    }
}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Error::Storage(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Error::Runtime(value)
    }
}
