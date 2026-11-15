//! 提供异步队列和有限并发工作执行辅助。
//! 队列通过通知机制唤醒消费者，避免空队列时忙等。

use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

struct Inner<T> {
    queue: Mutex<VecDeque<T>>,
    notify: Notify,
}

/// AsyncQueue 表示该模块对外暴露的结构化状态。
#[derive(Clone)]
pub struct AsyncQueue<T> {
    inner: Arc<Inner<T>>,
}

impl<T> AsyncQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner { queue: Mutex::new(VecDeque::new()), notify: Notify::new() }),
        }
    }

    pub async fn push(&self, item: T) {
        let mut q = self.inner.queue.lock().await;
        q.push_back(item);
        drop(q);
        self.inner.notify.notify_one();
    }

    pub async fn next(&self) -> T {
        loop {
            let mut q = self.inner.queue.lock().await;
            if let Some(item) = q.pop_front() {
                return item;
            }
            self.inner.notify.notified().await;
        }
    }
}

/// 执行 work 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub async fn work<T, F, Fut>(concurrency: usize, items: Vec<T>, f: F)
where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    use futures_util::StreamExt;
    futures_util::stream::iter(items).for_each_concurrent(concurrency, f).await;
}
