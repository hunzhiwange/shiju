//! 日志模块 - 提供轻量级、结构化的日志记录功能
//!
//! 本模块实现了一个简单但功能完备的日志系统，支持以下特性：
//!
//! - **多日志级别**：支持 Debug、Info、Warn、Error 四个级别
//! - **灵活输出**：支持输出到标准错误流或文件
//! - **标签系统**：支持为日志添加结构化标签，便于过滤和检索
//! - **计时器**：提供 `Timer` 类型，自动记录操作耗时
//! - **日志轮转**：自动清理旧的日志文件，保留最近的记录
//!
//! # 示例
//!
//! ```rust,ignore
//! use crate::app::agent::util::log::{init, create, Level, InitOptions};
//!
//! // 初始化日志系统（输出到文件）
//! init(InitOptions {
//!     print: false,
//!     dev: false,
//!     level: Some(Level::Info),
//! });
//!
//! // 创建带有服务标签的日志记录器
//! let logger = create(Some({
//!     let mut m = serde_json::Map::new();
//!     m.insert("service".to_string(), serde_json::Value::String("my-app".to_string()));
//!     m
//! }));
//!
//! // 记录日志
//! logger.info("应用启动", None);
//!
//! // 使用计时器
//! let timer = logger.time("执行任务", None);
//! // ... 执行任务 ...
//! timer.stop(); // 自动记录耗时
//! ```

use crate::app::agent::global;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 日志级别枚举
///
/// 定义了四个标准的日志级别，按照严重程度递增排列：
/// - `Debug`: 调试信息，仅在开发阶段使用
/// - `Info`: 一般信息，记录正常的操作流程
/// - `Warn`: 警告信息，记录潜在问题但不影响系统运行
/// - `Error`: 错误信息，记录需要关注的错误情况
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// 调试级别 - 用于详细的调试信息
    Debug,
    /// 信息级别 - 用于常规操作信息
    Info,
    /// 警告级别 - 用于潜在问题警告
    Warn,
    /// 错误级别 - 用于错误报告
    Error,
}

impl Level {
    /// 获取日志级别的优先级数值
    ///
    /// 优先级用于日志级别过滤，数值越大表示级别越高。
    /// 只有输入日志的优先级大于或等于当前配置的级别时，才会被记录。
    ///
    /// # 返回值
    ///
    /// 返回该级别对应的优先级数值：
    /// - Debug: 0
    /// - Info: 1
    /// - Warn: 2
    /// - Error: 3
    fn priority(self) -> usize {
        match self {
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warn => 2,
            Level::Error => 3,
        }
    }

    /// 获取日志级别的标签文本
    ///
    /// # 返回值
    ///
    /// 返回固定宽度（5字符）的级别标签字符串，用于日志格式化输出：
    /// - Debug: "DEBUG"
    /// - Info: "INFO "（右侧填充空格）
    /// - Warn: "WARN "
    /// - Error: "ERROR"
    fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO ",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        }
    }
}

/// 日志系统初始化选项
///
/// 用于配置日志系统的行为，包括输出目标、运行模式和日志级别。
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// 是否打印到标准错误流
    ///
    /// - `true`: 日志输出到 stderr
    /// - `false`: 日志输出到文件
    pub print: bool,

    /// 是否为开发模式
    ///
    /// 在开发模式下，日志文件名为 `dev.log`，便于开发调试。
    /// 在生产模式下，日志文件名包含时间戳，便于归档和追踪。
    pub dev: bool,

    /// 配置的日志级别
    ///
    /// 只有级别大于或等于此值的日志才会被记录。
    /// 如果为 `None`，则使用默认级别（Info）。
    pub level: Option<Level>,
}

/// 当前配置的日志级别（原子存储）
///
/// 存储当前生效的日志级别优先级数值。
/// 初始值为 `Level::Info` 的优先级（即 1）。
static LEVEL: AtomicUsize = AtomicUsize::new(Level::Info as usize);

/// 上次记录日志的时间戳（毫秒）
///
/// 用于计算两次日志记录之间的时间间隔。
/// 初始值为 0，表示尚无记录。
static LAST_MS: AtomicU64 = AtomicU64::new(0);

/// 获取当前时间的 Unix 时间戳（毫秒）
///
/// # 返回值
///
/// 返回自 Unix 纪元（1970-01-01 00:00:00 UTC）以来的毫秒数。
/// 如果获取时间失败，返回 `u64::MAX`。
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// 判断指定级别的日志是否应该被记录
///
/// 比较输入日志级别的优先级与当前配置的级别优先级，
/// 只有输入级别优先级大于或等于配置级别时才返回 true。
///
/// # 参数
///
/// - `input`: 待检查的日志级别
///
/// # 返回值
///
/// - `true`: 该级别日志应该被记录
/// - `false`: 该级别日志应该被忽略
fn should_log(input: Level) -> bool {
    let cur = LEVEL.load(Ordering::Relaxed);
    input.priority() >= cur
}

/// 日志写入器枚举
///
/// 定义了两种日志输出目标：
/// - 标准错误流（Stderr）
/// - 文件（通过缓冲写入器）
enum Writer {
    /// 输出到标准错误流
    Stderr,
    /// 输出到文件（带缓冲）
    File(Arc<Mutex<BufWriter<File>>>),
}

/// 全局日志写入器
///
/// 懒初始化的静态变量，默认输出到标准错误流。
/// 可以通过 `init()` 函数重新配置为文件输出。
static WRITER: LazyLock<Mutex<Writer>> = LazyLock::new(|| Mutex::new(Writer::Stderr));

/// 当前日志文件的路径
///
/// 当日志输出到文件时，存储日志文件的完整路径。
/// 当输出到 stderr 时，值为 `None`。
static LOG_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// 获取当前日志文件的路径
///
/// # 返回值
///
/// - `Some(PathBuf)`: 当前日志文件的完整路径
/// - `None`: 日志正在输出到标准错误流，无文件路径
///
/// # 示例
///
/// ```rust,ignore
/// if let Some(path) = file() {
///     println!("日志文件位于: {:?}", path);
/// }
/// ```
pub fn file() -> Option<PathBuf> {
    LOG_PATH.lock().ok().and_then(|p| p.clone())
}

/// 将一行日志写入到配置的输出目标
///
/// 根据当前的 `WRITER` 配置，将日志内容写入到标准错误流或文件。
/// 写入失败时静默忽略错误。
///
/// # 参数
///
/// - `line`: 要写入的日志行（应包含换行符）
fn write_line(line: &str) {
    let mut lock = match WRITER.lock() {
        Ok(l) => l,
        Err(_) => return,
    };
    match &mut *lock {
        Writer::Stderr => {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
        Writer::File(f) => {
            let mut file_lock = match f.lock() {
                Ok(l) => l,
                Err(_) => return,
            };
            let _ = file_lock.write_all(line.as_bytes());
            let _ = file_lock.flush();
        }
    }
}

/// 获取日志目录路径
///
/// 返回全局配置中定义的日志目录路径。
///
/// # 返回值
///
/// 返回日志目录的 `PathBuf`。
fn log_dir() -> PathBuf {
    global::paths().log.clone()
}

/// 初始化日志系统
///
/// 根据提供的选项配置日志系统。此函数应在应用启动时调用一次。
/// 它会：
/// 1. 设置日志级别
/// 2. 清理旧的日志文件
/// 3. 配置输出目标（stderr 或文件）
/// 4. 创建日志文件（如果输出到文件）
///
/// # 参数
///
/// - `options`: 初始化选项，包含输出目标、模式和日志级别配置
///
/// # 示例
///
/// ```rust,ignore
/// // 输出到文件，生产模式，Info 级别
/// init(InitOptions {
///     print: false,
///     dev: false,
///     level: Some(Level::Info),
/// });
///
/// // 输出到 stderr，开发模式
/// init(InitOptions {
///     print: true,
///     dev: true,
///     level: Some(Level::Debug),
/// });
/// ```
pub fn init(options: InitOptions) {
    let dir = log_dir();
    init_in_dir(options, &dir);
}

fn init_in_dir(options: InitOptions, dir: &Path) {
    // 设置日志级别
    if let Some(lv) = options.level {
        LEVEL.store(lv.priority(), Ordering::Relaxed);
    }

    // 清理旧的日志文件
    cleanup(dir);

    // 如果配置为打印模式，输出到 stderr
    if options.print {
        if let Ok(mut w) = WRITER.lock() {
            *w = Writer::Stderr;
        }
        if let Ok(mut p) = LOG_PATH.lock() {
            *p = None;
        }
        return;
    }

    // 确定日志文件名：开发模式使用固定名称，生产模式使用时间戳
    let name =
        if options.dev { "dev.log".to_string() } else { format!("{}.log", iso_file_stamp_utc()) };
    let path = dir.join(name);

    // 确保日志目录存在
    let _ = std::fs::create_dir_all(dir);

    // 清空现有文件（如果存在）
    let _ = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&path);

    // 以追加模式打开文件
    let file = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    // 创建缓冲写入器
    let writer = Arc::new(Mutex::new(BufWriter::new(file)));

    // 更新全局写入器
    if let Ok(mut w) = WRITER.lock() {
        *w = Writer::File(writer);
    }

    // 记录日志文件路径
    if let Ok(mut p) = LOG_PATH.lock() {
        *p = Some(path);
    }
}

/// 生成 ISO 8601 格式的时间戳字符串（UTC，秒级精度）
///
/// 格式：`YYYY-MM-DDTHH:MM:SS`
///
/// # 返回值
///
/// 返回格式化的 UTC 时间字符串。如果时间获取失败，返回 `"1970-01-01T00:00:00"`。
///
/// # 平台差异
///
/// - 非 WASM 平台：使用 `time` crate 生成准确时间
/// - WASM 平台：返回固定的时间字符串（受限于 WASM 环境）
fn iso_seconds_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = now.as_secs() as i64;

    #[cfg(not(target_arch = "wasm32"))]
    {
        use time::{OffsetDateTime, UtcOffset, format_description};
        let fmt = format_description::parse_borrowed::<2>(
            "[year]-[month]-[day]T[hour]:[minute]:[second]",
        )
        .unwrap_or_default();
        let dt = OffsetDateTime::from_unix_timestamp(secs)
            .unwrap_or_else(|_| OffsetDateTime::UNIX_EPOCH)
            .to_offset(UtcOffset::UTC);
        dt.format(&fmt).unwrap_or_else(|_| "1970-01-01T00:00:00".to_string())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = secs;
        "1970-01-01T00:00:00".to_string()
    }
}

/// 生成用于日志文件名的 ISO 时间戳字符串（UTC）
///
/// 格式：`YYYY-MM-DDTHHMMSS`（无分隔符，适合文件名）
///
/// # 返回值
///
/// 返回格式化的 UTC 时间字符串。如果时间获取失败，返回 `"1970-01-01T000000"`。
///
/// # 平台差异
///
/// - 非 WASM 平台：使用 `time` crate 生成准确时间
/// - WASM 平台：返回固定的时间字符串（受限于 WASM 环境）
fn iso_file_stamp_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = now.as_secs() as i64;

    #[cfg(not(target_arch = "wasm32"))]
    {
        use time::{OffsetDateTime, UtcOffset, format_description};
        let fmt =
            format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour][minute][second]")
                .unwrap_or_default();
        let dt = OffsetDateTime::from_unix_timestamp(secs)
            .unwrap_or_else(|_| OffsetDateTime::UNIX_EPOCH)
            .to_offset(UtcOffset::UTC);
        dt.format(&fmt).unwrap_or_else(|_| "1970-01-01T000000".to_string())
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = secs;
        "1970-01-01T000000".to_string()
    }
}

/// 清理旧的日志文件
///
/// 保留最近的日志文件，删除较旧的文件以避免磁盘空间占用过多。
/// 清理规则：
/// - 如果日志文件数量 <= 5，不执行清理
/// - 如果日志文件数量 > 10，删除最旧的文件，只保留 10 个
/// - 只清理符合时间戳命名格式的日志文件（`YYYY-MM-DDTHHMMSS.log`）
///
/// # 参数
///
/// - `dir`: 日志目录路径
fn cleanup(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    // 收集所有符合时间戳格式的日志文件
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|s| s.to_str()).map(is_timestamp_log_name).unwrap_or(false)
        })
        .collect();

    // 按文件名（包含时间戳）排序，最旧的在前
    files.sort();

    // 文件数量较少时不清理
    if files.len() <= 5 {
        return;
    }

    // 计算需要删除的文件数量（保留 10 个）
    let delete_count = files.len().saturating_sub(10);
    if delete_count == 0 {
        return;
    }

    // 删除最旧的文件
    for p in files.into_iter().take(delete_count) {
        let _ = std::fs::remove_file(p);
    }
}

/// 检查文件名是否符合时间戳日志文件的命名格式
///
/// 有效的文件名格式：`YYYY-MM-DDTHHMMSS.log`
/// 例如：`2024-03-15T143022.log`
///
/// # 参数
///
/// - `name`: 文件名字符串
///
/// # 返回值
///
/// - `true`: 文件名符合时间戳日志格式
/// - `false`: 文件名不符合格式
fn is_timestamp_log_name(name: &str) -> bool {
    let bytes = name.as_bytes();

    // 检查长度是否正确
    if bytes.len() != "0000-00-00T000000.log".len() {
        return false;
    }

    // 辅助函数：检查指定位置是否为数字
    let check_digit = |i: usize| bytes.get(i).map(|b| b.is_ascii_digit()).unwrap_or(false);

    // 检查数字位置：YYYY-MM-DDTHHMMSS
    // 位置：0-3（年），5-6（月），8-9（日），11-16（时分秒）
    for i in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 13, 14, 15, 16] {
        if !check_digit(i) {
            return false;
        }
    }

    // 检查分隔符和后缀
    name.get(4..5) == Some("-")
        && name.get(7..8) == Some("-")
        && name.get(10..11) == Some("T")
        && name.ends_with(".log")
}

/// 日志记录器
///
/// 提供结构化日志记录功能，支持：
/// - 添加标签（tags）用于分类和过滤
/// - 记录不同级别的日志
/// - 创建计时器以测量操作耗时
///
/// 日志记录器是线程安全的，可以在多个线程间共享。
///
/// # 示例
///
/// ```rust,ignore
/// let logger = create(Some({
///     let mut m = serde_json::Map::new();
///     m.insert("service".to_string(), serde_json::Value::String("api".to_string()));
///     m
/// }));
///
/// logger.info("请求开始", None);
/// logger.tag("user_id", "12345").info("用户登录", None);
///
/// let timer = logger.time("处理请求", None);
/// // ... 处理逻辑 ...
/// timer.stop();
/// ```
#[derive(Clone)]
pub struct Logger {
    /// 内部状态，包含标签映射
    inner: Arc<LoggerInner>,
}

/// 日志记录器内部状态
///
/// 存储日志记录器的标签集合。使用 `Mutex` 保证线程安全。
struct LoggerInner {
    /// 结构化标签映射
    tags: Mutex<Map<String, Value>>,
}

/// 全局日志记录器缓存
///
/// 按 service 名称缓存日志记录器实例，避免重复创建。
static LOGGERS: LazyLock<Mutex<HashMap<String, Logger>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 默认日志记录器
///
/// 提供一个预配置的默认日志记录器，带有 `"service": "default"` 标签。
/// 可用于简单的日志场景，无需自定义标签。
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::agent::util::log::DEFAULT;
///
/// DEFAULT.info("应用启动", None);
/// ```
pub static DEFAULT: LazyLock<Logger> = LazyLock::new(|| {
    create(Some(map_from_pairs([("service", Value::String("default".to_string()))])))
});

/// 创建新的日志记录器
///
/// 根据提供的标签创建日志记录器。如果标签中包含 `service` 字段，
/// 会尝试从缓存中复用已有的记录器实例。
///
/// # 参数
///
/// - `tags`: 可选的标签映射，用于结构化日志记录
///
/// # 返回值
///
/// 返回配置好的 `Logger` 实例
///
/// # 示例
///
/// ```rust,ignore
/// // 创建带服务标签的日志记录器
/// let logger = create(Some({
///     let mut m = serde_json::Map::new();
///     m.insert("service".to_string(), serde_json::Value::String("auth".to_string()));
///     m.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
///     m
/// }));
///
/// // 创建无标签的日志记录器
/// let simple_logger = create(None);
/// ```
pub fn create(tags: Option<Map<String, Value>>) -> Logger {
    let tags = tags.unwrap_or_default();

    // 如果有 service 标签，尝试从缓存获取
    if let Some(service) = tags.get("service").and_then(|v| v.as_str()).map(str::to_string)
        && let Ok(mut lock) = LOGGERS.lock()
    {
        // 检查缓存
        if let Some(cached) = lock.get(&service) {
            return cached.clone();
        }

        // 创建新记录器并缓存
        let logger = Logger { inner: Arc::new(LoggerInner { tags: Mutex::new(tags) }) };
        lock.insert(service, logger.clone());
        return logger;
    }

    // 无 service 标签或缓存失败，直接创建
    Logger { inner: Arc::new(LoggerInner { tags: Mutex::new(tags) }) }
}

/// 从键值对数组创建 JSON Map
///
/// # 参数
///
/// - `pairs`: 键值对数组，键为 `&'static str`，值为 `serde_json::Value`
///
/// # 返回值
///
/// 返回包含所有键值对的 `Map<String, Value>`
fn map_from_pairs<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

impl Logger {
    /// 为日志记录器添加标签（链式调用）
    ///
    /// 添加或更新一个标签，返回新的 `Logger` 实例以支持链式调用。
    /// 注意：此方法会修改当前记录器的标签集合。
    ///
    /// # 参数
    ///
    /// - `key`: 标签键名
    /// - `value`: 标签值（字符串形式）
    ///
    /// # 返回值
    ///
    /// 返回自身的克隆，以便链式调用
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// logger
    ///     .tag("user_id", "12345")
    ///     .tag("action", "login")
    ///     .info("用户操作", None);
    /// ```
    pub fn tag(&self, key: &str, value: &str) -> Self {
        if let Ok(mut lock) = self.inner.tags.lock() {
            lock.insert(key.to_string(), Value::String(value.to_string()));
        }
        self.clone()
    }

    /// 克隆日志记录器
    ///
    /// 创建当前记录器的独立副本，包含相同的标签快照。
    ///
    /// # 返回值
    ///
    /// 返回新的 `Logger` 实例
    pub fn clone_logger(&self) -> Self {
        let snap = self.tags_snapshot();
        create(Some(snap))
    }

    /// 记录 Debug 级别日志
    ///
    /// 用于记录详细的调试信息，通常仅在开发阶段启用。
    ///
    /// # 参数
    ///
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// logger.debug("处理请求", Some({
    ///     let mut m = serde_json::Map::new();
    ///     m.insert("request_id".to_string(), serde_json::Value::String("abc-123".to_string()));
    ///     m
    /// }));
    /// ```
    pub fn debug(&self, message: impl ToString, extra: Option<Map<String, Value>>) {
        self.log(Level::Debug, message.to_string(), extra);
    }

    /// 记录 Info 级别日志
    ///
    /// 用于记录常规的操作信息。
    ///
    /// # 参数
    ///
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// logger.info("服务启动完成", None);
    /// ```
    pub fn info(&self, message: impl ToString, extra: Option<Map<String, Value>>) {
        self.log(Level::Info, message.to_string(), extra);
    }

    /// 记录 Warn 级别日志
    ///
    /// 用于记录潜在问题或需要关注的情况。
    ///
    /// # 参数
    ///
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// logger.warn("配置项缺失，使用默认值", Some({
    ///     let mut m = serde_json::Map::new();
    ///     m.insert("config_key".to_string(), serde_json::Value::String("timeout".to_string()));
    ///     m
    /// }));
    /// ```
    pub fn warn(&self, message: impl ToString, extra: Option<Map<String, Value>>) {
        self.log(Level::Warn, message.to_string(), extra);
    }

    /// 记录 Error 级别日志
    ///
    /// 用于记录错误情况，需要关注和处理。
    ///
    /// # 参数
    ///
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// logger.error("数据库连接失败", Some({
    ///     let mut m = serde_json::Map::new();
    ///     m.insert("error".to_string(), serde_json::Value::String(err.to_string()));
    ///     m
    /// }));
    /// ```
    pub fn error(&self, message: impl ToString, extra: Option<Map<String, Value>>) {
        self.log(Level::Error, message.to_string(), extra);
    }

    /// 创建计时器
    ///
    /// 创建一个计时器用于测量操作的执行时间。
    /// 计时器启动时会立即记录一条 Info 日志（状态为 "started"），
    /// 停止时会记录另一条 Info 日志（状态为 "completed"，包含耗时）。
    ///
    /// 计时器会在被丢弃时自动停止（实现 `Drop` trait）。
    ///
    /// # 参数
    ///
    /// - `message`: 计时器的描述消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 返回值
    ///
    /// 返回 `Timer` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let timer = logger.time("处理请求", None);
    /// // ... 执行耗时操作 ...
    /// timer.stop(); // 或让 timer 自动 drop
    /// ```
    pub fn time(&self, message: impl ToString, extra: Option<Map<String, Value>>) -> Timer {
        let msg = message.to_string();

        // 记录开始日志
        let mut started_extra = extra.clone().unwrap_or_default();
        started_extra.insert("status".to_string(), Value::String("started".to_string()));
        self.info(msg.clone(), Some(started_extra));

        Timer {
            stopped: AtomicBool::new(false),
            logger: self.clone(),
            message: msg,
            start_ms: now_ms(),
            extra,
        }
    }

    /// 内部日志记录方法
    ///
    /// 执行实际的日志记录：检查级别、构建日志行、写入输出。
    ///
    /// # 参数
    ///
    /// - `level`: 日志级别
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    fn log(&self, level: Level, message: String, extra: Option<Map<String, Value>>) {
        // 检查日志级别
        if !should_log(level) {
            return;
        }

        // 构建日志行并写入
        let line = self.build(level, &message, extra.as_ref());
        write_line(&line);
    }

    /// 获取当前标签的快照
    ///
    /// # 返回值
    ///
    /// 返回当前所有标签的克隆副本
    fn tags_snapshot(&self) -> Map<String, Value> {
        self.inner.tags.lock().ok().map(|t| t.clone()).unwrap_or_default()
    }

    /// 构建格式化的日志行
    ///
    /// 将日志信息格式化为单行字符串，包含：
    /// - 日志级别标签
    /// - UTC 时间戳
    /// - 距上次日志的时间间隔
    /// - 标签键值对
    /// - 日志消息
    ///
    /// 输出格式：`LEVEL TIMESTAMP +XXms key=value key2=value2 message`
    ///
    /// # 参数
    ///
    /// - `level`: 日志级别
    /// - `message`: 日志消息
    /// - `extra`: 可选的额外标签映射
    ///
    /// # 返回值
    ///
    /// 返回格式化的日志行（包含末尾换行符）
    fn build(&self, level: Level, message: &str, extra: Option<&Map<String, Value>>) -> String {
        // 合并基础标签和额外标签
        let mut merged = self.tags_snapshot();
        if let Some(extra) = extra {
            for (k, v) in extra {
                merged.insert(k.clone(), v.clone());
            }
        }

        // 移除 null 值
        merged.retain(|_, v| !v.is_null());

        // 构建标签前缀字符串
        let prefix = merged
            .iter()
            .filter_map(|(k, v)| {
                let mut out = String::new();
                out.push_str(k);
                out.push('=');
                match v {
                    Value::String(s) => out.push_str(s),
                    _ => out.push_str(
                        &serde_json::to_string(v).unwrap_or_else(|_| "\"?\"".to_string()),
                    ),
                }
                Some(out)
            })
            .collect::<Vec<_>>()
            .join(" ");

        // 计算时间差
        let now = now_ms();
        let last = LAST_MS.swap(now, Ordering::Relaxed);
        let diff = if last == 0 { 0 } else { now.saturating_sub(last) };

        // 获取时间戳
        let ts = iso_seconds_utc();

        // 组装日志行各部分
        let mut parts: Vec<String> = Vec::new();
        parts.push(level.label().to_string());
        parts.push(ts);
        parts.push(format!("+{}ms", diff));
        if !prefix.is_empty() {
            parts.push(prefix);
        }
        if !message.is_empty() {
            parts.push(message.to_string());
        }

        parts.join(" ") + "\n"
    }
}

/// 计时器
///
/// 用于测量操作的执行时间。创建时自动记录开始日志，
/// 停止时记录完成日志并包含耗时信息。
///
/// 计时器实现了 `Drop` trait，在被丢弃时会自动调用 `stop()`。
///
/// # 示例
///
/// ```rust,ignore
/// let timer = logger.time("执行任务", None);
/// // ... 执行任务 ...
/// timer.stop(); // 显式停止
///
/// // 或者让计时器自动停止
/// {
///     let _timer = logger.time("自动计时的任务", None);
///     // ... 执行任务 ...
/// } // 离开作用域时自动停止
/// ```
pub struct Timer {
    /// 是否已停止（防止重复停止）
    stopped: AtomicBool,
    /// 关联的日志记录器
    logger: Logger,
    /// 计时器消息
    message: String,
    /// 开始时间（毫秒时间戳）
    start_ms: u64,
    /// 额外的标签映射
    extra: Option<Map<String, Value>>,
}

impl Timer {
    /// 停止计时器
    ///
    /// 记录完成日志，包含执行耗时。
    /// 此方法是幂等的，多次调用只会记录一次完成日志。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let timer = logger.time("处理请求", None);
    /// // ... 处理请求 ...
    /// timer.stop();
    /// ```
    pub fn stop(&self) {
        // 检查是否已停止，使用原子操作保证线程安全
        if self.stopped.swap(true, Ordering::Relaxed) {
            return;
        }

        // 构建完成日志的额外标签
        let mut completed_extra = self.extra.clone().unwrap_or_default();
        completed_extra.insert("status".to_string(), Value::String("completed".to_string()));
        completed_extra.insert(
            "duration".to_string(),
            Value::Number(serde_json::Number::from(now_ms().saturating_sub(self.start_ms))),
        );

        // 记录完成日志
        self.logger.info(self.message.clone(), Some(completed_extra));
    }
}

impl Drop for Timer {
    /// 计时器被丢弃时自动停止
    ///
    /// 这确保即使忘记显式调用 `stop()`，计时器也会正确记录完成日志。
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod log_tests;
