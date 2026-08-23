//! 验证惰性值在重置前复用缓存且仅执行一次初始化逻辑。
//! 重置后的下一次读取必须重新求值，同时先前克隆值保持独立。

use super::lazy::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn get_caches_until_reset() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_lazy = calls.clone();
    let lazy = Lazy::new(move || calls_for_lazy.fetch_add(1, Ordering::SeqCst));

    let first = lazy.get();
    let second = lazy.get();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    lazy.reset();
    assert_eq!(*lazy.get(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
