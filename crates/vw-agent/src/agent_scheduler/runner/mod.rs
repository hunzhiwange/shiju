//! 汇总本地与 Matrix 两类代理运行器及网关执行实现。
//!
//! Local and external task runners for Agent Scheduler.

pub(crate) mod gateway_executor;
pub mod local;
pub mod matrix;
