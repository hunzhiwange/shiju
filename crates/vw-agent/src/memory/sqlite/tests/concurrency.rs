use super::*;
use crate::memory::MemoryCategory;

// ─────────────────────────────────────────────────────────────────────────────
// §4.1 并发写入冲突测试
// ─────────────────────────────────────────────────────────────────────────────

/// 测试并发写入不丢失数据
///
/// 验证点：
/// - 10 个并发写入操作全部成功
/// - 所有数据都被正确保存
/// - 最终计数正确
#[tokio::test]
async fn sqlite_concurrent_writes_no_data_loss() {
    let (_tmp, mem) = temp_sqlite();
    let mem = std::sync::Arc::new(mem);

    let mut handles = Vec::new();
    // 启动 10 个并发写入任务
    for i in 0..10 {
        let mem = std::sync::Arc::clone(&mem);
        handles.push(tokio::spawn(async move {
            mem.store(
                &format!("concurrent_key_{i}"),
                &format!("value_{i}"),
                MemoryCategory::Core,
                None,
            )
            .await
            .unwrap();
        }));
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }

    // 验证所有写入都成功
    let count = mem.count().await.unwrap();
    assert_eq!(count, 10, "all 10 concurrent writes must succeed without data loss");
}

/// 测试并发读写不会导致 panic
///
/// 验证点：
/// - 5 个并发读操作与 5 个并发写操作能同时执行
/// - 不会发生 panic 或死锁
/// - 最终数据计数正确
#[tokio::test]
async fn sqlite_concurrent_read_write_no_panic() {
    let (_tmp, mem) = temp_sqlite();
    let mem = std::sync::Arc::new(mem);

    // 预先存储一条共享数据
    mem.store("shared_key", "initial", MemoryCategory::Core, None).await.unwrap();

    let mut handles = Vec::new();

    // 5 个并发读取任务
    for _ in 0..5 {
        let mem = std::sync::Arc::clone(&mem);
        handles.push(tokio::spawn(async move {
            let _ = mem.get("shared_key").await.unwrap();
        }));
    }

    // 5 个并发写入任务
    for i in 0..5 {
        let mem = std::sync::Arc::clone(&mem);
        handles.push(tokio::spawn(async move {
            mem.store(&format!("key_{i}"), &format!("val_{i}"), MemoryCategory::Core, None)
                .await
                .unwrap();
        }));
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }

    // 应有 6 条记录（1 条预存 + 5 条新写入）
    assert_eq!(mem.count().await.unwrap(), 6);
}
