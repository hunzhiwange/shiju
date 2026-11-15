//! 提供按字符串 key 隔离的异步读写锁。
//! 实现偏向等待中的写者，避免持续读流量导致写操作长期饥饿。

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

struct State {
    readers: usize,
    writer: bool,
    waiting_writers: usize,
}

struct Entry {
    state: Mutex<State>,
    readers_notify: Notify,
    writers_notify: Notify,
}

static LOCKS: LazyLock<Mutex<HashMap<String, Arc<Entry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get(key: &str) -> Arc<Entry> {
    let mut locks = LOCKS.lock().expect("locks mutex poisoned");
    if let Some(lock) = locks.get(key) {
        return lock.clone();
    }
    let entry = Arc::new(Entry {
        state: Mutex::new(State { readers: 0, writer: false, waiting_writers: 0 }),
        readers_notify: Notify::new(),
        writers_notify: Notify::new(),
    });
    locks.insert(key.to_string(), entry.clone());
    entry
}

/// ReadGuard 表示该模块对外暴露的结构化状态。
pub struct ReadGuard {
    entry: Arc<Entry>,
}

impl Drop for ReadGuard {
    fn drop(&mut self) {
        let mut st = self.entry.state.lock().expect("lock state poisoned");
        st.readers = st.readers.saturating_sub(1);
        if st.readers == 0 {
            self.entry.writers_notify.notify_one();
        }
    }
}

/// WriteGuard 表示该模块对外暴露的结构化状态。
pub struct WriteGuard {
    entry: Arc<Entry>,
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        let mut st = self.entry.state.lock().expect("lock state poisoned");
        st.writer = false;
        if st.waiting_writers > 0 {
            self.entry.writers_notify.notify_one();
        } else {
            self.entry.readers_notify.notify_waiters();
        }
    }
}

/// 执行 read 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub async fn read(key: &str) -> ReadGuard {
    let entry = get(key);
    loop {
        let wait = {
            let mut st = entry.state.lock().expect("lock state poisoned");
            if !st.writer && st.waiting_writers == 0 {
                st.readers += 1;
                None
            } else {
                Some(entry.readers_notify.notified())
            }
        };
        if let Some(wait) = wait {
            wait.await;
            continue;
        }
        return ReadGuard { entry: entry.clone() };
    }
}

/// 执行 write 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub async fn write(key: &str) -> WriteGuard {
    let entry = get(key);
    {
        let mut st = entry.state.lock().expect("lock state poisoned");
        st.waiting_writers += 1;
    }
    loop {
        let wait = {
            let mut st = entry.state.lock().expect("lock state poisoned");
            if !st.writer && st.readers == 0 {
                st.writer = true;
                st.waiting_writers = st.waiting_writers.saturating_sub(1);
                None
            } else {
                Some(entry.writers_notify.notified())
            }
        };
        if let Some(wait) = wait {
            wait.await;
            continue;
        }
        return WriteGuard { entry: entry.clone() };
    }
}
