//! 聚合调度执行引擎：正常派发由 `dispatch` 处理，异常收敛由 `recovery` 处理。
//! 两条路径共享存储状态机与租约语义，恢复流程不会绕过正常转换约束。

pub mod dispatch;
pub mod recovery;
