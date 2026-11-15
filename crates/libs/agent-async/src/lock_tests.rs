//! 验证同键异步读写锁的互斥等待与守卫释放唤醒语义。
//! 用例显式观察等待状态，确保读者和写者不会在持锁期间提前进入。

use super::lock::{read, write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn unique_key() -> String {
    static NEXT_KEY: AtomicUsize = AtomicUsize::new(0);
    format!("lock-test-{}-{}", std::process::id(), NEXT_KEY.fetch_add(1, Ordering::Relaxed))
}

#[tokio::test]
async fn write_waits_for_active_reader() {
    let key = unique_key();
    let reader = read(&key).await;
    let acquired = Arc::new(AtomicBool::new(false));
    let acquired_for_task = acquired.clone();
    let key_for_task = key.clone();

    let task = tokio::spawn(async move {
        let _writer = write(&key_for_task).await;
        acquired_for_task.store(true, Ordering::SeqCst);
    });
    tokio::task::yield_now().await;
    assert!(!acquired.load(Ordering::SeqCst));
    drop(reader);
    task.await.expect("writer task");
    assert!(acquired.load(Ordering::SeqCst));
}

#[tokio::test]
async fn dropping_writer_wakes_waiting_writer() {
    let key = unique_key();
    let first_writer = write(&key).await;
    let acquired = Arc::new(AtomicBool::new(false));
    let acquired_for_task = acquired.clone();
    let key_for_task = key.clone();

    let task = tokio::spawn(async move {
        let _second_writer = write(&key_for_task).await;
        acquired_for_task.store(true, Ordering::SeqCst);
    });
    tokio::task::yield_now().await;
    assert!(!acquired.load(Ordering::SeqCst));

    drop(first_writer);
    task.await.expect("second writer task");
    assert!(acquired.load(Ordering::SeqCst));
}

#[tokio::test]
async fn read_waits_for_active_writer() {
    let key = unique_key();
    let writer = write(&key).await;
    let acquired = Arc::new(AtomicBool::new(false));
    let acquired_for_task = acquired.clone();
    let key_for_task = key.clone();

    let task = tokio::spawn(async move {
        let _reader = read(&key_for_task).await;
        acquired_for_task.store(true, Ordering::SeqCst);
    });
    tokio::task::yield_now().await;
    assert!(!acquired.load(Ordering::SeqCst));

    drop(writer);
    task.await.expect("reader task");
    assert!(acquired.load(Ordering::SeqCst));
}
