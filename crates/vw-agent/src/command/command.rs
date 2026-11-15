//! 项目上下文适配；命令类型、注册和事件实现归 agent-commands。
use crate::app::agent::{config, project::instance, skill};
use std::sync::Arc;

pub use agent_commands::{ExecutedEvent, Info, Source, State, event, hints, r#default};

/// 获取命令系统状态的实例化函数
///
/// 返回一个闭包，该闭包被调用时会异步加载并返回命令系统的状态。
/// 状态通过项目实例机制管理，支持按项目隔离。
///
/// ## 返回值
///
/// 返回一个闭包，该闭包：
/// - 无参数
/// - 返回 `BoxFuture<Arc<State>>`
/// - 是 `Send + Sync + 'static` 的，可以跨线程共享
///
/// ## 实现细节
///
/// - 使用 `instance::state` 函数创建状态访问器
/// - 模块名称为 `"command"`
/// - 状态通过 `load_state` 异步加载
/// - 未提供状态变更回调（传入 `None`）
///
/// ## 生命周期
///
/// 状态在首次访问时加载，之后会被缓存。
/// 不同的项目实例会维护各自独立的命令状态。
fn instance_state()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "command",                                                           // 模块标识符
        || async { load_state().await },                                     // 状态加载函数
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>, // 无状态变更回调
    )
}

/// 按名称查询单个命令
///
/// 从命令注册表中查询指定名称的命令信息。
///
/// ## 参数
///
/// - `name`：命令名称（如 `"init"`、`"review"` 或技能名称）
///
/// ## 返回值
///
/// - `Some(Info)`：找到命令，返回命令信息的克隆
/// - `None`：命令不存在
///
/// ## 异步说明
///
/// 该函数是异步的，因为命令状态可能需要从磁盘或配置中加载。
///
/// ## 示例
///
/// ```rust,ignore
/// // 查询内置命令
/// if let Some(cmd) = get("init").await {
///     println!("描述: {:?}", cmd.description);
/// }
///
/// // 查询不存在的命令
/// assert!(get("nonexistent").await.is_none());
/// ```
///
/// ## 性能
///
/// 命令状态会被缓存，重复查询不会触发重新加载。
pub async fn get(name: &str) -> Option<Info> {
    instance_state()().await.commands.get(name).cloned()
}

/// 获取所有可用命令列表
///
/// 返回命令注册表中所有命令的信息列表。
///
/// ## 返回值
///
/// 返回 `Vec<Info>`，包含所有注册命令的克隆。
/// 列表顺序取决于内部 `HashMap` 的迭代顺序，不保证特定顺序。
///
/// ## 异步说明
///
/// 该函数是异步的，因为命令状态可能需要从磁盘或配置中加载。
///
/// ## 示例
///
/// ```rust,ignore
/// let commands = list().await;
/// println!("共有 {} 个命令", commands.len());
/// for cmd in commands {
///     println!("- {}: {:?}", cmd.name, cmd.description);
/// }
/// ```
///
/// ## 性能
///
/// - 时间复杂度：O(n)，n 为命令数量
/// - 命令状态会被缓存，重复调用不会触发重新加载
pub async fn list() -> Vec<Info> {
    instance_state()().await.commands.values().cloned().collect()
}

/// 发布命令执行事件
///
/// 将命令执行事件发布到事件总线，供订阅者处理。
///
/// ## 参数
///
/// - `evt`：命令执行事件数据
///
/// ## 返回值
///
/// - `Ok(())`：事件发布成功
/// - `Err(serde_json::Error)`：事件序列化失败
///
/// ## 事件类型
///
/// 事件类型为 `event::EXECUTED`（`"command.executed"`）。
///
/// ## 事件目录
///
/// 事件会被发布到当前项目实例的目录中。
///
/// ## 示例
///
/// ```rust,ignore
/// let event = ExecutedEvent {
///     name: "init".to_string(),
///     session_id: "session-123".to_string(),
///     arguments: "".to_string(),
///     message_id: "msg-456".to_string(),
/// };
/// publish_executed(event)?;
/// ```
///
/// ## 错误处理
///
/// 如果事件无法序列化为 JSON，会返回错误。
/// 调用者应适当处理此错误（如记录日志）。
pub fn publish_executed(evt: ExecutedEvent) -> Result<(), serde_json::Error> {
    agent_commands::publish_executed(evt, instance::directory())
}

/// 保留配置加载与技能发现的原有顺序及项目缓存语义。
async fn load_state() -> State {
    let _cfg = config::get().await;
    let worktree = instance::worktree();
    let skills = skill::all().await.into_iter().map(|skill| agent_commands::SkillCommand {
        name: skill.name,
        description: skill.description,
        content: skill.content,
    });
    agent_commands::load_state(&worktree, skills)
}
