//! 项目作用域状态缓存。
//!
//! 这里提供按项目根 key 和状态名称分组的惰性单例。状态值以 `Any` 存储，以便不同
//! 子系统复用同一套生命周期管理，同时通过创建时的泛型类型在取出时恢复强类型。

use super::BoxFuture;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

type AnyArc = Arc<dyn Any + Send + Sync>;
type DisposeFn = Arc<dyn Fn(AnyArc) -> BoxFuture<()> + Send + Sync + 'static>;

struct Entry {
    cell: Arc<OnceCell<AnyArc>>,
    dispose: Option<DisposeFn>,
}

#[derive(Default)]
struct State {
    records: HashMap<String, HashMap<&'static str, Entry>>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

/// 创建一个按 root/name 缓存的状态获取器。
///
/// 第一次调用获取器时运行 `init`，后续同一 root/name 复用同一个 `Arc<S>`。可选的
/// `dispose` 会在对应 root 被释放时执行。
///
/// # 返回值
///
/// 返回一个异步闭包，调用后得到共享状态实例。
pub fn create<S, Root, Init, InitFut, Dispose, DisposeFut>(
    root: Root,
    name: &'static str,
    init: Init,
    dispose: Option<Dispose>,
) -> impl Fn() -> BoxFuture<Arc<S>> + Send + Sync + 'static
where
    S: Send + Sync + 'static,
    Root: Fn() -> String + Send + Sync + 'static,
    Init: Fn() -> InitFut + Send + Sync + 'static,
    InitFut: Future<Output = S> + Send + 'static,
    Dispose: Fn(Arc<S>) -> DisposeFut + Send + Sync + 'static,
    DisposeFut: Future<Output = ()> + Send + 'static,
{
    let root = Arc::new(root);
    let init = Arc::new(init);
    let dispose: Option<DisposeFn> = dispose.map(|d| {
        let d = Arc::new(d);
        let f: DisposeFn = Arc::new(move |v: AnyArc| {
            let d = d.clone();
            Box::pin(async move {
                if let Ok(v) = v.downcast::<S>() {
                    (d)(v).await;
                }
            })
        });
        f
    });

    move || {
        let root = root.clone();
        let init = init.clone();
        let dispose = dispose.clone();
        Box::pin(async move {
            let key = (root)();
            let cell = {
                let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
                // 锁只保护表结构，初始化本身交给 OnceCell，避免持锁等待用户异步代码。
                let entry = lock.records.entry(key).or_default().entry(name).or_insert_with(|| {
                    Entry { cell: Arc::new(OnceCell::new()), dispose: dispose.clone() }
                });
                entry.cell.clone()
            };

            let any = cell
                .get_or_init(|| async move { Arc::new((init)().await) as AnyArc })
                .await
                .clone();

            any.downcast::<S>().unwrap()
        })
    }
}

/// 释放指定 root 下已经初始化的状态。
///
/// 未初始化的状态不会调用释放函数；释放函数按记录顺序串行等待完成。
pub async fn dispose(key: &str) {
    let entries = {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.records.remove(key)
    };
    let Some(entries) = entries else {
        return;
    };

    let mut tasks: Vec<BoxFuture<()>> = Vec::new();
    for (_, entry) in entries {
        let Some(dispose) = entry.dispose else {
            continue;
        };
        if let Some(v) = entry.cell.get() {
            // 只释放真正初始化过的值，保持惰性状态的零成本语义。
            tasks.push((dispose)(v.clone()));
        }
    }

    for task in tasks {
        task.await;
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;
