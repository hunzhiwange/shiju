//! 简单的进程内异步缓存。
//!
//! 本模块用于缓存按 key 惰性初始化的共享状态，避免在一次进程生命周期内
//! 重复执行昂贵的数据聚合或文件读取操作。
//!
//! 当前主要用于缓存 provider 聚合状态。

use once_cell::sync::Lazy;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

/// 以类型擦除形式存储在缓存中的共享值。
type AnyArc = Arc<dyn Any + Send + Sync>;

struct Entry {
    cell: Arc<OnceCell<AnyArc>>,
}

struct GlobalState {
    records: HashMap<String, Entry>,
}

static STATE: Lazy<Mutex<GlobalState>> =
    Lazy::new(|| Mutex::new(GlobalState { records: HashMap::new() }));

/// 按 key 获取缓存值；若不存在则仅初始化一次。
///
/// 该函数会对同一个 key 复用同一个 `OnceCell`，从而保证并发场景下
/// 初始化逻辑只执行一次。
pub async fn get_or_init<S, F, Fut>(key: &str, init: F) -> Arc<S>
where
    S: Send + Sync + 'static,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Arc<S>>,
{
    let cell = {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.records
            .entry(key.to_string())
            .or_insert_with(|| Entry { cell: Arc::new(OnceCell::new()) })
            .cell
            .clone()
    };
    let any = cell.get_or_init(|| async { init().await as AnyArc }).await.clone();
    any.downcast::<S>().unwrap()
}

/// 使指定 key 的缓存失效。
///
/// 下次访问该 key 时会重新执行初始化逻辑。
pub async fn invalidate(key: &str) {
    let _ = {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.records.remove(key)
    };
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod cache_tests;
