//! 调度器模块
//!
//! 本模块提供了定时任务调度功能，支持全局任务和实例级任务的管理。
//!
//! # 主要功能
//!
//! - 定时任务注册与调度
//! - 全局任务和实例任务的隔离管理
//! - 任务执行的生命周期控制
//! - 跨平台支持（Native 和 WASM）
//!
//! # 架构设计
//!
//! 调度器采用两层架构：
//! - **全局层**：所有实例共享的任务，使用 `GLOBAL` 静态变量存储
//! - **实例层**：每个实例独立的任务，使用 `INSTANCES` 静态变量按实例 ID 分组存储
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use std::time::Duration;
//! use std::sync::Arc;
//!
//! // 创建一个定时任务
//! let task = Task::new(
//!     "my_task",
//!     Duration::from_secs(60),
//!     Arc::new(|| Box::pin(async move {
//!         println!("任务执行中...");
//!         Ok(())
//!     }))
//! );
//!
//! // 注册全局任务
//! let mut task = task;
//! task.scope = Scope::Global;
//! register(task);
//!
//! // 或注册实例任务
//! let mut task = task;
//! task.scope = Scope::Instance("instance_1".to_string());
//! register(task);
//!
//! // 重置实例任务
//! reset_instance("instance_1");
//! ```

use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// 调度器专用日志记录器
///
/// 使用 `service: scheduler` 标签初始化，用于记录任务执行相关的日志信息。
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("scheduler".to_string()));
    log::create(Some(tags))
});

/// 任务执行函数的返回类型
///
/// 这是一个固定位置的 Future，输出类型为 `Result<(), String>`，
/// 表示任务执行成功或失败并返回错误信息。
pub type RunFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'static>>;

/// 任务执行函数的类型别名
///
/// 使用 `Arc` 包装的异步闭包，支持跨线程共享和执行。
/// 该闭包不接受参数，返回一个 `RunFuture`。
pub type RunFn = Arc<dyn Fn() -> RunFuture + Send + Sync + 'static>;

/// 任务作用域枚举
///
/// 定义定时任务的可见性和生命周期范围。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// 实例级任务
    ///
    /// 任务仅在特定实例中运行，当实例被重置时，任务会被清除。
    /// 适用于需要实例隔离的场景，如多租户环境。
    Instance(String),

    /// 全局任务
    ///
    /// 任务在全局范围内运行，不受实例重置影响。
    /// 适用于系统级的周期性任务，如清理、监控等。
    Global,
}

/// 定时任务结构体
///
/// 表示一个可调度的定时任务，包含任务标识、执行间隔、执行函数和作用域。
#[derive(Clone)]
pub struct Task {
    /// 任务唯一标识符
    ///
    /// 用于区分不同的任务，同一作用域内不能有重复的任务 ID。
    pub id: String,

    /// 任务执行间隔
    ///
    /// 定义任务每次执行之间的时间间隔。
    pub interval: Duration,

    /// 任务执行函数
    ///
    /// 异步函数，在每次触发时执行。
    pub run: RunFn,

    /// 任务作用域
    ///
    /// 决定任务是在全局还是实例级别运行。
    pub scope: Scope,
}

impl Task {
    /// 创建新的定时任务
    ///
    /// # 参数
    ///
    /// - `id`: 任务唯一标识符，可以是任何实现了 `Into<String>` 的类型
    /// - `interval`: 任务执行的时间间隔
    /// - `run`: 任务执行函数，使用 `Arc` 包装的异步闭包
    ///
    /// # 返回值
    ///
    /// 返回新创建的 `Task` 实例，默认作用域为空字符串的实例级任务
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    /// use std::sync::Arc;
    ///
    /// let task = Task::new(
    ///     "cleanup_task",
    ///     Duration::from_secs(3600),
    ///     Arc::new(|| Box::pin(async move {
    ///         // 执行清理逻辑
    ///         Ok(())
    ///     }))
    /// );
    /// ```
    pub fn new(id: impl Into<String>, interval: Duration, run: RunFn) -> Self {
        Self { id: id.into(), interval, run, scope: Scope::Instance(String::new()) }
    }
}

/// 任务句柄（非 WASM 平台）
///
/// 包装 `tokio::task::JoinHandle`，用于控制任务的执行。
#[cfg(not(target_arch = "wasm32"))]
pub struct TaskHandle(tokio::task::JoinHandle<()>);

/// 任务句柄（WASM 平台）
///
/// 在 WASM 环境中为空结构体，因为 WASM 的任务管理方式不同。
#[cfg(target_arch = "wasm32")]
pub struct TaskHandle;

impl TaskHandle {
    /// 中止任务执行
    ///
    /// 在非 WASM 平台上，调用 tokio 任务的 abort 方法终止任务。
    /// 在 WASM 平台上，此方法为空操作。
    pub fn abort(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.0.abort();
    }
}

/// 任务条目内部结构
///
/// 存储单个作用域（全局或实例）内的所有任务及其运行时句柄。
struct Entry {
    /// 任务定义映射表
    ///
    /// 键为任务 ID，值为任务定义。
    tasks: HashMap<String, Task>,

    /// 任务计时器句柄映射表
    ///
    /// 键为任务 ID，值为对应的任务句柄，用于控制任务的执行。
    timers: HashMap<String, TaskHandle>,
}

impl Entry {
    /// 创建新的任务条目
    fn new() -> Self {
        Self { tasks: HashMap::new(), timers: HashMap::new() }
    }
}

/// 全局任务存储
///
/// 使用互斥锁保护的任务条目，存储所有全局作用域的任务。
static GLOBAL: LazyLock<Mutex<Entry>> = LazyLock::new(|| Mutex::new(Entry::new()));

/// 实例任务存储
///
/// 使用互斥锁保护的映射表，键为实例 ID，值为该实例的任务条目。
static INSTANCES: LazyLock<Mutex<HashMap<String, Entry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 获取或创建 Tokio 运行时（非 WASM 平台）
///
/// 使用 `OnceLock` 确保运行时只创建一次。
/// 运行时配置为多线程模式，包含 2 个工作线程和最多 8 个阻塞线程。
///
/// # 返回值
///
/// 返回静态生命周期的 Tokio 运行时引用
#[cfg(not(target_arch = "wasm32"))]
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().expect("create tokio runtime"))
    })
}

/// 生成异步任务
///
/// 根据平台差异，使用不同的方式启动异步任务：
/// - 非 WASM 平台：使用 tokio 的 spawn 功能
/// - WASM 平台：使用 `wasm_bindgen_futures::spawn_local`
///
/// # 参数
///
/// - `fut`: 要执行的异步 Future
///
/// # 返回值
///
/// 返回任务句柄，可用于控制任务的执行
fn spawn(fut: impl Future<Output = ()> + Send + 'static) -> TaskHandle {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 优先尝试使用当前 tokio 运行时的句柄
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return TaskHandle(handle.spawn(fut));
        }
        // 如果没有当前运行时，使用全局运行时
        TaskHandle(runtime().spawn(fut))
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(fut);
        TaskHandle
    }
}

/// 重置实例的所有任务
///
/// 停止并清除指定实例的所有定时任务。
/// 这通常在实例销毁或重新初始化时调用。
///
/// # 参数
///
/// - `instance`: 实例标识符
///
/// # 行为说明
///
/// 1. 从 `INSTANCES` 中移除指定的实例条目
/// 2. 中止该实例下所有正在运行的定时器
/// 3. 清空任务列表
///
/// # 示例
///
/// ```rust,ignore
/// // 重置实例 "tenant_123" 的所有任务
/// reset_instance("tenant_123");
/// ```
pub fn reset_instance(instance: &str) {
    let mut instances = INSTANCES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut entry) = instances.remove(instance) else {
        return;
    };
    // 中止所有运行中的定时器
    for (_, handle) in entry.timers.drain() {
        handle.abort();
    }
    // 清空任务列表
    entry.tasks.clear();
}

/// 注册定时任务
///
/// 根据任务的作用域，将任务注册到相应的存储中。
///
/// # 参数
///
/// - `task`: 要注册的任务
///
/// # 行为说明
///
/// - 如果作用域为 `Global`，则注册到全局任务列表
/// - 如果作用域为 `Instance`，则注册到对应实例的任务列表
pub fn register(task: Task) {
    match task.scope.clone() {
        Scope::Global => register_in_global(task),
        Scope::Instance(instance) => register_in_instance(&instance, task),
    }
}

/// 在全局作用域注册任务
///
/// # 参数
///
/// - `task`: 要注册的任务
///
/// # 行为说明
///
/// 如果任务 ID 已存在，则忽略此次注册，避免重复注册。
fn register_in_global(task: Task) {
    let mut entry = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
    // 检查任务是否已存在，避免重复注册
    if entry.timers.contains_key(&task.id) {
        return;
    }
    install(&mut entry, task);
}

/// 在实例作用域注册任务
///
/// # 参数
///
/// - `instance`: 实例标识符
/// - `task`: 要注册的任务
///
/// # 行为说明
///
/// 如果实例中已存在相同 ID 的任务，会先中止旧任务再注册新任务。
fn register_in_instance(instance: &str, task: Task) {
    let mut instances = INSTANCES.lock().unwrap_or_else(|e| e.into_inner());
    // 获取或创建实例条目
    let entry = instances.entry(instance.to_string()).or_insert_with(Entry::new);
    // 如果任务已存在，先中止旧任务
    if let Some(existing) = entry.timers.remove(&task.id) {
        existing.abort();
    }
    install(entry, task);
}

/// 安装任务到条目中
///
/// 内部函数，将任务添加到指定条目并启动定时器。
///
/// # 参数
///
/// - `entry`: 任务条目引用
/// - `task`: 要安装的任务
///
/// # 行为说明
///
/// 1. 将任务定义存入 `tasks` 映射表
/// 2. 立即执行一次任务（`run_once`）
/// 3. 创建定时器，按指定间隔周期性执行任务
/// 4. 将定时器句柄存入 `timers` 映射表
fn install(entry: &mut Entry, task: Task) {
    // 存储任务定义
    entry.tasks.insert(task.id.clone(), task.clone());

    // 立即执行一次任务
    spawn(run_once(task.clone()));

    // 准备定时器所需的变量
    let id = task.id.clone();
    let interval = task.interval;
    let runner = task.run.clone();

    // 启动定时器任务
    let handle = spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // interval 首次 tick 会立即返回；启动执行已由 run_once 处理。
        ticker.tick().await;
        loop {
            // 等待下一个间隔
            ticker.tick().await;

            // 克隆变量以在新的异步块中使用
            let id2 = id.clone();
            let runner2 = runner.clone();

            // 在独立的任务中执行，避免阻塞定时器
            spawn(async move {
                run_inner(&id2, runner2).await;
            });
        }
    });

    // 存储定时器句柄
    entry.timers.insert(task.id, handle);
}

/// 执行一次任务
///
/// 内部辅助函数，包装任务执行逻辑。
///
/// # 参数
///
/// - `task`: 要执行的任务
async fn run_once(task: Task) {
    run_inner(&task.id, task.run).await;
}

/// 任务执行的核心逻辑
///
/// 执行任务函数并处理执行结果，记录相关日志。
///
/// # 参数
///
/// - `id`: 任务标识符，用于日志记录
/// - `run`: 任务执行函数
///
/// # 行为说明
///
/// - 任务开始时记录 INFO 日志
/// - 任务执行成功时不记录额外信息
/// - 任务执行失败时记录 ERROR 日志，包含任务 ID 和错误信息
async fn run_inner(id: &str, run: RunFn) {
    // 记录任务开始执行
    LOGGER.info("run", Some(extra([("id", Value::String(id.to_string()))])));

    // 执行任务并处理结果
    match (run)().await {
        Ok(()) => {
            // 任务执行成功，不记录额外信息
        }
        Err(error) => {
            // 任务执行失败，记录错误日志
            LOGGER.error(
                "run failed",
                Some(extra([
                    ("id", Value::String(id.to_string())),
                    ("error", Value::String(error)),
                ])),
            );
        }
    }
}

/// 创建日志额外字段映射表
///
/// 辅助函数，将键值对数组转换为 JSON 对象映射表。
///
/// # 参数
///
/// - `pairs`: 键值对数组，键为静态字符串，值为 JSON 值
///
/// # 返回值
///
/// 返回 `serde_json::Map<String, Value>` 类型的映射表
///
/// # 示例
///
/// ```rust,ignore
/// let extra = extra([
///     ("task_id", Value::String("task_1".to_string())),
///     ("status", Value::String("running".to_string())),
/// ]);
/// ```
fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
