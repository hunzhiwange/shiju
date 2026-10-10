//! 汇总调度器面向控制器和本地执行器的外部适配边界。
//! 未配置的外部能力由拒绝型实现返回明确错误，不伪造成功结果。

pub mod dispatch;
pub mod local;

#[cfg(test)]
#[path = "local_tests.rs"]
mod local_tests;
