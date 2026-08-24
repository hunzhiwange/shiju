//! TUI 使用的 Gateway 运行时、会话存储与流事件适配边界。
//!
//! 子模块将网络 DTO 和阻塞包装限制在运行时层，不让渲染与纯状态模型依赖传输细节。

pub(crate) mod capabilities;
pub(crate) mod cron;
pub(crate) mod diagnostics;
pub(crate) mod file_search;
pub(crate) mod gateway;
pub(crate) mod hooks;
pub(crate) mod integrations;
pub(crate) mod orchestration;
pub(crate) mod plan_run;
pub(crate) mod interaction;
pub(crate) mod session_diff;
mod session_store;
pub(crate) mod stream_adapter;
pub(crate) mod workspace_files;
pub(crate) mod workflow_run;

#[cfg(test)]
mod diagnostics_tests;
#[cfg(test)]
mod file_search_tests;
#[cfg(test)]
mod hooks_tests;
#[cfg(test)]
mod workspace_files_tests;
