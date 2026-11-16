//! Gateway `AppState` 类型契约测试。
//!
//! 路由和处理器会在 Axum 状态提取器之间复制 `AppState`，因此该测试用编译期
//! 约束保证状态结构保持可克隆。

use super::*;

#[test]
fn app_state_is_clone() {
    // 用泛型边界表达契约，字段调整导致 Clone 丢失时会在编译阶段暴露。
    fn assert_clone<T: Clone>() {}
    assert_clone::<AppState>();
}
