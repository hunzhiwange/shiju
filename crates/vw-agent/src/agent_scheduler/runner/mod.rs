//! 汇总本地代理运行器及网关执行实现。
//! 运行器负责资源生命周期和结果落库，具体代理调用由执行器契约隔离。
//!
//! Local and external task runners for Agent Scheduler.

pub(crate) mod gateway_executor;
pub mod local;
