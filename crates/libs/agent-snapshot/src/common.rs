//! 快照模块公共类型与日志辅助。
//!
//! 该文件只放置跨平台共享的数据结构、错误类型以及日志字段构造逻辑，
//! 避免将平台相关实现细节和公共接口耦合在同一个文件中。

#[cfg(not(target_arch = "wasm32"))]
use agent_log as log;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
use std::process::Output;
use std::sync::LazyLock;

/// Git 垃圾回收的默认修剪时间。
///
/// 设置为 7 天，意味着超过 7 天的未引用对象将被清理。
#[cfg(not(target_arch = "wasm32"))]
pub(super) const PRUNE: &str = "7.days";

/// 模块专用的日志记录器。
///
/// 使用 snapshot 作为服务标签，便于在日志中识别快照相关操作。
#[cfg(not(target_arch = "wasm32"))]
pub(super) static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("snapshot".to_string()));
    log::create(Some(tags))
});

/// 快照操作错误类型。
///
/// 封装了快照操作中可能遇到的各种错误情况。
#[derive(Debug)]
pub enum Error {
    /// I/O 错误（文件系统操作失败）
    Io(std::io::Error),
    /// UTF-8 转换错误（Git 输出无法解析为 UTF-8 字符串）
    Utf8(std::string::FromUtf8Error),
    /// Git 操作错误（包含错误描述信息）
    Git(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{}", e),
            Error::Utf8(e) => write!(f, "{}", e),
            Error::Git(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Error::Utf8(value)
    }
}

/// 文件补丁信息。
///
/// 表示一个快照中包含的文件变更集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Git 树对象的哈希值，唯一标识该快照。
    pub hash: String,
    /// 该快照中发生变更的文件列表（绝对路径）。
    pub files: Vec<String>,
}

/// 创建日志额外字段映射。
///
/// 将键值对数组转换为 JSON 对象映射，用于日志记录。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    map
}

/// 从命令输出创建日志额外字段映射。
///
/// 在基础字段上添加命令输出的详细信息（退出码、标准输出、标准错误）。
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn extra_from_output<const N: usize>(
    out: &Output,
    base: [(&'static str, Value); N],
) -> Map<String, Value> {
    let mut map = extra(base);
    map.insert("exit_code".to_string(), Value::Number(out.status.code().unwrap_or(-1).into()));
    map.insert(
        "stdout".to_string(),
        Value::String(String::from_utf8_lossy(&out.stdout).to_string()),
    );
    map.insert(
        "stderr".to_string(),
        Value::String(String::from_utf8_lossy(&out.stderr).to_string()),
    );
    map
}
#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
