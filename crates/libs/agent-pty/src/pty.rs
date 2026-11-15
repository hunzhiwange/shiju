//! PTY（伪终端）模块
//!
//! 本模块提供伪终端会话管理功能，允许在后台创建、管理和交互终端会话。
//! 主要用于代理运行时环境中执行交互式命令行程序。
//!
//! # 核心功能
//!
//! - 创建新的终端会话
//! - 列出和查询现有会话
//! - 向终端会话写入数据
//! - 从终端会话读取输出
//! - 调整终端窗口大小
//! - 删除和清理会话
//!
//! # 事件系统
//!
//! 会话状态变化会通过事件总线发布：
//! - `pty.created`: 会话创建
//! - `pty.updated`: 会话更新
//! - `pty.exited`: 进程退出
//! - `pty.deleted`: 会话删除
//! - `pty.data`: 终端输出数据

use agent_bus as bus;
use ordered_id as id;
use crate::host as instance;
use shell_command as shell;
use agent_log as log;
use portable_pty::{CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

/// PTY 模块的日志记录器实例
///
/// 使用 "pty" 作为服务标识符，用于记录模块内的操作日志
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("pty".to_string()));
        m
    }))
});

/// 终端输出缓冲区的最大大小限制（2MB）
///
/// 超过此限制后，旧的输出数据将被丢弃以保持缓冲区大小在限制内
const BUFFER_LIMIT: usize = 1024 * 1024 * 2;

/// PTY 相关事件定义模块
///
/// 定义了 PTY 会话生命周期中发布的各种事件类型
pub mod event {
    use agent_bus as bus;

    /// 会话创建事件：当新的 PTY 会话成功创建时触发
    pub const CREATED: bus::Definition = bus::Definition { r#type: "pty.created" };

    /// 会话更新事件：当会话信息（如标题或窗口大小）更新时触发
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "pty.updated" };

    /// 进程退出事件：当 PTY 中运行的进程退出时触发
    pub const EXITED: bus::Definition = bus::Definition { r#type: "pty.exited" };

    /// 会话删除事件：当 PTY 会话被删除时触发
    pub const DELETED: bus::Definition = bus::Definition { r#type: "pty.deleted" };

    /// 数据输出事件：当终端有新的输出数据时触发
    pub const DATA: bus::Definition = bus::Definition { r#type: "pty.data" };
}

/// PTY 模块的错误类型
///
/// 封装了 PTY 操作中可能出现的各种错误情况
#[derive(Debug)]
pub enum Error {
    /// 无效的操作或参数错误
    Invalid(String),
    /// I/O 操作错误
    Io(std::io::Error),
    /// JSON 序列化/反序列化错误
    Json(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Invalid(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Json(e) => write!(f, "{}", e),
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

/// PTY 会话的运行状态
///
/// 表示终端会话当前的生命周期状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// 会话正在运行中
    Running,
    /// 进程已退出
    Exited,
}

/// PTY 会话的元数据信息
///
/// 包含会话的所有描述性信息，用于 API 响应和状态查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    /// 会话的唯一标识符
    pub id: String,
    /// 会话的显示标题
    pub title: String,
    /// 正在执行的命令
    pub command: String,
    /// 命令行参数列表
    pub args: Vec<String>,
    /// 当前工作目录
    pub cwd: String,
    /// 会话运行状态
    pub status: Status,
    /// 进程 ID
    pub pid: u32,
}

/// 创建 PTY 会话的输入参数
///
/// 用于指定新会话的配置选项，所有字段都是可选的
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateInput {
    /// 要执行的命令，默认使用系统默认 shell
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// 命令行参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// 工作目录，默认为项目实例目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// 会话标题，默认为 "Terminal {id后4位}"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// 额外的环境变量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// 更新 PTY 会话的输入参数
///
/// 用于修改现有会话的属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInput {
    /// 新的会话标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// 新的终端窗口大小
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
}

/// 终端窗口尺寸
///
/// 定义终端的行列数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    /// 行数
    pub rows: u16,
    /// 列数
    pub cols: u16,
}

/// 终端输出缓冲区状态
///
/// 内部结构，用于管理终端输出数据的存储和检索
/// 实现了环形缓冲区的逻辑，当数据超过限制时自动丢弃旧数据
struct BufferState {
    /// 存储的实际字节数据
    bytes: Vec<u8>,
    /// 缓冲区起始位置在全局数据流中的偏移量
    /// 当数据被丢弃时此值会增加
    buffer_cursor: usize,
    /// 全局写入位置的光标
    cursor: usize,
}

impl BufferState {
    /// 创建新的空缓冲区状态
    fn new() -> Self {
        Self { bytes: Vec::new(), buffer_cursor: 0, cursor: 0 }
    }

    /// 向缓冲区追加新数据
    ///
    /// 如果缓冲区超过大小限制，会自动丢弃最旧的数据
    ///
    /// # 参数
    ///
    /// * `chunk` - 要追加的数据块
    fn push(&mut self, chunk: &[u8]) {
        // 更新全局光标位置
        self.cursor = self.cursor.saturating_add(chunk.len());
        self.bytes.extend_from_slice(chunk);

        // 检查是否超过缓冲区限制
        if self.bytes.len() <= BUFFER_LIMIT {
            return;
        }

        // 计算需要丢弃的超出部分
        let excess = self.bytes.len() - BUFFER_LIMIT;
        // 从头部移除超出的数据
        self.bytes.drain(0..excess);
        // 更新缓冲区起始偏移量
        self.buffer_cursor = self.buffer_cursor.saturating_add(excess);
    }

    /// 从指定光标位置读取数据
    ///
    /// # 参数
    ///
    /// * `cursor` - 起始读取位置，-1 表示从最新位置开始
    ///
    /// # 返回值
    ///
    /// 返回元组 (读取的数据, 缓冲区起始位置, 当前结束位置)
    fn slice_from(&self, cursor: i64) -> (Vec<u8>, usize, usize) {
        let start = self.buffer_cursor;
        let end = self.cursor;

        // 计算实际读取起始位置
        let from = if cursor == -1 {
            // -1 表示从最新位置开始，返回空数据
            end
        } else if cursor >= 0 {
            // 正数表示绝对位置
            cursor as usize
        } else {
            // 其他负数视为从开头开始
            0
        };

        // 如果请求的起始位置已经超过当前写入位置，返回空
        if from >= end {
            return (Vec::new(), start, end);
        }

        // 计算在缓冲区内的偏移量
        let offset = from.saturating_sub(start);

        // 如果偏移量超出缓冲区范围，返回空
        if offset >= self.bytes.len() {
            return (Vec::new(), start, end);
        }

        // 返回从偏移位置到末尾的所有数据
        (self.bytes[offset..].to_vec(), start, end)
    }
}

/// PTY 会话的内部状态
///
/// 封装了单个终端会话的所有状态，包括 PTY 主端、写入器、子进程和输出缓冲区
struct Session {
    /// 会话的元数据信息
    info: Mutex<Info>,
    /// PTY 主端接口，用于调整终端大小等操作
    master: Mutex<Box<dyn MasterPty + Send>>,
    /// 终端输入写入器
    writer: Mutex<Box<dyn Write + Send>>,
    /// 子进程句柄
    child: Mutex<Box<dyn portable_pty::Child + Send>>,
    /// 输出数据缓冲区
    buffer: Mutex<BufferState>,
}

/// 会话存储类型：线程安全的哈希表
type Sessions = Arc<Mutex<HashMap<String, Arc<Session>>>>;

/// 获取项目实例级别的会话存储
///
/// 返回一个函数，该函数异步获取当前项目实例的会话存储。
/// 会话存储是项目级别的，每个项目实例有独立的会话集合。
///
/// # 清理逻辑
///
/// 当项目实例被销毁时，会自动终止所有活跃的会话
fn instance_sessions() -> impl Fn() -> crate::host::BoxFuture<Arc<State>> {
    || instance::state()
}

/// 项目实例持有的终端状态；释放时由宿主调用 `dispose`。
#[derive(Default)]
pub struct State {
    sessions: Sessions,
}

impl State {
    /// 终止并移除该项目的全部会话。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispose(&self) {
        let entries = {
            let mut lock = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            let values = lock.values().cloned().collect::<Vec<_>>();
            lock.clear();
            values
        };
        for session in entries {
            let _ = kill_session(&session);
        }
    }
}

/// 列出所有 PTY 会话（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，始终返回空列表
#[cfg(target_arch = "wasm32")]
pub async fn list() -> Vec<Info> {
    Vec::new()
}

/// 列出所有 PTY 会话
///
/// 返回当前项目实例中所有活跃会话的信息列表
///
/// # 返回值
///
/// 包含所有会话元数据的向量
#[cfg(not(target_arch = "wasm32"))]
pub async fn list() -> Vec<Info> {
    let sessions = instance_sessions()().await;
    let lock = sessions.sessions.lock().unwrap_or_else(|e| e.into_inner());
    lock.values().filter_map(|s| s.info.lock().ok().map(|i| i.clone())).collect::<Vec<_>>()
}

/// 获取指定会话的信息（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，始终返回 None
#[cfg(target_arch = "wasm32")]
pub async fn get(_id: &str) -> Option<Info> {
    None
}

/// 获取指定会话的信息
///
/// 根据会话 ID 查找并返回会话的元数据
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
///
/// # 返回值
///
/// 如果找到会话则返回 Some(Info)，否则返回 None
#[cfg(not(target_arch = "wasm32"))]
pub async fn get(id: &str) -> Option<Info> {
    let sessions = instance_sessions()().await;
    let lock = sessions.sessions.lock().ok()?;
    let s = lock.get(id)?.clone();
    s.info.lock().ok().map(|i| i.clone())
}

/// 创建新的 PTY 会话（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，返回错误
#[cfg(target_arch = "wasm32")]
pub async fn create(_input: CreateInput) -> Result<Info, Error> {
    Err(Error::Invalid("pty 在 Web 版本不可用".to_string()))
}

/// 创建新的 PTY 会话
///
/// 根据输入参数创建并启动一个新的终端会话。会话会启动一个子进程，
/// 并在后台线程中持续读取其输出。
///
/// # 参数
///
/// * `input` - 创建参数，包括命令、参数、工作目录等
///
/// # 返回值
///
/// 成功返回会话信息，失败返回错误
///
/// # 环境变量
///
/// 会话会继承当前进程的所有环境变量，并额外设置：
/// - `TERM=xterm-256color`: 终端类型
/// - `VIBEWINDOW_TERMINAL=1`: 标识在 VibeWindow 终端中运行
///
/// # 事件
///
/// 创建成功后会发布 `pty.created` 事件
#[cfg(not(target_arch = "wasm32"))]
pub async fn create(input: CreateInput) -> Result<Info, Error> {
    // 生成唯一的会话 ID
    let id = id::create(id::Prefix::Pty, false, None).map_err(|e| Error::Invalid(e.to_string()))?;

    // 获取或确定要执行的命令
    let mut command = input.command.unwrap_or_else(|| shell::ACCEPTABLE.clone());
    let mut args = input.args.unwrap_or_default();

    // 如果是 shell 命令，添加登录参数
    if command.ends_with("sh") {
        args.push("-l".to_string());
    }

    // 确保命令不为空
    if command.is_empty() {
        command = shell::ACCEPTABLE.clone();
    }

    // 确定工作目录
    let cwd = input.cwd.unwrap_or_else(|| instance::directory());
    if cwd.trim().is_empty() {
        return Err(Error::Invalid("missing cwd".to_string()));
    }

    // 构建环境变量：继承当前环境并添加额外变量
    let mut env: HashMap<String, String> = std::env::vars().collect();
    if let Some(extra) = input.env {
        for (k, v) in extra {
            env.insert(k, v);
        }
    }

    // 设置标准终端环境变量
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("VIBEWINDOW_TERMINAL".to_string(), "1".to_string());

    // Windows 平台额外设置区域设置
    #[cfg(windows)]
    {
        env.insert("LC_ALL".to_string(), "C.UTF-8".to_string());
        env.insert("LC_CTYPE".to_string(), "C.UTF-8".to_string());
        env.insert("LANG".to_string(), "C.UTF-8".to_string());
    }

    // 记录会话创建日志
    LOGGER.info(
        "creating session",
        Some({
            let mut m = Map::new();
            m.insert("id".to_string(), Value::String(id.clone()));
            m.insert("cmd".to_string(), Value::String(command.clone()));
            m.insert("cwd".to_string(), Value::String(cwd.clone()));
            m
        }),
    );

    // 创建 PTY 对（主端和从端）
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| Error::Invalid(e.to_string()))?;

    // 构建命令
    let mut cmd = CommandBuilder::new(command.clone());
    cmd.args(args.clone());
    cmd.cwd(PathBuf::from(cwd.clone()));
    for (k, v) in env {
        cmd.env(k, v);
    }

    // 在 PTY 从端启动命令
    let child = pair.slave.spawn_command(cmd).map_err(|e| Error::Invalid(format!("{e:?}")))?;
    drop(pair.slave); // 关闭从端句柄

    // 获取进程 ID
    let pid = child.process_id().unwrap_or(0);

    // 生成会话标题
    let title =
        input.title.unwrap_or_else(|| format!("Terminal {}", &id[id.len().saturating_sub(4)..]));

    // 构建会话信息
    let info = Info {
        id: id.clone(),
        title,
        command,
        args,
        cwd: cwd.clone(),
        status: Status::Running,
        pid,
    };

    // 获取 PTY 读写器
    let mut reader = pair.master.try_clone_reader().map_err(|e| Error::Invalid(e.to_string()))?;
    let writer = pair.master.take_writer().map_err(|e| Error::Invalid(e.to_string()))?;
    let master: Box<dyn MasterPty + Send> = pair.master;

    // 创建会话对象
    let session = Arc::new(Session {
        info: Mutex::new(info.clone()),
        master: Mutex::new(master),
        writer: Mutex::new(writer),
        child: Mutex::new(child),
        buffer: Mutex::new(BufferState::new()),
    });

    // 将会话添加到存储
    let sessions = instance_sessions()().await;
    sessions.sessions.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), session.clone());

    // 准备后台线程需要的变量
    let directory = cwd.clone();
    let id2 = id.clone();
    let session2 = session.clone();

    // 启动输出读取线程
    // 持续读取终端输出并推送到缓冲区和事件总线
    std::thread::spawn(move || {
        loop {
            let mut buf = [0u8; 8192];
            let n = match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(_) => break, // 读取错误
            };
            let chunk = &buf[..n];

            // 将数据推入缓冲区并发布数据事件
            if let Ok(mut b) = session2.buffer.lock() {
                b.push(chunk);
                let cursor = b.cursor;
                let text = String::from_utf8_lossy(chunk).to_string();
                let _ = bus::publish(
                    event::DATA,
                    serde_json::json!({ "id": id2, "data": text, "cursor": cursor }),
                    Some(directory.clone()),
                );
            }
        }
    });

    // 启动进程退出监控线程
    let sessions2 = sessions.clone();
    let id3 = id.clone();
    let directory2 = cwd.clone();
    std::thread::spawn(move || {
        // 等待进程退出
        let exit_code = wait_exit_code(&session);

        // 更新会话状态为已退出
        if let Ok(mut info) = session.info.lock() {
            info.status = Status::Exited;
        }

        // 发布退出事件
        let _ = bus::publish(
            event::EXITED,
            serde_json::json!({ "id": id3, "exitCode": exit_code }),
            Some(directory2.clone()),
        );

        // 从存储中移除会话
        sessions2.sessions.lock().unwrap_or_else(|e| e.into_inner()).remove(&id3);
    });

    // 发布创建事件并返回会话信息
    let _ = bus::publish(event::CREATED, serde_json::json!({ "info": info }), Some(cwd));
    Ok(info)
}

/// 更新 PTY 会话（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，始终返回 None
#[cfg(target_arch = "wasm32")]
pub async fn update(_id: &str, _input: UpdateInput) -> Result<Option<Info>, Error> {
    Ok(None)
}

/// 更新 PTY 会话
///
/// 修改现有会话的属性，如标题或终端窗口大小
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
/// * `input` - 更新参数
///
/// # 返回值
///
/// 成功返回 Some(更新后的会话信息)，会话不存在返回 None
///
/// # 事件
///
/// 更新成功后会发布 `pty.updated` 事件
#[cfg(not(target_arch = "wasm32"))]
pub async fn update(id: &str, input: UpdateInput) -> Result<Option<Info>, Error> {
    let sessions = instance_sessions()().await;
    let s = {
        let lock = sessions.sessions.lock().unwrap_or_else(|e| e.into_inner());
        lock.get(id).cloned()
    };
    let Some(session) = s else { return Ok(None) };

    // 更新标题（如果提供）
    if let Some(title) = input.title {
        if let Ok(mut info) = session.info.lock() {
            info.title = title;
        }
    }

    // 更新终端大小（如果提供）
    if let Some(size) = input.size {
        if let Ok(m) = session.master.lock() {
            let _ = m.resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    // 获取更新后的会话信息
    let info = session.info.lock().ok().map(|i| i.clone());
    if let Some(info) = info.clone() {
        let _ = bus::publish(event::UPDATED, serde_json::json!({ "info": info }), None);
    }
    Ok(info)
}

/// 删除 PTY 会话（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，始终返回 true
#[cfg(target_arch = "wasm32")]
pub async fn remove(_id: &str) -> Result<bool, Error> {
    Ok(true)
}

/// 删除 PTY 会话
///
/// 终止指定会话并从存储中移除
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
///
/// # 返回值
///
/// 成功返回 Ok(true)
///
/// # 事件
///
/// 删除成功后会发布 `pty.deleted` 事件
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove(id: &str) -> Result<bool, Error> {
    let sessions = instance_sessions()().await;
    let removed = sessions.sessions.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
    let Some(session) = removed else { return Ok(true) };

    // 记录删除日志
    LOGGER.info(
        "removing session",
        Some({
            let mut m = Map::new();
            m.insert("id".to_string(), Value::String(id.to_string()));
            m
        }),
    );

    // 终止会话进程
    let _ = kill_session(&session);

    // 发布删除事件
    let _ = bus::publish(event::DELETED, serde_json::json!({ "id": id }), None);
    Ok(true)
}

/// 调整终端窗口大小（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，无操作
#[cfg(target_arch = "wasm32")]
pub async fn resize(_id: &str, _cols: u16, _rows: u16) {}

/// 调整终端窗口大小
///
/// 便捷函数，用于调整指定会话的终端窗口大小
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
/// * `cols` - 新的列数
/// * `rows` - 新的行数
#[cfg(not(target_arch = "wasm32"))]
pub async fn resize(id: &str, cols: u16, rows: u16) {
    let _ = update(id, UpdateInput { title: None, size: Some(Size { rows, cols }) }).await;
}

/// 向终端会话写入数据（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，无操作
#[cfg(target_arch = "wasm32")]
pub async fn write(_id: &str, _data: &str) {}

/// 向终端会话写入数据
///
/// 将输入数据发送到指定会话的标准输入
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
/// * `data` - 要写入的字符串数据
///
/// # 注意
///
/// 如果会话状态不是 Running，则不会执行写入操作
#[cfg(not(target_arch = "wasm32"))]
pub async fn write(id: &str, data: &str) {
    let sessions = instance_sessions()().await;
    let sessions = match sessions.sessions.lock() {
        Ok(lock) => lock,
        Err(e) => e.into_inner(),
    };
    let Some(session) = sessions.get(id).cloned() else { return };

    // 检查会话是否仍在运行
    if let Ok(info) = session.info.lock() {
        if info.status != Status::Running {
            return;
        }
    }

    // 执行写入操作
    if let Ok(mut w) = session.writer.lock() {
        let _ = w.write_all(data.as_bytes());
        let _ = w.flush();
    }
}

/// 从终端会话读取数据（WebAssembly 版本）
///
/// Web 版本不支持 PTY 功能，始终返回 None
#[cfg(target_arch = "wasm32")]
pub async fn read(_id: &str, _cursor: i64) -> Option<(String, usize)> {
    None
}

/// 从终端会话读取数据
///
/// 从指定光标位置读取终端输出缓冲区中的数据
///
/// # 参数
///
/// * `id` - 会话的唯一标识符
/// * `cursor` - 起始读取位置
///   - `-1`: 从最新位置开始（返回空）
///   - `>= 0`: 从指定绝对位置开始
///
/// # 返回值
///
/// 返回 Some((读取到的文本, 新的光标位置))，如果会话不存在则返回 None
#[cfg(not(target_arch = "wasm32"))]
pub async fn read(id: &str, cursor: i64) -> Option<(String, usize)> {
    let sessions = instance_sessions()().await;
    let lock = sessions.sessions.lock().ok()?;
    let s = lock.get(id)?.clone();
    let buf = s.buffer.lock().ok()?;
    let (bytes, _start, end) = buf.slice_from(cursor);
    Some((String::from_utf8_lossy(&bytes).to_string(), end))
}

/// 终止会话进程
///
/// 向会话的子进程发送终止信号
///
/// # 参数
///
/// * `session` - 会话引用
///
/// # 返回值
///
/// 始终返回 true
#[cfg(not(target_arch = "wasm32"))]
fn kill_session(session: &Arc<Session>) -> bool {
    if let Ok(mut child) = session.child.lock() {
        let _ = child.kill();
    }
    true
}

/// 等待会话进程退出并获取退出码
///
/// 阻塞等待直到进程退出，然后返回其退出码
///
/// # 参数
///
/// * `session` - 会话引用
///
/// # 返回值
///
/// 进程退出码，如果无法获取则返回 -1
#[cfg(not(target_arch = "wasm32"))]
fn wait_exit_code(session: &Arc<Session>) -> i32 {
    let status = session.child.lock().ok().and_then(|mut c| c.wait().ok());
    status.map(|s| s.exit_code() as i32).unwrap_or(-1)
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod pty_tests;
