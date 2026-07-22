//! 文件监视器模块
//!
//! 本模块提供文件系统监视功能，用于检测指定目录中文件的变更事件（添加、修改、删除）。
//! 监视器通过定期扫描文件系统快照的方式检测变更，并通过事件总线发布变更通知。
//!
//! # 主要功能
//!
//! - 启动目录监视任务（`init`）
//! - 停止目录监视任务（`stop`）
//! - 查询监视任务运行状态（`running`）
//! - 发布文件更新事件（`publish_updated`）
//!
//! # 平台兼容性
//!
//! - 在非 WASM32 目标平台上提供完整功能实现
//! - 在 WASM32 目标平台上提供空实现（占位符）
//!
//! # 使用示例
//!
//! ```ignore
//! use std::path::Path;
//!
//! // 启动监视器
//! watcher::init("/path/to/watch");
//!
//! // 检查是否在运行
//! if watcher::running("/path/to/watch") {
//!     println!("监视器正在运行");
//! }
//!
//! // 停止监视器
//! watcher::stop("/path/to/watch");
//! ```

use crate::app::agent::bus;
use crate::app::agent::file::ignore;
use crate::app::agent::flag;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[cfg(test)]
#[path = "watcher_tests.rs"]
mod watcher_tests;

/// 文件监视器事件定义模块
///
/// 定义文件监视器相关的事件类型，用于事件总线的订阅和发布。
pub mod event {
    use crate::app::agent::bus;

    /// 文件更新事件定义
    ///
    /// 当监视器检测到文件变更（添加、修改、删除）时发布此事件。
    /// 事件类型为 `"file.watcher.updated"`。
    ///
    /// # 事件属性
    ///
    /// - `file`: 变更文件的相对路径
    /// - `event`: 变更类型（"add"、"change"、"unlink"）
    pub const UPDATED: bus::Definition = bus::Definition { r#type: "file.watcher.updated" };
}

/// 文件更新事件属性结构体
///
/// 包含文件变更事件的详细信息，用于序列化并通过事件总线传递。
///
/// # 字段说明
///
/// - `file`: 发生变更的文件路径（相对于监视根目录）
/// - `event`: 变更类型，可能的值包括：
///   - `"add"`: 文件被创建
///   - `"change"`: 文件被修改
///   - `"unlink"`: 文件被删除
///
/// # 示例
///
/// ```ignore
/// let props = UpdatedProperties {
///     file: "src/main.rs".to_string(),
///     event: "change".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct UpdatedProperties {
    /// 变更文件的相对路径
    pub file: String,
    /// 变更类型（"add"、"change"、"unlink"）
    pub event: String,
}

/// 活跃监视任务的全局注册表
///
/// 存储所有正在运行的文件监视任务句柄，键为目录路径字符串。
/// 通过此注册表可以：
/// - 避免对同一目录重复启动监视任务
/// - 查询特定目录的监视状态
/// - 停止特定的监视任务
///
/// # 线程安全性
///
/// 使用 `Mutex` 包装确保多线程环境下的安全访问。
#[cfg(not(target_arch = "wasm32"))]
static TASKS: LazyLock<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 扫描目录并生成文件快照
///
/// 遍历指定目录下的所有文件，记录每个文件的相对路径和最后修改时间。
/// 快照用于后续比较以检测文件变更。
///
/// # 参数
///
/// - `root`: 要扫描的根目录路径
///
/// # 返回值
///
/// 返回一个哈希表，其中：
/// - 键：文件相对于根目录的路径（使用正斜杠分隔）
/// - 值：文件的最后修改时间
///
/// # 过滤规则
///
/// 函数会自动过滤以下内容：
/// - `.git` 目录及其所有子内容
/// - 非文件条目（目录、符号链接等）
/// - 匹配 ignore 规则的文件
///
/// # 错误处理
///
/// - 扫描过程中遇到错误的条目会被静默跳过
/// - 无法读取元数据的文件使用 `UNIX_EPOCH` 作为修改时间
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// let snapshot = scan_snapshot(Path::new("/project/src"));
/// println!("找到 {} 个文件", snapshot.len());
/// ```
#[cfg(not(target_arch = "wasm32"))]
fn scan_snapshot(root: &Path) -> HashMap<String, SystemTime> {
    let mut out = HashMap::new();

    // 创建目录遍历器，不跟随符号链接
    let walker = walkdir::WalkDir::new(root).follow_links(false);

    // 遍历目录树
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        // 跳过 .git 目录
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if name == ".git" {
                continue;
            }
        }

        // 只处理普通文件
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // 计算相对路径，统一使用正斜杠
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");

        // 跳过 .git/ 开头的路径
        if rel.starts_with(".git/") {
            continue;
        }

        // 应用 ignore 规则过滤
        if ignore::matches(&rel, None, None) {
            continue;
        }

        // 获取文件修改时间
        if let Ok(meta) = path.metadata() {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            out.insert(rel, mtime);
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn snapshot_updates(
    previous: &HashMap<String, SystemTime>,
    next: &HashMap<String, SystemTime>,
) -> Vec<UpdatedProperties> {
    let mut updates = Vec::new();

    for (path, mtime) in next {
        match previous.get(path) {
            None => {
                updates.push(UpdatedProperties { file: path.clone(), event: "add".to_string() })
            }
            Some(old) if old != mtime => {
                updates.push(UpdatedProperties { file: path.clone(), event: "change".to_string() })
            }
            Some(_) => {}
        }
    }

    for path in previous.keys() {
        if !next.contains_key(path) {
            updates.push(UpdatedProperties { file: path.clone(), event: "unlink".to_string() });
        }
    }

    updates
}

/// 启动文件监视任务
///
/// 在指定目录上启动异步监视任务，定期扫描文件变更并通过事件总线发布通知。
/// 如果该目录已有监视任务在运行，则直接返回不执行任何操作。
///
/// # 参数
///
/// - `directory`: 要监视的目录路径，可以是任何实现了 `AsRef<Path>` 的类型
///
/// # 功能特性
///
/// 1. **变更检测周期**: 每 750 毫秒扫描一次文件系统
/// 2. **变更类型**: 检测文件的添加、修改和删除
/// 3. **事件发布**: 通过事件总线发布 `file.watcher.updated` 事件
/// 4. **去重保护**: 同一目录只允许一个监视任务运行
///
/// # 配置标志
///
/// 函数会检查以下实验性标志：
/// - `VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER`: 如果设置，直接返回
/// - `VIBEWINDOW_EXPERIMENTAL_FILEWATCHER`: 如果未设置，直接返回
///
/// # 线程安全性
///
/// 使用全局 `TASKS` 注册表管理任务句柄，确保线程安全。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// // 启动对项目目录的监视
/// init("/path/to/project");
///
/// // 多次调用同一目录是安全的（会被忽略）
/// init("/path/to/project");
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn init(directory: impl AsRef<Path>) {
    // 检查禁用标志
    if *flag::VIBEWINDOW_EXPERIMENTAL_DISABLE_FILEWATCHER {
        return;
    }

    // 检查启用标志
    if !*flag::VIBEWINDOW_EXPERIMENTAL_FILEWATCHER {
        return;
    }

    let dir = directory.as_ref().to_path_buf();
    let key = dir.to_string_lossy().to_string();

    // 获取任务注册表的锁
    let mut tasks = TASKS.lock().unwrap_or_else(|e| e.into_inner());

    // 如果已有该目录的监视任务，直接返回
    if tasks.contains_key(&key) {
        return;
    }

    // 生成异步监视任务
    let handle = tokio::spawn(async move {
        // 初始快照
        let mut prev = scan_snapshot(&dir);

        // 主监视循环
        loop {
            // 等待下一个扫描周期
            tokio::time::sleep(Duration::from_millis(750)).await;

            // 获取新快照
            let next = scan_snapshot(&dir);

            // 发布新增、修改和删除事件
            for update in snapshot_updates(&prev, &next) {
                let _ = bus::publish(event::UPDATED, update, None);
            }

            // 更新快照用于下次比较
            prev = next;
        }
    });

    // 将任务句柄存入注册表
    tasks.insert(key, handle);
}

/// 启动文件监视任务（WASM32 平台空实现）
///
/// 在 WebAssembly 环境中，文件系统监视功能不可用，此函数为空实现。
///
/// # 参数
///
/// - `_directory`: 忽略的目录路径参数
#[cfg(target_arch = "wasm32")]
pub fn init(_directory: impl AsRef<Path>) {}

/// 停止文件监视任务
///
/// 终止指定目录上的文件监视任务。如果该目录没有活跃的监视任务，则不执行任何操作。
///
/// # 参数
///
/// - `directory`: 要停止监视的目录路径
///
/// # 行为说明
///
/// - 从全局任务注册表中移除并中止该目录的监视任务
/// - 任务会被立即中止（通过 `abort()`），不会等待当前扫描周期完成
///
/// # 线程安全性
///
/// 访问全局 `TASKS` 注册表时会自动处理锁中毒情况。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// // 停止监视
/// stop("/path/to/watched/directory");
///
/// // 重复停止是安全的
/// stop("/path/to/watched/directory");
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn stop(directory: impl AsRef<Path>) {
    let dir = directory.as_ref().to_path_buf();
    let key = dir.to_string_lossy().to_string();

    // 获取任务注册表的锁，处理可能的锁中毒
    let mut tasks = TASKS.lock().unwrap_or_else(|e| e.into_inner());

    // 移除并中止任务
    if let Some(handle) = tasks.remove(&key) {
        handle.abort();
    }
}

/// 停止文件监视任务（WASM32 平台空实现）
///
/// 在 WebAssembly 环境中，文件系统监视功能不可用，此函数为空实现。
///
/// # 参数
///
/// - `_directory`: 忽略的目录路径参数
#[cfg(target_arch = "wasm32")]
pub fn stop(_directory: impl AsRef<Path>) {}

/// 检查监视任务是否正在运行
///
/// 查询指定目录上是否有活跃的文件监视任务。
///
/// # 参数
///
/// - `directory`: 要查询的目录路径
///
/// # 返回值
///
/// 返回 `bool` 值：
/// - `true`: 该目录有活跃的监视任务
/// - `false`: 该目录没有监视任务
///
/// # 线程安全性
///
/// 如果无法获取任务注册表的锁，返回 `false`。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// // 检查监视状态
/// if running("/project/src") {
///     println!("监视器正在运行");
/// } else {
///     println!("监视器未运行");
///     init("/project/src");  // 启动监视器
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn running(directory: impl AsRef<Path>) -> bool {
    let dir = directory.as_ref().to_path_buf();
    let key = dir.to_string_lossy().to_string();

    // 尝试获取锁并检查，失败时返回 false
    TASKS.lock().ok().is_some_and(|m| m.contains_key(&key))
}

/// 检查监视任务是否正在运行（WASM32 平台）
///
/// 在 WebAssembly 环境中，文件系统监视功能不可用，始终返回 `false`。
///
/// # 参数
///
/// - `_directory`: 忽略的目录路径参数
///
/// # 返回值
///
/// 始终返回 `false`
#[cfg(target_arch = "wasm32")]
pub fn running(_directory: impl AsRef<Path>) -> bool {
    false
}

/// 手动发布文件更新事件
///
/// 通过事件总线发布自定义的文件更新事件。此函数允许外部代码
/// 手动触发文件变更通知，而不依赖自动监视机制。
///
/// # 参数
///
/// - `file`: 文件路径字符串
/// - `event`: 事件类型字符串（建议使用 "add"、"change"、"unlink"）
///
/// # 事件格式
///
/// 发布的事件包含以下 JSON 属性：
/// ```json
/// {
///     "file": "<文件路径>",
///     "event": "<事件类型>"
/// }
/// ```
///
/// # 使用场景
///
/// - 外部系统集成触发文件变更通知
/// - 测试和调试事件处理逻辑
/// - 补充自动监视未能检测到的变更
///
/// # 示例
///
/// ```ignore
/// // 发布文件创建事件
/// publish_updated("src/new_module.rs", "add");
///
/// // 发布文件修改事件
/// publish_updated("config.toml", "change");
///
/// // 发布文件删除事件
/// publish_updated("temp/file.txt", "unlink");
/// ```
pub fn publish_updated(file: impl Into<String>, event: impl Into<String>) {
    let mut props = Map::new();
    props.insert("file".to_string(), Value::String(file.into()));
    props.insert("event".to_string(), Value::String(event.into()));
    bus::publish_value(event::UPDATED, Value::Object(props), None);
}
