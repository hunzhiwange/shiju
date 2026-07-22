//! 当前项目实例上下文。
//!
//! 本模块用 Tokio task-local 保存“当前目录、sandbox/worktree、项目信息”，并提供
//! 项目级状态缓存。调用方通过 `provide` 进入上下文后，可以在同一异步任务链上读取
//! directory/worktree/project，或创建随项目生命周期释放的状态对象。

use super::BoxFuture;
use super::{Error, Info};
use crate::app::agent::bus;
use crate::app::agent::bus::event as bus_event;
use crate::app::agent::project;
use crate::app::agent::project::state;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
/// 项目实例上下文。
///
/// 保存调用目录、实际 sandbox/worktree 以及持久化项目信息，供 task-local API 读取。
pub struct Context {
    /// 调用方传入的目录路径。
    pub directory: String,
    /// 解析后的工作树或 sandbox 路径。
    pub worktree: String,
    /// 当前目录对应的项目元数据。
    pub project: Info,
}

tokio::task_local! {
    static CONTEXT: Context;
}

static CACHE: LazyLock<Mutex<HashMap<String, Arc<Context>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 在项目上下文中执行异步逻辑。
///
/// 首次进入某个目录时会发现项目并缓存上下文；可选的 `init` 会在上下文已建立后、
/// 主逻辑执行前运行。
///
/// # 返回值
///
/// 返回闭包 `f` 的执行结果。
///
/// # 错误
///
/// 项目发现或元数据持久化失败时返回错误。
pub async fn provide<R>(
    directory: impl AsRef<Path>,
    init: Option<Box<dyn FnOnce() -> BoxFuture<()> + Send + 'static>>,
    f: impl FnOnce() -> BoxFuture<R> + Send + 'static,
) -> Result<R, Error> {
    let directory = directory.as_ref().to_path_buf();
    let key = directory.to_string_lossy().to_string();

    let existing = { CACHE.lock().await.get(&key).cloned() };
    let ctx = if let Some(ctx) = existing {
        ctx
    } else {
        let tenant = project::default_tenant_scope();
        let (project, sandbox) = project::from_directory(&tenant, &directory).await?;
        let ctx = Arc::new(Context { directory: key.clone(), worktree: sandbox, project });
        if let Some(init) = init {
            let ctx2 = ctx.clone();
            CONTEXT
                .scope((*ctx2).clone(), async move {
                    (init)().await;
                })
                .await;
        }
        CACHE.lock().await.insert(key.clone(), ctx.clone());
        ctx
    };

    Ok(CONTEXT.scope((*ctx).clone(), async move { f().await }).await)
}

/// 返回当前上下文的调用目录。
///
/// 未处于项目上下文时返回空字符串。
pub fn directory() -> String {
    CONTEXT.try_with(|c| c.directory.clone()).unwrap_or_default()
}

/// 返回当前上下文的 sandbox/worktree 路径。
///
/// 未处于项目上下文时返回空字符串。
pub fn worktree() -> String {
    CONTEXT.try_with(|c| c.worktree.clone()).unwrap_or_default()
}

/// 返回当前上下文的项目信息。
///
/// 未处于项目上下文时返回 `None`。
pub fn project() -> Option<Info> {
    CONTEXT.try_with(|c| c.project.clone()).ok()
}

/// 判断路径是否属于当前项目可访问范围。
///
/// 该检查同时接受原始调用目录与解析后的 worktree。根目录 `/` 被显式排除，避免
/// 异常上下文把所有绝对路径都视为项目内路径。
pub fn contains_path(filepath: impl AsRef<Path>) -> bool {
    let filepath = filepath.as_ref();
    let dir = PathBuf::from(directory());
    if dir.as_os_str().is_empty() {
        return false;
    }
    if filepath.starts_with(&dir) {
        return true;
    }
    let wt = PathBuf::from(worktree());
    if wt.as_os_str() == "/" {
        return false;
    }
    filepath.starts_with(&wt)
}

/// 创建绑定到当前项目实例的惰性状态提供器。
///
/// 状态按当前 `directory()` 和 `name` 缓存；当项目实例 dispose 时会调用可选的释放逻辑。
///
/// # 返回值
///
/// 返回一个异步获取器，调用后得到共享的 `Arc<S>` 状态。
pub fn state<S, Init, InitFut, Dispose, DisposeFut>(
    name: &'static str,
    init: Init,
    dispose: Option<Dispose>,
) -> impl Fn() -> BoxFuture<Arc<S>> + Send + Sync + 'static
where
    S: Send + Sync + 'static,
    Init: Fn() -> InitFut + Send + Sync + 'static,
    InitFut: std::future::Future<Output = S> + Send + 'static,
    Dispose: Fn(Arc<S>) -> DisposeFut + Send + Sync + 'static,
    DisposeFut: std::future::Future<Output = ()> + Send + 'static,
{
    state::create(|| directory(), name, init, dispose)
}

/// 释放当前项目实例缓存的状态与上下文。
///
/// 未处于项目上下文时直接返回。释放完成后会发布实例销毁事件。
pub async fn dispose() {
    let dir = directory();
    if dir.is_empty() {
        return;
    }
    state::dispose(&dir).await;
    CACHE.lock().await.remove(&dir);
    let _ = bus::publish(
        bus_event::INSTANCE_DISPOSED,
        serde_json::json!({ "directory": dir }),
        Some(dir),
    );
}

/// 释放所有已缓存的项目实例。
///
/// 逐个进入对应上下文调用 `dispose`，确保每个实例都按同一路径发布销毁事件。
pub async fn dispose_all() {
    let keys = { CACHE.lock().await.keys().cloned().collect::<Vec<_>>() };
    for key in keys {
        let ctx = { CACHE.lock().await.get(&key).cloned() };
        let Some(ctx) = ctx else {
            continue;
        };
        let _ = CONTEXT
            .scope((*ctx).clone(), async move {
                let _ = dispose().await;
            })
            .await;
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod instance_tests;
