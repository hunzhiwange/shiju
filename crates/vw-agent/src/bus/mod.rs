//! # 事件总线模块
//!
//! 提供类型化的事件发布-订阅（Pub/Sub）机制，用于系统内部的解耦通信。
//!
//! ## 核心功能
//!
//! - **类型化事件发布**：通过 `Definition` 定义事件类型，支持结构化载荷
//! - **灵活订阅机制**：支持按类型订阅、通配符订阅、一次性订阅
//! - **全局事件流**：提供跨类型的全局事件订阅能力
//! - **目录上下文**：事件可携带目录信息，便于多实例场景下的事件路由
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::bus::{define, publish, subscribe, GlobalEvent, global_subscribe};
//!
//! // 定义事件类型
//! let my_event = define("my.custom.event");
//!
//! // 订阅特定类型事件
//! let unsub = subscribe(my_event, |payload| {
//!     println!("收到事件: {:?}", payload);
//! });
//!
//! // 发布事件
//! publish(my_event, serde_json::json!({"key": "value"}), None).ok();
//!
//! // 取消订阅
//! unsub();
//!
//! // 订阅所有事件（通配符）
//! subscribe_all(|payload| {
//!     println!("收到任意事件: {:?}", payload);
//! });
//!
//! // 全局订阅（可获取目录信息）
//! global_subscribe(|event: GlobalEvent| {
//!     println!("目录: {:?}, 载荷: {:?}", event.directory, event.payload);
//! });
//! ```

use crate::app::agent::util::log;
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 事件定义结构体
///
/// 用于标识特定类型的事件。事件类型是一个静态字符串，
/// 在编译期确定，避免运行时分配。
///
/// # 字段
///
/// - `r#type`: 事件类型标识符，使用点分命名法（如 `"server.instance.disposed"`）
#[derive(Debug, Clone, Copy)]
pub struct Definition {
    /// 事件类型标识符
    pub r#type: &'static str,
}

/// 预定义事件常量
///
/// 包含系统内置的事件类型定义，避免在代码中硬编码字符串。
pub mod event {
    use super::Definition;

    /// 实例销毁事件
    ///
    /// 当代理实例被销毁时触发，用于通知相关组件进行清理。
    pub const INSTANCE_DISPOSED: Definition = Definition { r#type: "server.instance.disposed" };
}

/// 订阅回调类型
///
/// 包装用户提供的回调函数，要求线程安全（`Send + Sync`）且具有静态生命周期。
type Subscription = Arc<dyn Fn(Value) + Send + Sync + 'static>;

/// 取消订阅回调类型
///
/// 调用此回调可移除对应的订阅，释放相关资源。
type Unsubscribe = Arc<dyn Fn() + Send + Sync + 'static>;

/// 事件总线内部状态
///
/// 存储所有活跃的订阅，按事件类型分组。
/// 每个订阅包含唯一 ID 和回调函数。
#[derive(Default)]
struct BusState {
    /// 订阅映射表
    ///
    /// Key: 事件类型字符串
    /// Value: 该类型下的所有订阅列表，每项为 (订阅ID, 回调函数)
    subscriptions: HashMap<String, Vec<(u64, Subscription)>>,
}

/// 事件总线专用日志记录器
///
/// 自动附加 `service: "bus"` 标签，便于日志过滤和追踪。
static LOG: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("bus".to_string()));
        m
    }))
});

/// 订阅 ID 生成器
///
/// 使用原子计数器为每个新订阅分配唯一 ID，用于取消订阅时精确匹配。
static SUB_ID: AtomicU64 = AtomicU64::new(1);

/// 事件总线全局状态
///
/// 使用 `Mutex` 保证线程安全，支持多线程并发访问。
static STATE: LazyLock<Mutex<BusState>> = LazyLock::new(|| Mutex::new(BusState::default()));

/// 全局事件结构体
///
/// 用于全局订阅模式，除了事件载荷外还包含目录上下文信息。
/// 目录字段可用于区分不同工作目录或实例的事件源。
///
/// # 序列化
///
/// 支持序列化为 JSON，便于日志记录和调试。
#[derive(Debug, Clone, Serialize)]
pub struct GlobalEvent {
    /// 事件来源目录（可选）
    ///
    /// 标识事件发生的目录上下文，多实例场景下可用于事件路由。
    pub directory: Option<String>,

    /// 事件载荷
    ///
    /// 包含 `type` 和 `properties` 字段的 JSON 对象。
    pub payload: Value,
}

/// 全局订阅状态
///
/// 存储所有全局订阅的回调函数，不区分事件类型。
#[derive(Default)]
struct GlobalState {
    /// 全局订阅列表
    ///
    /// 每项为 (订阅ID, 回调函数)，回调接收完整的 `GlobalEvent`。
    subscriptions: Vec<(u64, Arc<dyn Fn(GlobalEvent) + Send + Sync + 'static>)>,
}

/// 全局订阅状态存储
static GLOBAL_STATE: LazyLock<Mutex<GlobalState>> =
    LazyLock::new(|| Mutex::new(GlobalState::default()));

#[cfg(test)]
mod tests;

/// 创建事件定义
///
/// 从静态字符串创建事件类型定义。建议使用点分命名法命名事件类型，
/// 如 `"component.action.result"`。
///
/// # 参数
///
/// - `r#type`: 事件类型标识符，必须是静态生命周期字符串
///
/// # 返回值
///
/// 返回 `Definition` 实例，可用于发布或订阅事件。
///
/// # 示例
///
/// ```ignore
/// let user_created = define("user.created");
/// let task_completed = define("task.completed");
/// ```
pub fn define(r#type: &'static str) -> Definition {
    Definition { r#type }
}

/// 发布类型化事件
///
/// 将结构化数据序列化后发布到事件总线，通知所有相关订阅者。
///
/// # 参数
///
/// - `def`: 事件类型定义
/// - `properties`: 事件载荷，任何可序列化为 JSON 的类型
/// - `directory`: 可选的目录上下文，用于全局订阅者识别事件来源
///
/// # 返回值
///
/// - `Ok(())`: 发布成功
/// - `Err(e)`: 序列化失败时返回 JSON 错误
///
/// # 示例
///
/// ```ignore
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct UserCreated {
///     user_id: u64,
///     username: String,
/// }
///
/// let event = define("user.created");
/// publish(event, UserCreated { user_id: 1, username: "alice".into() }, None)?;
/// ```
pub fn publish<T: Serialize>(
    def: Definition,
    properties: T,
    directory: Option<String>,
) -> Result<(), serde_json::Error> {
    // 将属性序列化为 JSON Value 后调用底层发布函数
    publish_value(def, serde_json::to_value(properties)?, directory);
    Ok(())
}

/// 发布预序列化的事件
///
/// 与 `publish` 功能相同，但接受已序列化的 `Value` 作为载荷，
/// 避免重复序列化开销。
///
/// # 参数
///
/// - `def`: 事件类型定义
/// - `properties`: 已序列化的事件载荷
/// - `directory`: 可选的目录上下文
///
/// # 内部流程
///
/// 1. 构造包含 `type` 和 `properties` 的载荷对象
/// 2. 记录发布日志
/// 3. 获取类型化订阅和通配符订阅的回调副本
/// 4. 依次调用所有订阅回调
/// 5. 触发全局事件发射
pub fn publish_value(def: Definition, properties: Value, directory: Option<String>) {
    // 构造标准事件载荷格式
    let payload = json!({
        "type": def.r#type,
        "properties": properties
    });

    // 记录事件发布日志
    LOG.debug(
        "publishing",
        Some({
            let mut m = Map::new();
            m.insert("type".to_string(), Value::String(def.r#type.to_string()));
            m
        }),
    );

    // 从状态中提取订阅回调（分为类型化订阅和通配符订阅）
    // 使用 lock 期间最小化临界区，避免在持有锁时调用用户回调
    let (typed, wildcard) = {
        let lock = STATE.lock().unwrap_or_else(|e| e.into_inner());

        // 获取该事件类型的订阅回调
        let typed = lock
            .subscriptions
            .get(def.r#type)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, f)| f)
            .collect::<Vec<_>>();

        // 获取通配符订阅（"*" 类型）的回调
        let wildcard = lock
            .subscriptions
            .get("*")
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, f)| f)
            .collect::<Vec<_>>();
        (typed, wildcard)
    };

    // 依次调用所有订阅回调（先类型化，后通配符）
    for sub in typed.into_iter().chain(wildcard) {
        sub(payload.clone());
    }

    // 发射全局事件，通知全局订阅者
    emit_global(GlobalEvent { directory, payload });
}

/// 订阅特定类型事件
///
/// 注册回调函数以接收指定类型的所有事件通知。
///
/// # 参数
///
/// - `def`: 要订阅的事件类型定义
/// - `callback`: 事件回调函数，接收 JSON 载荷
///
/// # 返回值
///
/// 返回取消订阅函数，调用后将移除此订阅。
///
/// # 示例
///
/// ```ignore
/// let event = define("task.completed");
/// let unsub = subscribe(event, |payload| {
///     if let Some(task_id) = payload.get("task_id") {
///         println!("任务完成: {:?}", task_id);
///     }
/// });
///
/// // 不再需要时取消订阅
/// unsub();
/// ```
pub fn subscribe(def: Definition, callback: impl Fn(Value) + Send + Sync + 'static) -> Unsubscribe {
    raw(def.r#type, callback)
}

/// 订阅所有事件（通配符订阅）
///
/// 注册回调函数以接收任意类型的事件通知。
///
/// # 参数
///
/// - `callback`: 事件回调函数，接收包含 `type` 和 `properties` 的完整载荷
///
/// # 返回值
///
/// 返回取消订阅函数。
///
/// # 使用场景
///
/// - 日志记录和审计
/// - 调试和监控
/// - 需要处理多种事件类型的通用处理器
///
/// # 示例
///
/// ```ignore
/// subscribe_all(|payload| {
///     println!("事件类型: {:?}", payload.get("type"));
///     println!("事件数据: {:?}", payload.get("properties"));
/// });
/// ```
pub fn subscribe_all(callback: impl Fn(Value) + Send + Sync + 'static) -> Unsubscribe {
    raw("*", callback)
}

/// 一次性订阅
///
/// 注册回调函数，当回调返回 `true` 时自动取消订阅。
/// 适用于等待特定条件的事件。
///
/// # 参数
///
/// - `def`: 要订阅的事件类型定义
/// - `callback`: 条件回调函数，返回 `true` 表示满足条件，应取消订阅
///
/// # 工作原理
///
/// 1. 创建共享槽位存储取消订阅函数
/// 2. 包装用户回调，当返回 `true` 时调用取消订阅
/// 3. 立即注册订阅
///
/// # 示例
///
/// ```ignore
/// let event = define("process.exit");
///
/// // 等待进程退出事件
/// once(event, |payload| {
///     if let Some(code) = payload.get("properties").and_then(|p| p.get("exit_code")) {
///         if code == 0 {
///             println!("进程成功退出");
///             return true; // 取消订阅
///         }
///     }
///     false // 继续监听
/// });
/// ```
pub fn once(def: Definition, callback: impl Fn(Value) -> bool + Send + Sync + 'static) {
    // 共享槽位，用于存储取消订阅函数
    let slot: Arc<Mutex<Option<Unsubscribe>>> = Arc::new(Mutex::new(None));
    let slot2 = slot.clone();

    // 注册订阅，包装用户回调
    let unsub = subscribe(def, move |event| {
        // 用户回调返回 true 时取消订阅
        if callback(event) {
            if let Some(f) = slot2.lock().ok().and_then(|mut s| s.take()) {
                f();
            }
        }
    });

    // 将取消订阅函数存入槽位
    if let Ok(mut s) = slot.lock() {
        *s = Some(unsub);
    }
}

/// 底层订阅实现
///
/// 内部函数，执行实际的订阅注册逻辑。
///
/// # 参数
///
/// - `type_key`: 事件类型键（可以是具体类型或 "*"）
/// - `callback`: 事件回调函数
///
/// # 返回值
///
/// 返回取消订阅函数。
///
/// # 实现细节
///
/// 1. 记录订阅日志
/// 2. 分配唯一订阅 ID
/// 3. 将订阅添加到状态映射表
/// 4. 返回闭包，调用时从映射表中移除订阅
fn raw(type_key: &str, callback: impl Fn(Value) + Send + Sync + 'static) -> Unsubscribe {
    // 记录订阅操作日志
    LOG.info(
        "subscribing",
        Some({
            let mut m = Map::new();
            m.insert("type".to_string(), Value::String(type_key.to_string()));
            m
        }),
    );

    // 分配唯一订阅 ID
    let id = SUB_ID.fetch_add(1, Ordering::Relaxed);

    // 将回调包装为 Arc
    let cb: Subscription = Arc::new(callback);

    // 将订阅添加到状态映射表
    {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.subscriptions.entry(type_key.to_string()).or_default().push((id, cb));
    }

    // 捕获类型键用于取消订阅
    let type_key = type_key.to_string();

    // 返回取消订阅函数
    Arc::new(move || {
        // 记录取消订阅日志
        LOG.info(
            "unsubscribing",
            Some({
                let mut m = Map::new();
                m.insert("type".to_string(), Value::String(type_key.clone()));
                m
            }),
        );

        // 从状态映射表中移除订阅
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(list) = lock.subscriptions.get_mut(&type_key) {
            // 保留 ID 不匹配的订阅
            list.retain(|(sub_id, _)| *sub_id != id);

            // 如果列表为空，移除整个类型条目以节省内存
            if list.is_empty() {
                lock.subscriptions.remove(&type_key);
            }
        }
    })
}

/// 全局事件订阅
///
/// 注册回调函数以接收所有事件的完整信息，包括目录上下文。
/// 与 `subscribe_all` 不同，此函数提供 `GlobalEvent` 而非原始 `Value`。
///
/// # 参数
///
/// - `callback`: 事件回调函数，接收 `GlobalEvent` 参数
///
/// # 返回值
///
/// 返回取消订阅函数。
///
/// # 使用场景
///
/// - 需要目录上下文的事件处理
/// - 跨实例的事件聚合
/// - 完整的事件日志记录
///
/// # 示例
///
/// ```ignore
/// global_subscribe(|event: GlobalEvent| {
///     println!("来源目录: {:?}", event.directory);
///     println!("事件类型: {:?}", event.payload.get("type"));
/// });
/// ```
pub fn global_subscribe(callback: impl Fn(GlobalEvent) + Send + Sync + 'static) -> Unsubscribe {
    // 分配唯一订阅 ID（复用同一计数器）
    let id = SUB_ID.fetch_add(1, Ordering::Relaxed);

    // 包装回调
    let cb: Arc<dyn Fn(GlobalEvent) + Send + Sync + 'static> = Arc::new(callback);

    // 添加到全局订阅列表
    {
        let mut lock = GLOBAL_STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.subscriptions.push((id, cb));
    }

    // 返回取消订阅函数
    Arc::new(move || {
        let mut lock = GLOBAL_STATE.lock().unwrap_or_else(|e| e.into_inner());
        // 移除匹配 ID 的订阅
        lock.subscriptions.retain(|(sub_id, _)| *sub_id != id);
    })
}

/// 发射全局事件
///
/// 内部函数，将事件发送给所有全局订阅者。
///
/// # 参数
///
/// - `event`: 要发射的全局事件
///
/// # 实现细节
///
/// 1. 在锁保护下克隆所有回调引用
/// 2. 释放锁后依次调用回调，避免死锁风险
fn emit_global(event: GlobalEvent) {
    // 克隆回调列表，最小化临界区
    let subs = {
        let lock = GLOBAL_STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.subscriptions.iter().map(|(_, f)| f.clone()).collect::<Vec<_>>()
    };

    // 依次调用所有全局订阅回调
    for f in subs {
        f(event.clone());
    }
}
