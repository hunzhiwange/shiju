//! 验证延迟执行守卫在析构时调用闭包且可显式解除。
//! 两条路径共同约束回调最多执行一次，防止清理逻辑重复发生。

use super::defer::{Defer, defer};
use std::cell::Cell;

#[test]
fn runs_deferred_closure_on_drop() {
    let called = Cell::new(false);
    {
        let _guard = defer(|| called.set(true));
    }

    assert!(called.get());
}

#[test]
fn disarm_prevents_drop_callback() {
    let called = Cell::new(false);
    Defer::new(|| called.set(true)).disarm();

    assert!(!called.get());
}
