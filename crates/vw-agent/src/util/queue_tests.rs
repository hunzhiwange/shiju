//! 验证异步队列保持先进先出顺序并完整处理工作项。

use super::queue::{AsyncQueue, work};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn queue_returns_pushed_items_fifo() {
    let queue = AsyncQueue::new();
    queue.push(1).await;
    queue.push(2).await;

    assert_eq!(queue.next().await, 1);
    assert_eq!(queue.next().await, 2);
}

#[tokio::test]
async fn work_processes_all_items() {
    let count = Arc::new(AtomicUsize::new(0));
    let count_for_work = count.clone();
    work(2, vec![1, 2, 3], move |_| {
        let count = count_for_work.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;

    assert_eq!(count.load(Ordering::SeqCst), 3);
}
