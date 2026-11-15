//! 当前项目技能缓存与目录组装。
use crate::app::agent::project::instance;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::{config, flag};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
pub use skill_core::project::{Info, State};

/// 创建技能状态实例的工厂函数
///
/// 返回一个闭包，该闭包在被调用时会异步获取当前项目实例的技能状态。
/// 状态通过项目实例管理器进行缓存，避免重复加载。
///
/// # 返回值
///
/// 返回一个实现了 `Fn() -> BoxFuture<Arc<State>> + Send + Sync + 'static` 的闭包。
/// 该闭包可以被多次调用，每次都会返回当前项目实例的技能状态。
///
/// # 实现细节
///
/// - 使用 `instance::state` 函数注册状态加载器
/// - 首次访问时调用 `load_state()` 加载技能
/// - 不提供状态更新回调（使用 `None`）
fn instance_state()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "skill",
        || async { load_state().await },
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>,
    )
}

/// 根据名称获取指定的技能
///
/// 从当前项目实例的技能状态中查找指定名称的技能信息。
///
/// # 参数
///
/// - `name`: 技能的名称（对应 SKILL.md frontmatter 中的 name 字段）
///
/// # 返回值
///
/// - `Some(Info)`: 如果找到匹配的技能，返回技能信息的克隆
/// - `None`: 如果未找到指定名称的技能
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 异步获取技能
/// if let Some(skill) = skill::get("code-review").await {
///     println!("技能描述: {}", skill.description);
/// }
/// ```
///
/// # 性能
///
/// - 从缓存的技能状态中查找，时间复杂度为 O(1)
/// - 返回 Info 的克隆，适合多次使用
pub async fn get(name: &str) -> Option<Info> {
    instance_state()().await.skills.get(name).cloned()
}

/// 获取所有已加载的技能列表
///
/// 返回当前项目实例中所有已加载技能的信息列表。
///
/// # 返回值
///
/// 返回一个 `Vec<Info>`，包含所有技能的信息克隆。
/// 如果没有加载任何技能，返回空向量。
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 获取并遍历所有技能
/// for skill in skill::all().await {
///     println!("技能名称: {}", skill.name);
///     println!("技能描述: {}", skill.description);
///     println!("位置: {}", skill.location);
/// }
/// ```
///
/// # 性能
///
/// - 从缓存的技能状态中收集，避免重新扫描文件系统
/// - 返回所有技能的克隆，适合遍历和处理
pub async fn all() -> Vec<Info> {
    instance_state()().await.skills.values().cloned().collect()
}

/// 获取所有包含技能文件的目录路径列表
///
/// 返回当前已加载技能所在的所有目录路径。
/// 可用于了解技能的来源分布和文件系统布局。
///
/// # 返回值
///
/// 返回一个 `Vec<String>`，包含所有技能所在目录的路径。
/// 如果没有加载任何技能，返回空向量。
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::skill;
///
/// // 获取所有技能目录
/// for dir in skill::dirs().await {
///     println!("技能目录: {}", dir);
/// }
/// ```
///
/// # 用途
///
/// - 调试和日志：了解技能加载的来源
/// - 索引和监控：追踪技能文件的位置
/// - 工具集成：提供技能目录给外部工具使用
pub async fn dirs() -> Vec<String> {
    instance_state()().await.dirs.clone()
}

#[cfg(target_arch = "wasm32")]
async fn load_state() -> State {
    State::default()
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_state() -> State {
    let cfg = config::get().await;
    let mut external_dirs = Vec::new();
    if !*flag::VIBEWINDOW_DISABLE_EXTERNAL_SKILLS {
        let start = instance::directory();
        let workspace_dir = if start.is_empty() { None } else { Some(PathBuf::from(&start)) };
        external_dirs = crate::app::agent::skills::discover_local_skill_source_dirs_with_provider(
            workspace_dir.as_deref(), cfg.skills.directory_provider,
        ).into_iter().map(|source| source.path).collect();
    }
    skill_core::project::load_state(&external_dirs, || config::directories()).await
}
#[cfg(test)]
#[path = "skill_tests.rs"]
mod skill_tests;
