//! 验证立即执行辅助函数原样返回闭包结果。
//! 用例只固定求值语义，不引入对闭包捕获实现的额外约束。

use super::iife::iife;

#[test]
fn returns_closure_result() {
    assert_eq!(iife(|| 2 + 3), 5);
}
