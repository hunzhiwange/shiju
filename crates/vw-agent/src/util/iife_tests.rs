//! 验证立即执行辅助函数原样返回闭包结果。

use super::iife::iife;

#[test]
fn returns_closure_result() {
    assert_eq!(iife(|| 2 + 3), 5);
}
