//! 验证 LLM 请求日志的开关、脱敏、截断与写入错误处理。

use super::*;

use std::thread;

#[test]
fn global_logger_reuses_the_same_instance() {
    let first = &*LOGGER;
    let second = &*LOGGER;

    assert!(std::ptr::eq(first, second));
}

#[test]
// 全局日志器跨线程共享同一实例，避免并发请求各自创建写入器。
fn global_logger_reference_is_shared_across_threads() {
    let expected_address = std::ptr::from_ref(&*LOGGER) as usize;
    let handles =
        (0..4).map(|_| thread::spawn(|| std::ptr::from_ref(&*LOGGER) as usize)).collect::<Vec<_>>();

    for handle in handles {
        assert_eq!(handle.join().expect("logger access should not panic"), expected_address);
    }
}
