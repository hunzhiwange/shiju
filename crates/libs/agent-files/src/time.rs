//! 记录会话内文件读取时间，并在写入前校验文件是否已被外部修改。
//!
//! 该模块用于支撑文件编辑工具的“先读后写”约束：每个会话读取文件后会记录
//! 时间戳，写入前再比较文件系统修改时间，避免覆盖用户或其他进程的更新。

use runtime_flags;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[cfg(test)]
#[path = "time_tests.rs"]
mod time_tests;

static READ: LazyLock<Mutex<HashMap<String, HashMap<String, SystemTime>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static LOCKS: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static LOCKS_SYNC: LazyLock<Mutex<HashMap<String, Arc<std::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 记录指定会话刚读取过某个文件。
///
/// # 参数
///
/// - `session_id`: 当前代理会话标识。
/// - `file`: 已读取文件的规范化路径字符串。
pub fn read(session_id: &str, file: &str) {
    let mut lock = READ.lock().unwrap_or_else(|e| e.into_inner());
    let session = lock.entry(session_id.to_string()).or_default();
    session.insert(file.to_string(), SystemTime::now());
}

/// 返回指定会话读取某个文件时记录的时间。
///
/// # 参数
///
/// - `session_id`: 当前代理会话标识。
/// - `file`: 需要查询的文件路径字符串。
///
/// # 返回值
///
/// 找到读取记录时返回对应 `SystemTime`，否则返回 `None`。
pub fn get(session_id: &str, file: &str) -> Option<SystemTime> {
    READ.lock().ok().and_then(|m| m.get(session_id).and_then(|s| s.get(file)).copied())
}

/// 在异步文件级互斥锁内执行给定任务。
///
/// # 参数
///
/// - `filepath`: 用作锁 key 的文件路径。
/// - `f`: 需要串行执行的异步任务。
///
/// # 返回值
///
/// 返回异步任务的执行结果。
pub async fn with_lock<T, F>(filepath: &str, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let mutex = {
        let mut lock = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
        lock.entry(filepath.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    let guard = mutex.lock().await;
    let out = f.await;
    drop(guard);
    out
}

/// 在同步文件级互斥锁内执行闭包。
///
/// # 参数
///
/// - `filepath`: 用作锁 key 的文件路径。
/// - `f`: 需要串行执行的同步闭包。
///
/// # 返回值
///
/// 返回闭包的执行结果。
pub fn with_lock_sync<T>(filepath: &str, f: impl FnOnce() -> T) -> T {
    let mutex = {
        let mut lock = LOCKS_SYNC.lock().unwrap_or_else(|e| e.into_inner());
        lock.entry(filepath.to_string())
            .or_insert_with(|| Arc::new(std::sync::Mutex::new(())))
            .clone()
    };
    let guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    let out = f();
    drop(guard);
    out
}

/// 写入前文件时间校验错误。
#[derive(Debug)]
pub enum Error {
    /// 当前会话没有读取记录，不能直接覆盖文件。
    MustReadFirst { filepath: String },
    /// 文件在上次读取后又被修改，继续写入可能覆盖外部变更。
    ModifiedSinceRead { filepath: String },
    /// 读取文件元数据时遇到的底层 I/O 错误。
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MustReadFirst { filepath } => write!(
                f,
                "You must read file {} before overwriting it. Use the Read tool first",
                filepath
            ),
            Error::ModifiedSinceRead { filepath } => write!(
                f,
                "File {} has been modified since it was last read.\n\nPlease read the file again before modifying it.",
                filepath
            ),
            Error::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

/// 异步校验文件在当前会话读取后未被修改。
///
/// # 参数
///
/// - `session_id`: 当前代理会话标识。
/// - `filepath`: 需要写入前校验的文件路径。
///
/// # 错误
///
/// 当文件未先读取、读取后被修改，或读取文件元数据失败时返回 `Error`。
pub async fn assert(session_id: &str, filepath: impl AsRef<Path>) -> Result<(), Error> {
    if *flag::VIBEWINDOW_DISABLE_FILETIME_CHECK {
        return Ok(());
    }

    let filepath = filepath.as_ref().to_path_buf();
    let filepath_str = filepath.to_string_lossy().to_string();
    let time = get(session_id, &filepath_str)
        .ok_or_else(|| Error::MustReadFirst { filepath: filepath_str.clone() })?;

    let stats = std::fs::metadata(&filepath)?;
    let mtime = stats.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if mtime > time {
        return Err(Error::ModifiedSinceRead { filepath: filepath_str });
    }
    Ok(())
}

/// 同步校验文件在当前会话读取后未被修改。
///
/// # 参数
///
/// - `session_id`: 当前代理会话标识。
/// - `filepath`: 需要写入前校验的文件路径。
///
/// # 错误
///
/// 当文件未先读取、读取后被修改，或读取文件元数据失败时返回 `Error`。
pub fn assert_sync(session_id: &str, filepath: impl AsRef<Path>) -> Result<(), Error> {
    if *flag::VIBEWINDOW_DISABLE_FILETIME_CHECK {
        return Ok(());
    }

    let filepath = filepath.as_ref().to_path_buf();
    let filepath_str = filepath.to_string_lossy().to_string();
    let time = get(session_id, &filepath_str)
        .ok_or_else(|| Error::MustReadFirst { filepath: filepath_str.clone() })?;

    let stats = std::fs::metadata(&filepath)?;
    let mtime = stats.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if mtime > time {
        return Err(Error::ModifiedSinceRead { filepath: filepath_str });
    }
    Ok(())
}

/// 将路径转换为用于内部索引的字符串形式。
///
/// # 参数
///
/// - `path`: 待转换路径。
///
/// # 返回值
///
/// 返回平台路径的 lossy 字符串表示。
pub fn normalize(path: impl AsRef<Path>) -> String {
    let p = path.as_ref();
    let p: PathBuf = if p.is_absolute() { p.to_path_buf() } else { p.to_path_buf() };
    p.to_string_lossy().to_string()
}
