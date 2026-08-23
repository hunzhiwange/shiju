//! 汇总调度器面向控制器、本地执行器和 Matrix 通道的外部适配边界。

pub mod controller;
pub mod local;
pub mod matrix;

#[cfg(test)]
#[path = "controller_tests.rs"]
mod controller_tests;

#[cfg(test)]
#[path = "local_tests.rs"]
mod local_tests;
