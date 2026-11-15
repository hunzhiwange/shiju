//! TUI 使用的 Gateway 运行时、会话存储与流事件适配边界。
//!
//! 子模块将网络 DTO 和阻塞包装限制在运行时层，不让渲染与纯状态模型依赖传输细节。

pub mod capabilities;
pub mod cron;
pub mod diagnostics;
pub mod file_search;
pub mod hooks;
pub mod integrations;
pub mod orchestration;
pub mod workflow_run;
pub mod workspace_files;

#[cfg(test)]
mod diagnostics_tests;
#[cfg(test)]
mod file_search_tests;
#[cfg(test)]
mod hooks_tests;
#[cfg(test)]
mod workspace_files_tests;

#[cfg(test)]
mod gateway_composition_tests;
